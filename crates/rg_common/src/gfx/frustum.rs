use glam::{Mat4, Vec3, Vec4};

#[derive(Debug, Clone, Copy)]
pub struct AABB {
    pub min: Vec3,
    pub max: Vec3,
}

#[derive(Debug, Clone, Copy)]
pub struct Plane {
    pub normal: Vec3,
    pub d: f32,
}

impl Plane {
    fn new(coefs: Vec4) -> Self {
        let xyz = Vec3::new(coefs.x, coefs.y, coefs.z);
        let length = xyz.length();

        Self {
            normal: xyz / length,
            d: coefs.w / length,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Intersection {
    Outside,
    Inside,
    Intersect,
}

pub struct Frustum {
    planes: [Plane; 6],
}

impl Frustum {
    pub fn from_view_projection(vp: Mat4) -> Self {
        let r0 = vp.row(0);
        let r1 = vp.row(1);
        let r2 = vp.row(2);
        let r3 = vp.row(3);

        let left = Plane::new(r3 + r0);

        let right = Plane::new(r3 - r0);

        let bottom = Plane::new(r3 + r1);

        let top = Plane::new(r3 - r1);

        let near = Plane::new(r2);

        let far = Plane::new(r3 - r2);

        Self {
            planes: [left, right, bottom, top, near, far],
        }
    }

    pub fn intersects_aabb(&self, aabb: &AABB) -> Intersection {
        let mut total_inside = 0;

        for plane in &self.planes {
            // Find positive (near along normal) vertex
            // and negative (far along normal) vertex of AABB.
            let mut p_vertex = aabb.min;
            let mut n_vertex = aabb.max;

            if plane.normal.x >= 0.0 {
                p_vertex.x = aabb.max.x;
                n_vertex.x = aabb.min.x;
            }
            if plane.normal.y >= 0.0 {
                p_vertex.y = aabb.max.y;
                n_vertex.y = aabb.min.y;
            }
            if plane.normal.z >= 0.0 {
                p_vertex.z = aabb.max.z;
                n_vertex.z = aabb.min.z;
            }

            // Calculate distance from plane to p_vertex.
            // dot(normal, point) + d
            if plane.normal.dot(p_vertex) + plane.d < 0.0 {
                // If nearest point is behind plane, 
                // then whole box is outside of frustum.
                return Intersection::Outside;
            }

            // Calculate distance to n_vertex.
            if plane.normal.dot(n_vertex) + plane.d < 0.0 {
                // If far point is behind plane and near is in front — box intersects plane.
                // Otherwise box is fully in front of plane.
            } else {
                total_inside += 1;
            }
        }

        // If box passed all checks and for all planes far point was in front:
        if total_inside == 6 {
            Intersection::Inside
        } else {
            Intersection::Intersect
        }
    }
}
