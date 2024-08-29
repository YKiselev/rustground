use std::ops::{Add, Mul};

use crate::vec3f::Vector3f;
use crate::vec4f::Vector4f;

/// Column-oriented 4x4 matrix.
/// ```text
/// A B C D
/// E F G H
/// I J K L
/// M N O P
///```
/// or as indices:
///```text
/// 0 4  8 12
/// 1 5  9 13
/// 2 6 10 14
/// 3 7 11 15
///```
/// So A have index 0, E - 1, I - 2, M - 3, etc.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Matrix {
    pub m: [f32; 16],
}

impl Matrix {
    pub fn new() -> Self {
        Matrix { m: [0.; 16] }
    }

    pub fn identity() -> Self {
        Matrix {
            m: [
                1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.,
            ],
        }
    }

    /// Returns an orthographic projection matrix.
    /// # Arguments
    /// * `left`   the left screen coordinate (usually 0)
    /// * `right`  the right screen coordinate (usually width)
    /// * `top`    the top screen coordinate (usually height)
    /// * `bottom` the bottom screen coordinate (usually 0)
    /// * `near`   the near z value (for example -1)
    /// * `far`    the far z coordinate (for example 1)
    /// * `m`      the buffer to store resulting matrix in.
    pub fn orthographic(left: f32, right: f32, top: f32, bottom: f32, near: f32, far: f32) -> Self {
        Matrix {
            m: [
                2. / (right - left),
                0.,
                0.,
                0.,
                0.,
                2. / (top - bottom),
                0.,
                0.,
                0.,
                0.,
                -2. / (far - near),
                0.,
                -(right + left) / (right - left),
                -(top + bottom) / (top - bottom),
                -(far + near) / (far - near),
                1.,
            ],
        }
    }

    /// Returns perspective projection matrix.
    /// # Arguments
    /// * `left`   the left screen coordinate (usually 0)
    /// * `right`  the right screen coordinate (usually width)
    /// * `top`    the top screen coordinate (usually height)
    /// * `bottom` the bottom screen coordinate (usually 0)
    /// * `near`   the near z value (for example -1)
    /// * `far`    the far z coordinate (for example 1)
    /// * `m`      the buffer to store resulting matrix in.
    pub fn perspective(left: f32, right: f32, top: f32, bottom: f32, near: f32, far: f32) -> Self {
        Matrix {
            m: [
                2.0 * near / (right - left),
                0.,
                0.,
                0.,
                0.,
                (2.0 * near / (top - bottom)),
                0.,
                0.,
                (right + left) / (right - left),
                (top + bottom) / (top - bottom),
                -(far + near) / (far - near),
                -1.,
                0.,
                0.,
                -2.0 * far * near / (far - near),
                0.,
            ],
        }
    }

    /// Returns perspective projection matrix.
    /// # Arguments
    /// * `fow`   the horizontal field of view (in radians)
    /// * `ratio` the aspect ratio between width and height of screen
    /// * `near`  the near z coordinate (should be > 0)
    /// * `far`   the far z coordinate
    /// * `m`     the buffer to store resulting matrix in.
    pub fn perspective_fow(fow: f32, ratio: f32, near: f32, far: f32) -> Self {
        let w = near * (0.5 * fow).tan();
        let h = w / ratio;
        Matrix::perspective(-w, w, h, -h, near, far)
    }

    /// Multiplies this matrix by translation matrix derived from `(dx,dy,dz)`.
    /// # Arguments
    /// * `self`   this matrix
    /// * `dx`     x translation
    /// * `dy`     y translation
    /// * `dz`     z translation
    pub fn translate(&self, dx: f32, dy: f32, dz: f32) -> Self {
        let a = &self.m;
        let mut m = a.clone();

        let m12 = a[0] * dx + a[4] * dy + a[8] * dz + a[12];
        let m13 = a[1] * dx + a[5] * dy + a[9] * dz + a[13];
        let m14 = a[2] * dx + a[6] * dy + a[10] * dz + a[14];
        let m15 = a[3] * dx + a[7] * dy + a[11] * dz + a[15];

        m[12] = m12;
        m[13] = m13;
        m[14] = m14;
        m[15] = m15;
        Matrix { m }
    }

    /// Combines scaling `(sx,sy,sz)` with this matrix.
    /// # Arguments
    /// * `self`   this matrix
    /// * `sx`     x scaling factor
    /// * `sy`     y  scaling factor
    /// * `sz`     z  scaling factor
    pub fn scale(&self, sx: f32, sy: f32, sz: f32) -> Self {
        let a = &self.m;
        // r0
        let m0 = a[0] * sx;
        let m4 = a[4] * sy;
        let m8 = a[8] * sz;
        let m12 = a[12];
        // r1
        let m1 = a[1] * sx;
        let m5 = a[5] * sy;
        let m9 = a[9] * sz;
        let m13 = a[13];
        // r2
        let m2 = a[2] * sx;
        let m6 = a[6] * sy;
        let m10 = a[10] * sz;
        let m14 = a[14];
        // r3
        let m3 = a[3] * sx;
        let m7 = a[7] * sy;
        let m11 = a[11] * sz;
        let m15 = a[15];

        Matrix {
            m: [
                m0, m1, m2, m3, m4, m5, m6, m7, m8, m9, m10, m11, m12, m13, m14, m15,
            ],
        }
    }

    /// Returns transposed matrix.
    pub fn transpose(&self) -> Self {
        let a = &self.m;
        let m0 = a[0];
        let m1 = a[1];
        let m2 = a[2];
        let m3 = a[3];

        let m4 = a[4];
        let m5 = a[5];
        let m6 = a[6];
        let m7 = a[7];

        let m8 = a[8];
        let m9 = a[9];
        let m10 = a[10];
        let m11 = a[11];

        let m12 = a[12];
        let m13 = a[13];
        let m14 = a[14];
        let m15 = a[15];
        Matrix {
            m: [
                m0, m4, m8, m12, m1, m5, m9, m13, m2, m6, m10, m14, m3, m7, m11, m15,
            ],
        }
    }

    /// Returns rotation matrix derived from Euler's angles `ax, ay, az`.
    /// # Arguments
    /// * `ax` the x-axis rotation angle (counter-clockwise, in radians)
    /// * `ay` the y-axis rotation angle (counter-clockwise, in radians)
    /// * `az` the z-axis rotation angle (counter-clockwise, in radians)
    pub fn rotation(ax: f32, ay: f32, az: f32) -> Self {
        let a = ax.cos();
        let b = ax.sin();
        let c = ay.cos();
        let d = ay.sin();
        let e = az.cos();
        let f = az.sin();
        Matrix {
            m: [
                c * e,
                b * d * e + a * f,
                -a * d * e + b * f,
                0.,
                -c * f,
                -b * d * f + a * e,
                a * d * f + b * e,
                0.,
                d,
                -b * c,
                a * c,
                0.,
                0.,
                0.,
                0.,
                1.,
            ],
        }
    }

    /// For 2x2 matrix M determinant is A*D - B*C
    ///```text
    ///     | A B |
    /// M = |     |
    ///     | C D |
    ///```
    /// So for our 4x4 matrix determinant `Det = A * Asubd - B * Bsubd + C * Csubd - D * Dsubd` where
    ///```text
    ///     0 4  8 12
    ///     1 5  9 13
    /// M   2 6 10 14
    ///     3 7 11 15
    ///
    ///       5  9 13
    /// Asub  6 10 14
    ///       7 11 15
    ///
    ///       1  9 13
    /// Bsub  2 10 14
    ///       3 11 15
    ///
    ///       1 5 13
    /// Csub  2 6 14
    ///       3 7 15
    ///
    ///       1 5 9
    /// Dsub  2 6 10
    ///       3 7 11
    ///```
    /// Here each number is a matrix element index:
    /// ```text
    /// Asubd = 5 * (10*15 - 11*14) - 9 * (6*15 - 7*14) + 13 * (6*11 - 7*10)
    /// Bsubd = 1 * (10*15 - 11*14) - 9 * (2*15 - 3*14) + 13 * (2*11 - 3*10)
    /// Csubd = 1 * (6*15 - 7*14) - 5 * (2*15 - 3*14) + 13 * (2*7 - 3*6)
    /// Dsubd = 1 * (6*11 - 7*10) - 5 * (2*11 - 3*10) + 9 * (2*7 - 3*6)
    /// ```
    pub fn determinant(&self) -> f32 {
        let a = &self.m;
        let d6_11 = a[6] * a[11] - a[7] * a[10];
        let d6_15 = a[6] * a[15] - a[7] * a[14];
        let d10_15 = a[10] * a[15] - a[11] * a[14];
        let d2_11 = a[2] * a[11] - a[3] * a[10];
        let d2_15 = a[2] * a[15] - a[3] * a[14];
        let d2_7 = a[2] * a[7] - a[3] * a[6];

        let a_subd = a[5] * d10_15 - a[9] * d6_15 + a[13] * d6_11;
        let b_subd = a[1] * d10_15 - a[9] * d2_15 + a[13] * d2_11;
        let c_subd = a[1] * d6_15 - a[5] * d2_15 + a[13] * d2_7;
        let d_subd = a[1] * d6_11 - a[5] * d2_11 + a[9] * d2_7;

        return a[0] * a_subd - a[4] * b_subd + a[8] * c_subd - a[12] * d_subd;
    }

    /// Calculate inverse matrix.
    /// See [Math.determinant] for details.
    ///```text
    /// 0 4  8 12     A B C D
    /// 1 5  9 13     E F G H
    /// 2 6 10 14     I J K L
    /// 3 7 11 15     M N O P
    ///
    /// Asub 5  9 13   Bsub 1  9 13   Csub 1 5 13   Dsub 1 5  9
    ///      6 10 14        2 10 14        2 6 14        2 6 10
    ///      7 11 15        3 11 15        3 7 15        3 7 11
    ///
    /// Esub 4  8 12   Fsub 0  8 12   Gsub 0 4 12   Hsub 0 4  8
    ///      6 10 14        2 10 14        2 6 14        2 6 10
    ///      7 11 15        3 11 15        3 7 15        3 7 11
    ///
    /// Isub 4  8 12   Jsub 0  8 12   Ksub 0 4 12   Lsub 0 4  8
    ///      5  9 13        1  9 13        1 5 13        1 5  9
    ///      7 11 15        3 11 15        3 7 15        3 7 11
    ///
    /// Msub 4  8 12   Nsub 0  8 12   Osub 0 4 12   Psub 0 4  8
    ///      5  9 13        1  9 13        1 5 13        1 5  9
    ///      6 10 14        2 10 14        2 6 14        2 6 10
    ///```
    pub fn inverse(&self) -> Self {
        assert_eq!(self.m.len(), 16);
        let ma = &self.m;
        let d6_11 = ma[6] * ma[11] - ma[7] * ma[10];
        let d6_15 = ma[6] * ma[15] - ma[7] * ma[14];
        let d10_15 = ma[10] * ma[15] - ma[11] * ma[14];
        let d2_11 = ma[2] * ma[11] - ma[3] * ma[10];
        let d2_15 = ma[2] * ma[15] - ma[3] * ma[14];
        let d2_7 = ma[2] * ma[7] - ma[3] * ma[6];
        let d9_14 = ma[9] * ma[14] - ma[10] * ma[13];
        let d9_15 = ma[9] * ma[15] - ma[11] * ma[13];
        let d5_14 = ma[5] * ma[14] - ma[6] * ma[13];
        let d5_15 = ma[5] * ma[15] - ma[7] * ma[13];
        let d1_14 = ma[1] * ma[14] - ma[2] * ma[13];
        let d1_15 = ma[1] * ma[15] - ma[3] * ma[13];
        let d5_10 = ma[5] * ma[10] - ma[6] * ma[9];
        let d5_11 = ma[5] * ma[11] - ma[7] * ma[9];
        let d1_11 = ma[1] * ma[11] - ma[3] * ma[9];
        let d1_6 = ma[1] * ma[6] - ma[2] * ma[5];
        let d1_7 = ma[1] * ma[7] - ma[3] * ma[5];
        let d1_10 = ma[1] * ma[10] - ma[2] * ma[9];

        // row 0
        let a = ma[5] * d10_15 - ma[9] * d6_15 + ma[13] * d6_11;
        let b = ma[1] * d10_15 - ma[9] * d2_15 + ma[13] * d2_11;
        let c = ma[1] * d6_15 - ma[5] * d2_15 + ma[13] * d2_7;
        let d = ma[1] * d6_11 - ma[5] * d2_11 + ma[9] * d2_7;

        // row 1
        let e = ma[4] * d10_15 - ma[8] * d6_15 + ma[12] * d6_11;
        let f = ma[0] * d10_15 - ma[8] * d2_15 + ma[12] * d2_11;
        let g = ma[0] * d6_15 - ma[4] * d2_15 + ma[12] * d2_7;
        let h = ma[0] * d6_11 - ma[4] * d2_11 + ma[8] * d2_7;

        // row 2
        let i = ma[4] * d9_15 - ma[8] * d5_15 + ma[12] * d5_11;
        let j = ma[0] * d9_15 - ma[8] * d1_15 + ma[12] * d1_11;
        let k = ma[0] * d5_15 - ma[4] * d1_15 + ma[12] * d1_7;
        let l = ma[0] * d5_11 - ma[4] * d1_11 + ma[8] * d1_7;

        // row 3
        let m = ma[4] * d9_14 - ma[8] * d5_14 + ma[12] * d5_10;
        let n = ma[0] * d9_14 - ma[8] * d1_14 + ma[12] * d1_10;
        let o = ma[0] * d5_14 - ma[4] * d1_14 + ma[12] * d1_6;
        let p = ma[0] * d5_10 - ma[4] * d1_10 + ma[8] * d1_6;

        let det = ma[0] * a - ma[4] * b + ma[8] * c - ma[12] * d;
        assert_ne!(det, 0.0);

        let m = Matrix {
            m: [a, -e, i, -m, -b, f, -j, n, c, -g, k, -o, -d, h, -l, p],
        };

        let trans = m.transpose();
        let ood = 1.0 / det;
        trans * ood
    }

    /// Creates viewing matrix derived from the `eye` point, a reference point `target` indicating the center of the scene and vector `up`
    /// Helpful tip: it's better to think of this as a coordinate system rotation.
    /// # Arguments
    /// * `target` the target point in the scene
    /// * `eye`    the eye point
    /// * `up`     the upward vector, must not be parallel to the direction vector `dir = target - eye`
    pub fn look_at(target: Vector3f, eye: Vector3f, up: Vector3f) -> Self {
        let z_axis = (eye - target).normalize();
        let x_axis = up.cross(z_axis).normalize();
        let y_axis = z_axis.cross(x_axis);
        let m = Matrix {
            m: [
                x_axis.x, x_axis.y, x_axis.z, 0., y_axis.x, y_axis.y, y_axis.z, 0., z_axis.x,
                z_axis.y, z_axis.z, 0., 0., 0., 0., 1.,
            ],
        };
        m.transpose().translate(-eye.x, -eye.y, -eye.z)
    }
}

/// Multiplies this matrix by vector `v` and stores result in vector `r`. This is a right multiplication
/// # Arguments
/// * `self` this matrix
/// * `v` the vector
impl Mul<Vector3f> for Matrix {
    type Output = Vector3f;

    fn mul(self, v: Vector3f) -> Self::Output {
        let a = &self.m;
        Vector3f {
            x: a[0] * v.x + a[4] * v.y + a[8] * v.z + a[12],
            y: a[1] * v.x + a[5] * v.y + a[9] * v.z + a[13],
            z: a[2] * v.x + a[6] * v.y + a[10] * v.z + a[14],
        }
    }
}

impl Mul<Vector4f> for Matrix {
    type Output = Vector4f;

    /// Multiplies this matrix by vector `v`. This is a right multiplication
    /// # Arguments
    /// * `self` this matrix
    /// * `v`    the vector
    fn mul(self, rhs: Vector4f) -> Self::Output {
        let a = &self.m;
        Vector4f {
            x: a[0] * rhs.x + a[4] * rhs.y + a[8] * rhs.z + a[12] * rhs.w,
            y: a[1] * rhs.x + a[5] * rhs.y + a[9] * rhs.z + a[13] * rhs.w,
            z: a[2] * rhs.x + a[6] * rhs.y + a[10] * rhs.z + a[14] * rhs.w,
            w: a[3] * rhs.x + a[7] * rhs.y + a[11] * rhs.z + a[15] * rhs.w,
        }
    }
}

impl Mul<f32> for Matrix {
    type Output = Matrix;

    fn mul(self, rhs: f32) -> Self::Output {
        let a = &self.m;
        Matrix {
            m: [
                rhs * a[0],
                rhs * a[1],
                rhs * a[2],
                rhs * a[3],
                rhs * a[4],
                rhs * a[5],
                rhs * a[6],
                rhs * a[7],
                rhs * a[8],
                rhs * a[9],
                rhs * a[10],
                rhs * a[11],
                rhs * a[12],
                rhs * a[13],
                rhs * a[14],
                rhs * a[15],
            ],
        }
    }
}

impl Mul<Matrix> for Matrix {
    type Output = Matrix;

    /// Each row of this matrix is multiplied by the column of second (component-wise) and sum of results is stored in result's cell.
    /// # Arguments
    /// * `self` the first matrix
    /// * `rhs`  the second matrix
    fn mul(self, rhs: Matrix) -> Self::Output {
        let a = &self.m;
        let b = &rhs.m;
        // r0
        let m0 = a[0] * b[0] + a[4] * b[1] + a[8] * b[2] + a[12] * b[3];
        let m4 = a[0] * b[4] + a[4] * b[5] + a[8] * b[6] + a[12] * b[7];
        let m8 = a[0] * b[8] + a[4] * b[9] + a[8] * b[10] + a[12] * b[11];
        let m12 = a[0] * b[12] + a[4] * b[13] + a[8] * b[14] + a[12] * b[15];
        // r1
        let m1 = a[1] * b[0] + a[5] * b[1] + a[9] * b[2] + a[13] * b[3];
        let m5 = a[1] * b[4] + a[5] * b[5] + a[9] * b[6] + a[13] * b[7];
        let m9 = a[1] * b[8] + a[5] * b[9] + a[9] * b[10] + a[13] * b[11];
        let m13 = a[1] * b[12] + a[5] * b[13] + a[9] * b[14] + a[13] * b[15];
        // r2
        let m2 = a[2] * b[0] + a[6] * b[1] + a[10] * b[2] + a[14] * b[3];
        let m6 = a[2] * b[4] + a[6] * b[5] + a[10] * b[6] + a[14] * b[7];
        let m10 = a[2] * b[8] + a[6] * b[9] + a[10] * b[10] + a[14] * b[11];
        let m14 = a[2] * b[12] + a[6] * b[13] + a[10] * b[14] + a[14] * b[15];
        // r3
        let m3 = a[3] * b[0] + a[7] * b[1] + a[11] * b[2] + a[15] * b[3];
        let m7 = a[3] * b[4] + a[7] * b[5] + a[11] * b[6] + a[15] * b[7];
        let m11 = a[3] * b[8] + a[7] * b[9] + a[11] * b[10] + a[15] * b[11];
        let m15 = a[3] * b[12] + a[7] * b[13] + a[11] * b[14] + a[15] * b[15];

        Matrix {
            m: [
                m0, m1, m2, m3, m4, m5, m6, m7, m8, m9, m10, m11, m12, m13, m14, m15,
            ],
        }
    }
}

impl Add for Matrix {
    type Output = Matrix;

    /// Adds one matrix to another.
    /// * `self` the first matrix
    /// * `rhs`  the second matrix
    fn add(self, rhs: Self) -> Self::Output {
        let a = &self.m;
        let b = &rhs.m;
        let m0 = a[0] + b[0];
        let m1 = a[1] + b[1];
        let m2 = a[2] + b[2];
        let m3 = a[3] + b[3];

        let m4 = a[4] + b[4];
        let m5 = a[5] + b[5];
        let m6 = a[6] + b[6];
        let m7 = a[7] + b[7];

        let m8 = a[8] + b[8];
        let m9 = a[9] + b[9];
        let m10 = a[10] + b[10];
        let m11 = a[11] + b[11];

        let m12 = a[12] + b[12];
        let m13 = a[13] + b[13];
        let m14 = a[14] + b[14];
        let m15 = a[15] + b[15];

        Matrix {
            m: [
                m0, m1, m2, m3, m4, m5, m6, m7, m8, m9, m10, m11, m12, m13, m14, m15,
            ],
        }
    }
}

#[cfg(test)]
mod test {
    use approx::assert_relative_eq;

    use crate::vec4f::Vector4f;

    use super::*;

    fn assert_v(expected: Vector3f, actual: Vector3f) {
        assert_relative_eq!(expected.x, actual.x, epsilon = 0.001);
        assert_relative_eq!(expected.y, actual.y, epsilon = 0.001);
        assert_relative_eq!(expected.z, actual.z, epsilon = 0.001);
    }

    fn assert_v4(expected: Vector4f, actual: Vector4f) {
        assert_relative_eq!(expected.x, actual.x, epsilon = 0.001);
        assert_relative_eq!(expected.y, actual.y, epsilon = 0.001);
        assert_relative_eq!(expected.z, actual.z, epsilon = 0.001);
        assert_relative_eq!(expected.w, actual.w, epsilon = 0.001);
    }

    fn assert_m(a: Matrix, b: Matrix) {
        for i in 0..=15 {
            assert_relative_eq!(a.m[i], b.m[i], epsilon = 0.001);
        }
    }

    fn v3(x: f32, y: f32, z: f32) -> Vector3f {
        Vector3f::new(x, y, z)
    }

    fn v4(x: f32, y: f32, z: f32, w: f32) -> Vector4f {
        Vector4f::new(x, y, z, w)
    }

    #[test]
    fn identity() {
        assert_eq!(
            Matrix::identity().m,
            [1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.]
        )
    }

    #[test]
    fn translate() {
        fn translate(v: Vector3f, trans: Vector3f, expected: Vector3f) {
            let r = Matrix::identity().translate(trans.x, trans.y, trans.z) * v;
            assert_eq!(r, expected);
        }

        let args = [
            (v3(1., 2., 3.), v3(0., 0., 0.), v3(1., 2., 3.)),
            (v3(1., 2., 3.), v3(1., 0., 0.), v3(2., 2., 3.)),
            (v3(1., 2., 3.), v3(0., 1., 0.), v3(1., 3., 3.)),
            (v3(1., 2., 3.), v3(0., 0., 1.), v3(1., 2., 4.)),
            (v3(1., 2., 3.), v3(-1., 0., 0.), v3(0., 2., 3.)),
            (v3(1., 2., 3.), v3(0., -1., 0.), v3(1., 1., 3.)),
            (v3(1., 2., 3.), v3(0., 0., -1.), v3(1., 2., 2.)),
            (v3(1., 2., 3.), v3(1., 1., 1.), v3(2., 3., 4.)),
        ];
        for arg in args {
            let (v, t, exp) = arg;
            translate(v, t, exp);
        }
    }

    #[test]
    fn adding_translation_equals_to_multiplication() {
        let rm = Matrix::rotation(0., 0., 45.0f32.to_radians());
        let tm = Matrix::identity().translate(1., 2., 3.);
        let m = rm * tm;

        // add translation and store in separate matrix
        let m2 = rm.translate(1., 2., 3.);

        assert_eq!(m, m2);
    }

    #[test]
    fn translate_existing() {
        let r = Matrix::rotation(0., 0., 45.0f32.to_radians()).translate(1., 2., 3.);
        let r = r * v3(4., 5., 6.);
        assert_eq!(r, v3(-1.4142137, 8.485281, 9.));
    }

    #[test]
    fn scale() {
        let m = Matrix {
            m: std::array::from_fn::<f32, 16, _>(|i| i as f32 + 1.),
        };
        let r = m.scale(2., 4., 8.);
        assert_eq!(
            r,
            Matrix {
                m: [2., 4., 6., 8., 20., 24., 28., 32., 72., 80., 88., 96., 13., 14., 15., 16.]
            }
        )
    }

    #[test]
    fn transpose() {
        let m = Matrix {
            m: std::array::from_fn::<f32, 16, _>(|i| i as f32 + 1.),
        };
        let r = m.transpose();
        assert_eq!(
            r,
            Matrix {
                m: [1., 5., 9., 13., 2., 6., 10., 14., 3., 7., 11., 15., 4., 8., 12., 16.]
            }
        )
    }

    #[test]
    fn add() {
        let ma = Matrix {
            m: std::array::from_fn::<f32, 16, _>(|i| i as f32 + 1.),
        };
        let mb = Matrix {
            m: std::array::from_fn::<f32, 16, _>(|i| 16. - i as f32),
        };
        let r = ma + mb;
        assert_eq!(
            r,
            Matrix {
                m: [17., 17., 17., 17., 17., 17., 17., 17., 17., 17., 17., 17., 17., 17., 17., 17.]
            }
        )
    }

    #[test]
    fn multiply_by_scalar() {
        let m = Matrix {
            m: std::array::from_fn::<f32, 16, _>(|i| i as f32 + 1.),
        };
        let r = m * 2.;
        assert_eq!(
            r,
            Matrix {
                m: [2., 4., 6., 8., 10., 12., 14., 16., 18., 20., 22., 24., 26., 28., 30., 32.]
            }
        )
    }

    #[test]
    fn multiply_by_vector() {
        let m = Matrix {
            m: [
                1., 4., 7., 0., 2., 5., 8., 0., 3., 6., 9., 0., 0., 0., 0., 1.,
            ],
        };
        let vec = v3(1., 2., 3.);
        let r = m * vec;
        assert_eq!(r, v3(14., 32., 50.));
    }

    #[test]
    fn multiply() {
        let m = Matrix {
            m: [
                1., 5., 9., 0., 2., 6., 10., 0., 3., 7., 11., 0., 0., 0., 0., 1.,
            ],
        };
        let r = m * Matrix {
            m: [
                1., 0., 0., 1., 0., 1., 0., 2., 0., 0., 1., 3., 0., 0., 0., 1.,
            ],
        };
        assert_eq!(
            r,
            Matrix {
                m: [1., 5., 9., 1., 2., 6., 10., 2., 3., 7., 11., 3., 0., 0., 0., 1.]
            }
        );
    }

    #[test]
    fn rotate() {
        fn rotate(ax: f32, ay: f32, az: f32, v: Vector3f, expected: Vector3f) {
            let m = Matrix::rotation(ax.to_radians(), ay.to_radians(), az.to_radians());
            assert_v(m * v, expected);
        }
        let args = [
            (90.0, 0.0, 0.0, v3(0., 1., 0.), v3(0., 0., 1.)),
            (0.0, 90.0, 0.0, v3(1., 0., 0.), v3(0., 0., -1.)),
            (0.0, 0.0, 90.0, v3(1., 0., 0.), v3(0., 1., 0.)),
            (45.0, 0.0, 0.0, v3(0., 1., 0.), v3(0., 0.707, 0.707)),
            (0.0, 45.0, 0.0, v3(1., 0., 0.), v3(0.707, 0., -0.707)),
            (0.0, 0.0, 45.0, v3(1., 0., 0.), v3(0.707, 0.707, 0.)),
        ];
        for n in args {
            rotate(n.0, n.1, n.2, n.3, n.4);
        }
    }

    #[test]
    fn determinant() {
        let m = Matrix {
            m: [
                1., 3., 4., 10., 2., 5., 9., 11., 6., 8., 12., 15., 7., 13., 14., 16.,
            ],
        };
        assert_relative_eq!(m.determinant(), -594.0, epsilon = 0.001);
    }

    #[test]
    fn determinant_one_for_identity() {
        assert_eq!(Matrix::identity().determinant(), 1.0);
    }

    #[test]
    fn inverse() {
        let m = Matrix {
            m: [
                1., 2., 4., 6., 3., 1., 7., 10., 5., 8., 1., 12., 9., 11., 13., 1.,
            ],
        };
        let r = m.inverse();
        assert_m(
            r,
            Matrix {
                m: [
                    -1643. / 2369.,
                    744. / 2369.,
                    194. / 2369.,
                    90. / 2369.,
                    816. / 2369.,
                    -593. / 2369.,
                    81. / 2369.,
                    62. / 2369.,
                    439. / 2369.,
                    -20. / 2369.,
                    -209. / 2369.,
                    74. / 2369.,
                    104. / 2369.,
                    87. / 2369.,
                    80. / 2369.,
                    -85. / 2369.,
                ],
            },
        );
    }

    #[test]
    fn orthographic() {
        let m = Matrix::orthographic(0., 100., 200., 0., -1., 1.);
        let mut vec = v3(0., 0., 0.);
        assert_v(v3(-1., -1., 0.), m * vec);

        vec.set(50., 100., 0.);
        assert_v(v3(0., 0., 0.), m * vec);

        vec.set(100., 200., 0.);
        assert_v(v3(1., 1., 0.), m * vec);
    }

    #[test]
    fn look_at() {
        fn look_at(
            origin: Vector3f,
            eye: Vector3f,
            up: Vector3f,
            ax: Vector3f,
            ay: Vector3f,
            az: Vector3f,
        ) {
            let m = Matrix::look_at(origin, eye, up);
            let v1 = v3(1., 0., 0.);
            let v2 = v3(0., 1., 0.);
            let v3 = v3(0., 0., 1.);
            assert_v(ax, m * v1);
            assert_v(ay, m * v2);
            assert_v(az, m * v3);
        }
        let args = [
            (
                v3(1., 0., 0.),
                v3(0., 0., 0.),
                v3(0., 0., 1.),
                v3(0., 0., -1.),
                v3(-1., 0., 0.),
                v3(0., 1., 0.),
            ),
            (
                v3(1., 1., 0.),
                v3(0., 0., 0.),
                v3(0., 0., 1.),
                v3(0.707, 0., -0.707),
                v3(-0.707, 0., -0.707),
                v3(0., 1., 0.),
            ),
            (
                v3(0., 1., 0.),
                v3(0., 0., 0.),
                v3(0., 0., 1.),
                v3(1., 0., 0.),
                v3(0., 0., -1.),
                v3(0., 1., 0.),
            ),
            (
                v3(0., 0., 1.),
                v3(0., 0., 0.),
                v3(-1., 0., 0.),
                v3(0., -1., 0.),
                v3(-1., 0., 0.),
                v3(0., 0., -1.),
            ),
            (
                v3(0., 0., 0.),
                v3(-1., 0., 0.),
                v3(0., 0., 1.),
                v3(0., 0., -2.),
                v3(-1., 0., -1.),
                v3(0., 1., -1.),
            ),
            (
                v3(0., 0., 0.),
                v3(-1., -1., 0.),
                v3(0., 0., 1.),
                v3(0.707, 0., -2.121),
                v3(-0.707, 0., -2.121),
                v3(0., 1., -1.414),
            ),
            (
                v3(0., 0., 0.),
                v3(0., -1., 0.),
                v3(0., 0., 1.),
                v3(1., 0., -1.),
                v3(0., 0., -2.),
                v3(0., 1., -1.),
            ),
        ];
        for n in args {
            look_at(n.0, n.1, n.2, n.3, n.4, n.5);
        }
    }

    #[test]
    fn perspective() {
        fn perspective(v: Vector4f, expected: Vector4f) {
            let m = Matrix::perspective(-1., 1., 1., -1., 1., 10.);
            assert_v4(expected, m * v);
        }
        let args = [
            (v4(0., 0., 0., 1.), v4(0., 0., -2.222, 0.)),
            (v4(0., 0., 1., 1.), v4(0., 0., -3.444, -1.)),
            (v4(0., 0., 5., 1.), v4(0., 0., -8.333, -5.)),
            (v4(0., 0., 10., 1.), v4(0., 0., -14.444, -10.)),
            (v4(0., 0., 15., 1.), v4(0., 0., -20.555, -15.)),
            (v4(1., 1., 1., 1.), v4(1., 1., -3.444, -1.)),
            (v4(1., 1., 2., 1.), v4(1., 1., -4.666, -2.)),
        ];
    }
}
