use std::ops::Mul;
use encase::private::AsRefMatrixParts;

use super::quaternion::Quaternion;
use super::vector::{Vector3, Vector4};
use super::Radians;

#[derive(Clone, Copy)]
pub struct Matrix4 {
    internal: glam::Mat4,
}

impl From<Quaternion> for Matrix4 {
    fn from(q: Quaternion) -> Self {
        Self {
            internal: glam::Mat4::from_quat(q.internal),
        }
    }
}

impl Matrix4 {
    #[cfg_attr(rustfmt, rustfmt_skip)]    
    pub const fn new(
        c0r0: f32, c0r1: f32, c0r2: f32, c0r3: f32,
        c1r0: f32, c1r1: f32, c1r2: f32, c1r3: f32,
        c2r0: f32, c2r1: f32, c2r2: f32, c2r3: f32,
        c3r0: f32, c3r1: f32, c3r2: f32, c3r3: f32,
    ) -> Self {
        Self {
            internal: glam::Mat4::from_cols(
                glam::Vec4::new(c0r0, c0r1, c0r2, c0r3), 
                glam::Vec4::new(c1r0, c1r1, c1r2, c1r3), 
                glam::Vec4::new(c2r0, c2r1, c2r2, c2r3), 
                glam::Vec4::new(c3r0, c3r1, c3r2, c3r3)
            ),        
        }
    }

    pub fn invert(&self) -> Result<Self, ()> {
        let internal = self.internal.inverse();
        // it seems like this is how glam indicates the matrix was not invertible
        if internal.determinant() == 0. {
            return Err(());
        }

        Ok(Self {
            internal,
        })
    }

    pub fn from_translation(t: Vector3) -> Self {
        Self {
            internal: glam::Mat4::from_translation(t.internal),
        }
    }

    pub fn create_perspective<A>(
        vertical_fov: A,
        aspect_ratio: f32,
        near_clipping_z: f32,
        far_clipping_z: Option<f32>,
    ) -> Self
    where
        A: Into<Radians>,
    {
        let fov_y_radians = vertical_fov.into().0;

        let internal = match far_clipping_z {
            Some(z_far) => glam::Mat4::perspective_lh(fov_y_radians, aspect_ratio, near_clipping_z, z_far),
            None => glam::Mat4::perspective_infinite_lh(fov_y_radians, aspect_ratio, near_clipping_z),
        };
        
        Self {
            internal,        }
    }

    /// Second column of matrix.
    pub fn y(&self) -> Vector4 {
        Vector4 {
            internal: self.internal.y_axis,
        }
    }

    pub fn to_glam_mat4(&self) -> glam::Mat4 {
        self.internal
    }
}

impl Into<[[f32; 4]; 4]> for Matrix4 {
    fn into(self) -> [[f32; 4]; 4] {
        *self.internal.as_ref_parts()
    }
}

impl Mul<Matrix4> for Matrix4 {
    type Output = Self;

    fn mul(self, rhs: Matrix4) -> Self::Output {
        Self {
            internal: self.internal * rhs.internal,
        }
    }
}

pub struct Matrix3 {
    internal: glam::Mat3,
}

impl From<Quaternion> for Matrix3 {
    fn from(q: Quaternion) -> Self {
        Self {
            internal: glam::Mat3::from_quat(q.internal),
        }
    }
}

impl Matrix3 {
    pub fn invert(&self) -> Result<Self, ()> {
        let internal = self.internal.inverse();
        if internal.determinant() == 0. {
            return Err(());
        }

        Ok(Self {
            internal,
        })
    }

    pub fn x(&self) -> Vector3 {
        Vector3 {
            internal: self.internal.x_axis,
        }
    }
}
