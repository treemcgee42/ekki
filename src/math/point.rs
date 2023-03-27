use std::ops::{Add, Sub};

use super::vector::{Vector2, Vector3};

#[derive(Clone, Copy)]
pub struct Point2 {
    pub(super) internal: glam::Vec2,
}

impl Point2 {
    #[inline]
    pub fn new(x: f32, y: f32) -> Self {
        Self {
            internal: glam::Vec2::new(x, y),
        }
    }

    #[inline]
    pub fn x(&self) -> f32 {
        self.internal.x
    }

    #[inline]
    pub fn y(&self) -> f32 {
        self.internal.y
    }
}

impl Sub for Point2 {
    type Output = Vector2;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::Output::new(self.x() - rhs.x(), self.y() - rhs.y())
    }
}

impl Add<Vector2> for Point2 {
    type Output = Point2;

    fn add(self, rhs: Vector2) -> Self::Output {
        Self::Output::new(self.x() + rhs.x(), self.y() + rhs.y())
    }
}

impl Sub<Vector2> for Point2 {
    type Output = Point2;

    fn sub(self, rhs: Vector2) -> Self::Output {
        Self::Output::new(self.x() - rhs.x(), self.y() - rhs.y())
    }
}

#[derive(Clone, Copy)]
pub struct Point3 {
    pub(super) internal: glam::Vec3,
}

impl Sub for Point3 {
    type Output = Vector3;

    fn sub(self, rhs: Self) -> Self::Output {
        Vector3 {
            internal: self.internal - rhs.internal,
        }
    }
}

impl Sub for &Point3 {
    type Output = Vector3;

    fn sub(self, rhs: Self) -> Self::Output {
        self.clone() - rhs.clone()
    }
}

impl Into<[f32; 3]> for Point3 {
    fn into(self) -> [f32; 3] {
        self.internal.into()
    }
}

impl Point3 {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self {
            internal: glam::Vec3::new(x, y, z),
        }
    }

    pub fn origin() -> Self {
        Self {
            internal: glam::Vec3::new(0.0, 0.0, 0.0),
        }
    }

    pub fn to_vec3(&self) -> Vector3 {
        Vector3 {
            internal: self.internal,
        }
    }
}
