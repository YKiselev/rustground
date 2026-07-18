
use bitflags::bitflags;

#[derive(Debug, Default, PartialEq, Hash, Clone, Copy)]
pub struct Material {
    pub flags: MaterialFlag,
}

impl Material {}

bitflags! {
    #[derive(Debug, Default, PartialEq, Hash, Clone, Copy)]
    pub struct MaterialFlag: u16 {
        const NONE = 0;
        const OPAQUE = 1 << 0; // whether object of this material can't be seen trough?
        const SOLID = 1 << 1; // whether object of this material is penetrable?
    }
}

#[derive(Debug)]
pub struct Materials {
    data: Vec<Material>, // material at index 0 is always transparent and non-solid
}

impl Materials {
    pub fn new() -> Self {
        Self { data: Vec::new() }
    }

    pub fn add(&mut self, material: Material) {
        self.data.push(material);
    }

    pub fn get(&self, material: u8) -> Option<&Material> {
        self.data.get(material as usize)
    }
}

impl Default for Materials {
    fn default() -> Self {
        Self {
            data: vec![
                Material::default(),
                Material {
                    flags: MaterialFlag::OPAQUE | MaterialFlag::SOLID,
                },
            ],
        }
    }
}
