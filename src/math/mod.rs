pub mod color;
pub mod matrix;
pub mod point;
pub mod quaternion;
pub mod vector;

pub struct Radians(pub f32);
pub struct Degrees(pub f32);

impl Into<Radians> for Degrees {
    fn into(self) -> Radians {
        Radians(self.0.to_radians())
    }
}
