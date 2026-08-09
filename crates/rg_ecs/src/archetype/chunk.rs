use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU32, Ordering},
        RwLock,
    },
};

use crate::{
    archetype::archetype::COLUMN_ENTITY_ID, component::{cast, cast_mut, ComponentId, ComponentStorage}, entity::EntityId
};

type ColumnMap = HashMap<ComponentId, RwLock<Box<dyn ComponentStorage>>>;

pub struct Chunk {
    pub(crate) columns: ColumnMap,
    available_rows: AtomicU32,
}

impl Chunk {
    /// Wraps supplied columns in this chunk.
    /// # Arguments:
    /// * `capacity` - the capacity of each supplied column
    pub(crate) fn new(columns: ColumnMap, capacity: usize) -> Self {
        Chunk {
            columns,
            available_rows: AtomicU32::new(capacity as u32),
        }
    }

    pub(super) fn available(&self) -> u32 {
        self.available_rows.load(Ordering::Acquire)
    }

    /// Adds new row for passed entity to this storage and returns local index
    /// # Arguments:
    /// * `ent_id` - entity id
    pub(super) fn add(&self, ent_id: EntityId) -> usize {
        assert!(self.available() > 0);
        let mut index = 0;
        for (_, column) in self.columns.iter() {
            index = column.write().unwrap().add();
        }
        if let Some(column) = self.columns.get(&COLUMN_ENTITY_ID) {
            // Cell already added in above loop, now set value
            cast_mut::<EntityId>(column.write().unwrap().as_mut())[index] = ent_id;
        }
        self.available_rows.fetch_sub(1, Ordering::Relaxed);
        index
    }

    pub(super) fn get_entity_id(&self, index: usize) -> Option<EntityId> {
        let column = self.columns.get(&COLUMN_ENTITY_ID)?;
        cast::<EntityId>(column.read().unwrap().as_ref())
            .get(index)
            .copied()
    }

    /// Removes row from this chunk
    /// # Arguments:
    /// * `index` - row index
    pub(super) fn remove(&self, index: usize) -> Option<EntityId> {
        for column in self.columns.values() {
            column.write().unwrap().remove(index);
        }
        self.available_rows.fetch_add(1, Ordering::Relaxed);
        self.get_entity_id(index)
    }

    /// Moves row from this chunk to another storage. Returns id of the entity which has taken place of the moved one.
    pub(super) fn move_to<T>(&self, index: usize, dest: &Chunk, value: T) -> (usize, Option<EntityId>)
    where
        T: Default + 'static,
    {
        for (comp_id, column) in self.columns.iter() {
            let lock = dest.get_column(*comp_id).unwrap();
            let mut guard = lock.write().unwrap();
            column.write().unwrap().move_to(index, guard.as_mut());
        }
        let lock = dest.get_column_for_type::<T>().unwrap();
        let idx = {
            let mut guard = lock.write().unwrap();
            let typed_col = cast_mut::<T>(guard.as_mut());
            let idx = typed_col.len();
            typed_col.push(value);
            idx
        };
        dest.available_rows.fetch_sub(1, Ordering::Relaxed);
        self.available_rows.fetch_add(1, Ordering::Relaxed);
        (idx, self.get_entity_id(index))
    }

    #[inline(always)]
    pub(crate) fn get_column(
        &self,
        comp_id: ComponentId,
    ) -> Option<&RwLock<Box<dyn ComponentStorage>>> {
        self.columns.get(&comp_id)
    }

    #[inline(always)]
    pub(super) fn get_column_for_type<T>(&self) -> Option<&RwLock<Box<dyn ComponentStorage>>>
    where
        T: Default + 'static,
    {
        self.columns.get(&ComponentId::new::<T>())
    }

    pub(super) fn row_count(&self) -> usize {
        for (_, col) in self.columns.iter() {
            return col.read().unwrap().row_count();
        }
        0
    }
}
