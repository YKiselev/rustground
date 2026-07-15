use rand::Rng;

const HYPER_CUBE_INDICES: [[u8; 3]; 4096] = new_index_array();

pub struct HyperCube {
    pub origin: [f32; 3],
    pub material: Vec<u8>,
}

impl HyperCube {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        let mut material = Vec::with_capacity(4096);
        let mut rng = rand::thread_rng();
        let min_probability = 0.3;
        let max_probability = 0.95;
        let mut probability = max_probability;
        let probability_step = (max_probability - min_probability) / 4096.0;
        for _ in 0..4096 {
            let mat = if rng.gen_bool(probability) { 1 } else { 0 };
            material.push(mat);
            probability -= probability_step;
        }
        Self {
            origin: [x, y, z],
            material,
        }
    }

    pub fn indices() -> &'static [[u8; 3]; 4096] {
        &HYPER_CUBE_INDICES
    }
}

///
/// Hyper cube indices generator
///
const fn new_index_array() -> [[u8; 3]; 4096] {
    let mut indices = [[0u8; 3]; 4096];
    let mut idx = 0;
    let mut i = 0;
    let mut j = 0;
    let mut k = 0;
    while k < 16 {
        while j < 16 {
            while i < 16 {
                indices[idx] = [i, j, k];
                idx += 1;
                i += 1;
            }
            i = 0;
            j += 1;
        }
        j = 0;
        k += 1;
    }
    indices
}
