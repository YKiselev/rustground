use std::ptr::slice_from_raw_parts;
use std::{marker::PhantomData, ptr::slice_from_raw_parts_mut};

use bytes::{Buf, BytesMut};

pub const PAGE_SIZE: usize = 4096;

#[derive(PartialEq, Eq, Clone, Copy)]
#[repr(u8)]
enum NodeType {
    Inner = 1,
    Leaf = 2,
}

impl TryFrom<u8> for NodeType {
    type Error = String;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(NodeType::Inner),
            2 => Ok(NodeType::Leaf),
            _ => Err(format!("Unknown variant: {}", value)),
        }
    }
}

const fn align_offset<T>(mut offset: usize) -> usize {
    let alignment = std::mem::align_of::<T>();
    while offset & (alignment - 1) != 0 {
        offset += 1;
    }
    offset
}

fn slice_of<T>(buf: &[u8]) -> &[T] {
    assert_eq!(0, buf.len() % std::mem::size_of::<T>());

    let ptr = buf.as_ptr();
    let slice_ptr = slice_from_raw_parts(ptr as *const T, buf.len());
    unsafe { &*slice_ptr }
}

fn mut_slice_of<T>(buf: &mut [u8]) -> &mut [T] {
    assert_eq!(0, buf.len() % std::mem::size_of::<T>());

    let ptr = buf.as_mut_ptr();
    let slice_ptr = slice_from_raw_parts_mut(ptr as *mut T, buf.len());
    unsafe { &mut *slice_ptr }
}

type NodeId = usize;
type DataRef = usize;

const MAGIC_OFFSET: usize = 0; // 3 bytes magic
const TYPE_OFFSET: usize = 3; // 1 byte node type (u8)
const KEY_COUNT_OFFSET: usize = 4; // 2 bytes key count (u16)

struct Split<K, V> {
    key: K,
    node: BTreeNode<K, V>,
}

struct BTreeNode<K, V> {
    buffer: BytesMut,
    _phantom: PhantomData<(K, V)>,
}

impl<K, V> BTreeNode<K, V>
where
    K: Sized + Ord,
    V: Sized,
{
    const KEY_SIZE: usize = std::mem::size_of::<K>();
    const DATA_REF_SIZE: usize = std::mem::size_of::<DataRef>();
    const NODE_ID_SIZE: usize = std::mem::size_of::<NodeId>();
    const KEYS_OFFSET: usize = align_offset::<K>(KEY_COUNT_OFFSET + 2); // keys array

    const MAX_INNER_PAIRS: usize = const {
        let common_size = Self::KEY_SIZE + Self::NODE_ID_SIZE;
        let capacity = PAGE_SIZE - Self::KEYS_OFFSET;
        capacity / common_size
    };
    const MAX_INNER_KEYS: usize = const { (Self::MAX_INNER_PAIRS / 2).saturating_sub(1) };
    const MAX_INNER_NODE_IDS: usize = const { Self::MAX_INNER_PAIRS / 2 };
    const NODE_IDS_OFFSET: usize =
        const { align_offset::<NodeId>(Self::KEYS_OFFSET + Self::MAX_INNER_KEYS * Self::KEY_SIZE) };

    const MAX_LEAF_PAIRS: usize = const {
        let common_size = Self::KEY_SIZE + Self::DATA_REF_SIZE;
        let capacity = PAGE_SIZE - Self::KEYS_OFFSET;
        capacity / common_size
    };
    const MAX_LEAF_KEYS: usize = const { (Self::MAX_LEAF_PAIRS / 2).saturating_sub(1) };
    const MAX_LEAF_DATA_REFS: usize = const { Self::MAX_LEAF_PAIRS / 2 };
    const DATA_REFS_OFFSET: usize =
        const { align_offset::<DataRef>(Self::KEYS_OFFSET + Self::MAX_LEAF_KEYS * Self::KEY_SIZE) };

    fn get_type(&self) -> Result<NodeType, String> {
        self.buffer
            .get(TYPE_OFFSET)
            .ok_or("Invaild index".to_string())
            .and_then(|v| NodeType::try_from(*v))
    }

    fn get_key_count(&self) -> usize {
        if self.buffer.len() > KEY_COUNT_OFFSET + 2 {
            let bytes: Result<[u8; 2], _> =
                self.buffer[KEY_COUNT_OFFSET..KEY_COUNT_OFFSET + 2].try_into();
            if let Ok(bytes) = bytes {
                return u16::from_be_bytes(bytes) as usize;
            }
        }
        0
    }

    fn get_keys(&self) -> Result<&[K], String> {
        let key_count = self.get_key_count();
        let size_in_bytes = key_count * Self::KEY_SIZE;
        let keys = &self.buffer[Self::KEYS_OFFSET..Self::KEYS_OFFSET + size_in_bytes];
        Ok(slice_of(keys))
    }

    fn get_keys_mut(&mut self) -> Result<&mut [K], String> {
        let node_type = self.get_type()?;
        let size_in_bytes = if node_type == NodeType::Inner {
            Self::MAX_INNER_KEYS
        } else {
            Self::MAX_LEAF_KEYS
        };
        let keys = &mut self.buffer[Self::KEYS_OFFSET..Self::KEYS_OFFSET + size_in_bytes];
        Ok(mut_slice_of(keys))
    }

    fn get_node_ids(&self) -> Result<&[NodeId], String> {
        let key_count = self.get_key_count();
        let id_count = if key_count == 0 { 0 } else { key_count + 1 };
        let size_in_bytes = id_count * Self::NODE_ID_SIZE;
        let node_ids = &self.buffer[Self::NODE_IDS_OFFSET..Self::NODE_IDS_OFFSET + size_in_bytes];
        Ok(slice_of(node_ids))
    }

    fn get_data_refs(&self) -> Result<&[DataRef], String> {
        let key_count = self.get_key_count();
        let ref_count = if key_count == 0 { 0 } else { key_count + 1 };
        let size_in_bytes = ref_count * Self::DATA_REF_SIZE;
        let data_refs =
            &self.buffer[Self::DATA_REFS_OFFSET..Self::DATA_REFS_OFFSET + size_in_bytes];
        Ok(slice_of(data_refs))
    }

    fn search(&self, key: &K, owner: &BTree<K, V>) -> Result<Option<DataRef>, String> {
        let data = self.get_keys()?;
        let idx = data.binary_search(key);
        if let Ok(idx) = idx {
            let node_type = self.get_type()?;
            match node_type {
                NodeType::Inner => {
                    let node_ids = self.get_node_ids()?;
                    let node = owner.get_node(node_ids[idx])?;

                    return node.search(key, owner);
                }
                NodeType::Leaf => {
                    let refs = self.get_data_refs()?;
                    return Ok(Some(refs[idx]));
                }
            }
        } else {
            Ok(None)
        }
    }

    fn insert(
        &mut self,
        key: &K,
        data_ref: DataRef,
        owner: &mut BTree<K, V>,
    ) -> Result<Option<Split<K, V>>, String> {
        let key_count = self.get_key_count();
        let node_type = self.get_type()?;
        let max_keys = if node_type == NodeType::Inner {
            Self::MAX_INNER_KEYS
        } else {
            Self::MAX_LEAF_KEYS
        };
        if key_count == max_keys {
            let split = self.split(owner)?;

            
            
        } else {
        }
        todo!()
    }

    fn split(&mut self, owner: &mut BTree<K, V>,) -> Result<Split<K, V>, String> {
        let node_type = self.get_type()?;
        let new_node_id = owner.allocate_node(node_type)?;
        let new_node = owner.get_node_mut(new_node_id)?;

        match node_type {
            NodeType::Inner => todo!(),
            NodeType::Leaf => todo!(),
        }
    }
}

struct BTree<K, V> {
    root: BTreeNode<K, V>,
}

impl<K, V> BTree<K, V> {
    fn get_node(&self, node_id: NodeId) -> Result<&BTreeNode<K, V>, String> {
        todo!()
    }

    fn get_node_mut(&mut self, node_id: NodeId) -> Result<&mut BTreeNode<K, V>, String> {
        todo!()
    }

    fn allocate_node(&mut self, node_type: NodeType) -> Result<NodeId, String> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use crate::align_offset;

    #[test]
    fn should_align() {
        assert_eq!(1, align_offset::<u8>(1));
        assert_eq!(2, align_offset::<u16>(1));
        assert_eq!(4, align_offset::<u32>(1));
        assert_eq!(8, align_offset::<u64>(2));
    }
}
