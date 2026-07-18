use glam::Vec3;
use indexmap::IndexMap;
use rg_common::world::HyperCube;
use rustc_hash::FxBuildHasher;

type HyperCubeMap = IndexMap<(i32, i32, i32), HyperCube, FxBuildHasher>;

enum LoadState {
    Loaded(HyperCube),
    Loading
}

pub struct World {
    visible_part: HyperCubeMap,
}

impl World {
    pub fn new() -> Self {
        Self {
            visible_part: HyperCubeMap::with_capacity_and_hasher(600, FxBuildHasher::default()),
        }
    }

    pub fn prefetch(x: i32, y: i32, z: i32) {

    }
}
