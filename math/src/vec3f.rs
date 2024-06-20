#[derive(Debug, PartialEq)]
pub struct Vector3f {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vector3f {
    pub fn new(x: f32, y: f32, z: f32) -> Vector3f {
        Vector3f {
            x,
            y,
            z,
        }
    }

    pub fn zero() -> Vector3f {
        Vector3f {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }

    pub fn set(&mut self, x: f32, y: f32, z: f32) -> &Self {
        self.x = x;
        self.y = y;
        self.z = z;
        self
    }

    pub fn cross(&self, b: &Vector3f) -> Vector3f {
        Vector3f {
            x: self.y * b.z - self.z * b.y,
            y: self.z * b.x - self.x * b.z,
            z: self.x * b.y - self.y * b.x,
        }
    }

    pub fn square_length(&self) -> f32 {
        self.x * self.x + self.y * self.y + self.z * self.z
    }

    pub fn length(&self) -> f32 {
        self.square_length().sqrt()
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    // #[test]
    // fn creates() {
    //     assert_eq!(Vector3f::new(), Vector3f { x: 0.0, y: 0.0, z: 0.0 });
    // }

    #[test]
    fn sets() {
        assert_eq!(*Vector3f::zero().set(1., 2., 3.), Vector3f::new(1., 2., 3.));
    }

    #[test]
    fn cross() {
        let a = Vector3f::new(0., 1., 0.);
        let v = Vector3f::new(1., 0., 0.).cross(&a);
        assert_eq!(v, Vector3f { x: 0., y: 0., z: 1. });

        let a = Vector3f::new(0., -1., 0.);
        let v = Vector3f::new(-1., 0., 0.).cross(&a);
        assert_eq!(v, Vector3f::new(0., 0., 1.));

        let a = Vector3f::new(0., 1., 0.);
        let v = Vector3f::new(-1., 0., 0.).cross(&a);
        assert_eq!(v, Vector3f::new(0., 0., -1.));

        let a = Vector3f::new(0., 1., 0.);
        let v = Vector3f::new(1., 0., 0.).cross(&a);
        assert_eq!(v, Vector3f::new(0., 0., 1.));
    }

    #[test]
    fn square_length() {
        assert_eq!(Vector3f::new(1., 1., 1.).square_length(), 3.0)
    }

    #[test]
    fn length() {
        assert_eq!(Vector3f::new(1., 1., 1.).length(), 1.7320508)
    }
}
