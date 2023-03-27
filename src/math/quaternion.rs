use std::ops::Mul;

use super::{vector::Vector3, Radians};

#[derive(Clone, Copy)]
pub struct Quaternion {
    pub(super) internal: glam::Quat,
}

impl Quaternion {
    pub fn new(w: f32, xi: f32, yj: f32, zk: f32) -> Self {
        Self {
            internal: glam::Quat::from_xyzw(xi, yj, zk, w),
        }
    }

    pub fn identity() -> Self {
        Self::new(1.0, 0.0, 0.0, 0.0)
    }

    pub fn rotation_from_axis_angle<A>(axis: Vector3, angle: A) -> Self
    where
        A: Into<Radians>,
    {
        let rad: Radians = angle.into();
        Self {
            internal: glam::Quat::from_axis_angle(axis.internal, rad.0),
        }
    }

    pub fn normalize(self) -> Self {
        Self {
            internal: self.internal.normalize(),
        }
    }
}

impl Mul for Quaternion {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self {
            internal: self.internal * rhs.internal,
        }
    }
}
