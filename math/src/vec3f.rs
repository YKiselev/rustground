use std::ops::{Add, Div, Mul, Sub};

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Vector3f {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vector3f {
    pub fn new(x: f32, y: f32, z: f32) -> Vector3f {
        Vector3f { x, y, z }
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

    pub fn cross(&self, b: Vector3f) -> Self {
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

    pub fn normalize(&self) -> Self {
        let len = self.length();
        if len > 0.0 {
            let ool = 1.0 / len;
            return Vector3f::new(self.x * ool, self.y * ool, self.z * ool);
        }
        self.clone()
    }

    pub fn dot(&self, b: Vector3f) -> f32 {
        self.x * b.x + self.y * b.y + self.z * b.z
    }
}

impl Add for Vector3f {
    type Output = Vector3f;

    fn add(self, rhs: Self) -> Self::Output {
        Vector3f {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            z: self.z + rhs.z,
        }
    }
}

impl Sub for Vector3f {
    type Output = Vector3f;

    fn sub(self, rhs: Self) -> Self::Output {
        Vector3f {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
            z: self.z - rhs.z,
        }
    }
}

impl Mul for Vector3f {
    type Output = Vector3f;

    fn mul(self, rhs: Self) -> Self::Output {
        Vector3f {
            x: self.x * rhs.x,
            y: self.y * rhs.y,
            z: self.z * rhs.z,
        }
    }
}

impl Mul<f32> for Vector3f {
    type Output = Vector3f;

    fn mul(self, rhs: f32) -> Self::Output {
        Vector3f {
            x: self.x * rhs,
            y: self.y * rhs,
            z: self.z * rhs,
        }
    }
}

impl Div for Vector3f {
    type Output = Vector3f;

    fn div(self, rhs: Self) -> Self::Output {
        Vector3f {
            x: self.x / rhs.x,
            y: self.y / rhs.y,
            z: self.z / rhs.z,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sets() {
        assert_eq!(*Vector3f::zero().set(1., 2., 3.), Vector3f::new(1., 2., 3.));
    }

    #[test]
    fn cross() {
        let a = Vector3f::new(0., 1., 0.);
        let v = Vector3f::new(1., 0., 0.).cross(a);
        assert_eq!(
            v,
            Vector3f {
                x: 0.,
                y: 0.,
                z: 1.
            }
        );

        let a = Vector3f::new(0., -1., 0.);
        let v = Vector3f::new(-1., 0., 0.).cross(a);
        assert_eq!(v, Vector3f::new(0., 0., 1.));

        let a = Vector3f::new(0., 1., 0.);
        let v = Vector3f::new(-1., 0., 0.).cross(a);
        assert_eq!(v, Vector3f::new(0., 0., -1.));

        let a = Vector3f::new(0., 1., 0.);
        let v = Vector3f::new(1., 0., 0.).cross(a);
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

    #[test]
    fn normalize() {
        assert_eq!(
            Vector3f::new(1., 1., 1.).normalize(),
            Vector3f::new(0.57735026, 0.57735026, 0.57735026)
        );
    }

    #[test]
    fn add() {
        let a = Vector3f::new(1., 2., 3.);
        let b = Vector3f::new(4., 5., 6.);
        assert_eq!(a + b, Vector3f::new(5., 7., 9.));
    }

    #[test]
    fn multiply() {
        let a = Vector3f::new(1., 2., 3.);
        let b = Vector3f::new(4., 5., 6.);
        assert_eq!(a * b, Vector3f::new(4., 10., 18.));
    }

    #[test]
    fn multiply_by_scalar() {
        let a = Vector3f::new(1., 2., 3.);
        assert_eq!(a * 3., Vector3f::new(3., 6., 9.));
    }

    #[test]
    fn sub() {
        let a = Vector3f::new(4., 7., 11.);
        let b = Vector3f::new(1., 2., 3.);
        assert_eq!(a - b, Vector3f::new(3., 5., 8.));
    }

    #[test]
    fn div() {
        let a = Vector3f::new(2., 6., 9.);
        let b = Vector3f::new(1., 2., 3.);
        assert_eq!(a / b, Vector3f::new(2., 3., 3.));
    }

    #[test]
    fn dot() {
        assert_eq!(
            Vector3f::new(1., 1., 1.).dot(Vector3f::new(2., 3., 4.)),
            9.0
        )
    }
}
