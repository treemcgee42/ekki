use super::point::Point3;
use std::ops::{Add, AddAssign, Mul, Neg, Sub};

#[derive(Clone)]
pub struct Vector2 {
    pub(super) internal: glam::Vec2,
}

impl Add<Vector2> for Vector2 {
    type Output = Vector2;

    fn add(self, rhs: Vector2) -> Self::Output {
        Self::Output {
            internal: self.internal + rhs.internal,
        }
    }
}

impl Sub<Vector2> for Vector2 {
    type Output = Vector2;

    fn sub(self, rhs: Vector2) -> Self::Output {
        Self {
            internal: self.internal - rhs.internal,
        }
    }
}

impl Sub<&Vector2> for &Vector2 {
    type Output = Vector2;

    fn sub(self, rhs: &Vector2) -> Self::Output {
        Self::Output {
            internal: self.internal - rhs.internal,
        }
    }
}

impl AddAssign for Vector2 {
    fn add_assign(&mut self, rhs: Self) {
        self.internal += rhs.internal;
    }
}

impl Mul<Vector2> for f32 {
    type Output = Vector2;

    fn mul(self, rhs: Vector2) -> Self::Output {
        Self::Output {
            internal: self * rhs.internal,
        }
    }
}

impl Mul<&Vector2> for f32 {
    type Output = Vector2;

    fn mul(self, rhs: &Vector2) -> Self::Output {
        Self::Output {
            internal: self * rhs.internal,
        }
    }
}

impl Neg for Vector2 {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self {
            internal: -self.internal,
        }
    }
}

impl Into<[f32; 2]> for Vector2 {
    fn into(self) -> [f32; 2] {
        self.internal.into()
    }
}

impl Vector2 {
    pub fn new(x: f32, y: f32) -> Self {
        Self {
            internal: glam::Vec2::new(x, y),
        }
    }

    pub fn x(&self) -> f32 {
        self.internal.x
    }

    pub fn y(&self) -> f32 {
        self.internal.y
    }

    pub fn normalize(self) -> Self {
        Self {
            internal: self.internal.normalize(),
        }
    }

    /// Get the vector perpindicular to the parameter, which the direction you would
    /// get by rotating the parameter clockwise. The length of the resulting vector
    /// is the same as the parameter.
    pub fn get_perpindicular_cw_vector(v: Self) -> Self {
        Self {
            internal: glam::Vec2::new(v.y(), -v.x()),
        }
    }

    pub fn are_approximately_equal(v1: &Self, v2: &Self) -> bool {
        if f32::abs(v1.x() - v2.x()) < f32::EPSILON && f32::abs(v1.y() - v2.y()) < f32::EPSILON {
            return true;
        }

        false
    }
}

pub struct Vector3 {
    pub(super) internal: glam::Vec3,
}

impl Add for Vector3 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            internal: self.internal + rhs.internal,
        }
    }
}

impl Neg for Vector3 {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self {
            internal: -self.internal,
        }
    }
}

impl From<Point3> for Vector3 {
    fn from(p: Point3) -> Self {
        Self {
            internal: p.internal,
        }
    }
}

impl Vector3 {
    pub fn x(&self) -> f32 {
        self.internal.x
    }

    pub fn y(&self) -> f32 {
        self.internal.y
    }

    pub fn xy(&self) -> Vector2 {
        Vector2 {
            internal: glam::Vec2::new(self.x(), self.y()),
        }
    }

    pub fn unit_x() -> Self {
        Self {
            internal: glam::Vec3::new(1., 0., 0.),
        }
    }

    pub fn unit_y() -> Self {
        Self {
            internal: glam::Vec3::new(0., 1., 0.),
        }
    }

    pub fn dot(v1: Self, v2: Self) -> f32 {
        glam::Vec3::dot(v1.internal, v2.internal)
    }
}

pub struct Vector4 {
    pub(super) internal: glam::Vec4,
}

impl Vector4 {
    pub fn truncate(self) -> Vector3 {
        Vector3 {
            internal: self.internal.truncate(),
        }
    }
}
