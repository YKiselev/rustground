use bitflags::bitflags;
use glam::Vec3;
use rand::Rng;

bitflags! {
    ///
    /// Bit flags for cube faces
    ///
    pub struct CubeNormals: u32 {
        const FRONT = 1 << 0;
        const BACK = 1 << 1;
        const LEFT = 1 << 2;
        const RIGHT = 1 << 3;
        const TOP = 1 << 4;
        const BOTTOM = 1 << 5;
    }
}

#[rustfmt::skip]
static CUBE_NORMALS: [Vec3; 6] = [
    //  Front face (normal Z+)
    Vec3::new(0.0, 0.0, 1.0),
    //  Back face (normal Z-)
    Vec3::new(0.0, 0.0, -1.0),
    //  Left face (normal X-)
    Vec3::new(-1.0, 0.0, 0.0),
    //  Right face (normal X+)
    Vec3::new(1.0, 0.0, 0.0),
    //  Top face (normal Y+)
    Vec3::new(0.0, 1.0, 0.0),
    //  Bottom face (normal Y-)
    Vec3::new(0.0, -1.0, 0.0),
];

const CUBE_HALF_SIZE: f32 = 0.5;
const HYPER_CUBE_HALF_SIZE: f32 = 16.0 * (2.0 * CUBE_HALF_SIZE) / 2.0;

pub struct HyperCube {
    pub origin: Vec3,
    pub materials: Vec<u8>,
    pub pvs: fixedbitset::FixedBitSet,
}

impl HyperCube {
    pub fn from_materials(x: f32, y: f32, z: f32, materials: Vec<u8>) -> Self {
        assert_eq!(4096, materials.len());

        Self {
            origin: Vec3 { x, y, z },
            materials,
            pvs: fixedbitset::FixedBitSet::with_capacity(4096),
        }
    }

    pub fn new(x: f32, y: f32, z: f32) -> Self {
        let materials = generate_materials();
        Self::from_materials(x, y, z, materials)
    }

    pub fn solid() -> Self {
        let materials: [u8; 4096] = std::array::from_fn(|_| 1u8);
        let mut result = Self::from_materials(0.0, 0.0, 0.0, Vec::from(materials));
        result.init_pvs();
        result
    }

    pub fn normal(face: usize) -> Vec3 {
        CUBE_NORMALS[face]
    }

    ///
    /// Classifies any point agains this hyper cube.
    /// Returns bitmask of potentially visible faces.
    ///
    pub fn classify(&self, p: Vec3) -> u32 {
        let center = self.origin + HYPER_CUBE_HALF_SIZE;
        let mut result = 0;

        for (i, normal) in CUBE_NORMALS.iter().enumerate() {
            // Distance is positive if point is in front of plane, 0 if on plane and negative if behind plane
            let distance = normal.dot(p - center) - HYPER_CUBE_HALF_SIZE;
            if distance > 0.0 {
                // this face is visible
                result |= 1 << i;
            }
        }

        result
    }

    pub fn is_transparent(&self, i: i32, j: i32, k: i32) -> bool {
        if i < 0 || i > 15 || j < 0 || j > 15 || k < 0 || k > 15 {
            return true;
        }
        let index = i + (j << 4) + (k << 8);
        if index < 0 || index > 4095 {
            println!("Oops: {:?}={}", (i,j,k), index);
        }
        self.materials[index as usize] == 0
    }

    ///
    /// Cube is blocked if all its neighbors are opaque
    ///
    fn is_blocked(&self, i: i32, j: i32, k: i32) -> bool {
        !self.is_transparent(i - 1, j, k)
            && !self.is_transparent(i + 1, j, k)
            && !self.is_transparent(i, j - 1, k)
            && !self.is_transparent(i, j + 1, k)
            && !self.is_transparent(i, j, k - 1)
            && !self.is_transparent(i, j, k + 1)
    }

    pub fn init_pvs(&mut self) {
        self.pvs.clear();

        for index in 0..4096 {
            let (i, j, k) = from_index(index);
            if !self.is_blocked(i, j, k) {
                self.pvs.set(index, true);
            }
        }
    }
}

fn from_index(index: usize) -> (i32, i32, i32) {
    let i = index & 15;
    let j = (index >> 4) & 15;
    let k = (index >> 8) & 15;
    (i as i32, j as i32, k as i32)
}

fn from_xyz(x: i32, y: i32, z: i32) -> i32 {
    x + ((y) << 4) + ((z) << 8)
}

fn generate_materials() -> Vec<u8> {
    let mut materials = Vec::with_capacity(4096);
    let mut rng = rand::thread_rng();
    let min_probability = 0.3;
    let max_probability = 0.95;
    let mut probability = max_probability;
    let probability_step = (max_probability - min_probability) / 4096.0;
    for _ in 0..4096 {
        let mat = if rng.gen_bool(probability) { 1 } else { 0 };
        materials.push(mat);
        probability -= probability_step;
    }
    materials
}

///
/// Tests
///
#[cfg(test)]
mod tests {
    use glam::Vec3;

    use super::*;
    use crate::world::{HyperCube, hyper_cube::CUBE_NORMALS};

    #[test]
    fn test() {
        let mut idx = 0;
        for k in 0..16 {
            for j in 0..16 {
                for i in 0..16 {
                    let (x, y, z) = from_index(idx as usize);
                    let idx2 = from_xyz(x, y, z);
                    assert_eq!((i, j, k), (x, y, z));
                    assert_eq!(idx, idx2);
                    idx += 1;
                }
            }
        }
    }

    #[test]
    fn should_init_pvs() {
        let hc = HyperCube::solid();

        println!(
            "pvs={}, total of {} visible cubes",
            hc.pvs,
            hc.pvs.count_ones(..)
        );
    }

    #[test]
    fn should_classify() {
        let half_size = 0.5;
        let center = Vec3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        };
        let points = CUBE_NORMALS.map(|n| (n * 2.0) + center);
        println!(
            "For cube of half-size {} with center in {}:",
            half_size, center
        );
        for p in points {
            for normal in CUBE_NORMALS {
                if p.x != 0.0 && normal.x != 0.0
                    || p.y != 0.0 && normal.y != 0.0
                    || p.z != 0.0 && normal.z != 0.0
                {
                    let r = normal.dot(p - center) - half_size;
                    println!("point {} is classified as {} against {}", p, r, normal);
                }
            }
        }
    }
}
