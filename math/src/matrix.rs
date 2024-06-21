/**
 * Column-oriented 4x4 matrix.
 * <pre>
 * A B C D
 * E F G H
 * I J K L
 * M N O P
 *
 * or as indices:
 *
 * 0 4  8 12
 * 1 5  9 13
 * 2 6 10 14
 * 3 7 11 15
</pre> *
 * So A have index 0, E - 1, I - 2, M - 3, etc.
 *
 * @author Yuriy Kiselev (uze@yandex.ru).
 */
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Matrix {
    pub m: [f32; 16],
}

impl Matrix {
    pub fn new() -> Self {
        Matrix {
            m: [0.; 16]
        }
    }

    pub fn identity() -> Self {
        Matrix {
            m: [
                1., 0., 0., 0.,
                0., 1., 0., 0.,
                0., 0., 1., 0.,
                0., 0., 0., 1.
            ]
        }
    }

    /**
     * Calculates orthographic projection matrix.
     *
     * @param left   the left screen coordinate (usually 0)
     * @param right  the right screen coordinate (usually width)
     * @param top    the top screen coordinate (usually height)
     * @param bottom the bottom screen coordinate (usually 0)
     * @param near   the near z value (for example -1)
     * @param far    the far z coordinate (for example 1)
     * @param m      the buffer to store resulting matrix in.
     */
    pub fn orthographic(left: f32, right: f32, top: f32, bottom: f32, near: f32, far: f32) -> Self {
        Matrix {
            m: [2. / (right - left), 0., 0., 0.,
                0., 2. / (top - bottom), 0., 0.,
                0., 0., -2. / (far - near), 0.,
                -(right + left) / (right - left),
                -(top + bottom) / (top - bottom),
                -(far + near) / (far - near),
                1.]
        }
    }

    /**
     * Calculates perspective projection matrix.
     *
     * @param left   the left screen coordinate (usually 0)
     * @param right  the right screen coordinate (usually width)
     * @param top    the top screen coordinate (usually height)
     * @param bottom the bottom screen coordinate (usually 0)
     * @param near   the near z value (for example -1)
     * @param far    the far z coordinate (for example 1)
     * @param m      the buffer to store resulting matrix in.
     */
    pub fn perspective(left: f32, right: f32, top: f32, bottom: f32, near: f32, far: f32) -> Self {
        Matrix {
            m: [2.0 * near / (right - left), 0., 0., 0.,
                0., (2.0 * near / (top - bottom)), 0., 0.,
                (right + left) / (right - left),
                (top + bottom) / (top - bottom),
                -(far + near) / (far - near),
                -1.,
                0., 0., -2.0 * far * near / (far - near), 0.
            ]
        }
    }

    /**
     * Calculates perspective projection matrix.
     *
     * @param fow   the horizontal field of view (in radians)
     * @param ratio the aspect ratio between width and height of screen
     * @param near  the near z coordinate (should be > 0)
     * @param far   the far z coordinate
     * @param m     the buffer to store resulting matrix in.
     */
    pub fn perspective_fow(fow: f32, ratio: f32, near: f32, far: f32) -> Self {
        let w = near * (0.5 * fow).tan();
        let h = w / ratio;
        Matrix::perspective(-w, w, h, -h, near, far)
    }

    /**
     * Multiplies matrix `a` by translation matrix derived from `(dx,dy,dz)` and stores result in `result`.
     *
     * @param a      the original matrix to add translation to
     * @param dx     x translation
     * @param dy     y translation
     * @param dz     z translation
     * @param result the buffer to store result.
     */
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
        Matrix {
            m
        }
    }

    /**
     * Combines scaling `(sx,sy,sz)` with matrix `a` and stores resulting matrix in `result`
     *
     * @param a      the original matrix
     * @param sx     x scaling factor
     * @param sy     y  scaling factor
     * @param sz     z  scaling factor
     * @param result the buffer to store result
     */
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
                m0, m1, m2, m3,
                m4, m5, m6, m7,
                m8, m9, m10, m11,
                m12, m13, m14, m15
            ]
        }
    }

    /**
     * Transposes the matrix `a` and stores resulting matrix in `result`
     *
     * @param a      the matrix to transpose
     * @param result the buffer to store result
     */
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
                m0, m4, m8, m12,
                m1, m5, m9, m13,
                m2, m6, m10, m14,
                m3, m7, m11, m15
            ]
        }
    }

    /**
     * Initializes `result` with rotation matrix from Euler's angles `ax, ay, az`.
     *
     * @param ax     the x-axis rotation angle (counter-clockwise)
     * @param ay     the y-axis rotation angle (counter-clockwise)
     * @param az     the z-axis rotation angle (counter-clockwise)
     * @param result the buffer to store result
     */
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
                0., 0., 0., 1.
            ]
        }
    }

    /**
     * For 2x2 matrix M determinant is A*D - B*C
     * <pre>
     *     | A B |
     * M = |     |
     *     | C D |
     * </pre>
     *
     * So for our 4x4 matrix determinant `Det = A * Asubd - B * Bsubd + C * Csubd - D*Dsubd` where
     * <pre>
     *     0 4  8 12
     *     1 5  9 13
     * M   2 6 10 14
     *     3 7 11 15
     *
     *       5  9 13
     * Asub  6 10 14
     *       7 11 15
     *
     *       1  9 13
     * Bsub  2 10 14
     *       3 11 15
     *
     *       1 5 13
     * Csub  2 6 14
     *       3 7 15
     *
     *       1 5 9
     * Dsub  2 6 10
     *       3 7 11
     *
     * Here each number is a matrix element index:
     * Asubd = 5 * (10*15 - 11*14) - 9 * (6*15 - 7*14) + 13 * (6*11 - 7*10)
     * Bsubd = 1 * (10*15 - 11*14) - 9 * (2*15 - 3*14) + 13 * (2*11 - 3*10)
     * Csubd = 1 * (6*15 - 7*14) - 5 * (2*15 - 3*14) + 13 * (2*7 - 3*6)
     * Dsubd = 1 * (6*11 - 7*10) - 5 * (2*11 - 3*10) + 9 * (2*7 - 3*6)
    </pre> *
     *
     * @param a the matrix
     * @return the determinant of a matrix
     */
    pub fn determinant(&self) -> f32 {
        let a = &self.m;
        let d6_11 = a[6] * a[11] - a[7] * a[10];
        let d6_15 = a[6] * a[15] - a[7] * a[14];
        let d10_15 = a[10] * a[15] - a[11] * a[14];
        let d2_11 = a[2] * a[11] - a[3] * a[10];
        let d2_15 = a[2] * a[15] - a[3] * a[14];
        let d2_7 = a[2] * a[7] - a[3] * a[6];

        let Asubd = a[5] * d10_15 - a[9] * d6_15 + a[13] * d6_11;
        let Bsubd = a[1] * d10_15 - a[9] * d2_15 + a[13] * d2_11;
        let Csubd = a[1] * d6_15 - a[5] * d2_15 + a[13] * d2_7;
        let Dsubd = a[1] * d6_11 - a[5] * d2_11 + a[9] * d2_7;

        return a[0] * Asubd - a[4] * Bsubd + a[8] * Csubd - a[12] * Dsubd;
    }
}

#[cfg(test)]
mod test {}