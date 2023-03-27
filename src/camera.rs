use std::f64::consts;

use crate::math::{
    matrix::{Matrix3, Matrix4},
    point::Point3,
    quaternion::Quaternion,
    vector::{Vector2, Vector3},
    Degrees, Radians,
};

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: Matrix4 = Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
);

pub struct Camera {
    view_info: ViewInfo,
    projection_info: ProjectionInfo,
    view_projection_matrix: Matrix4,
}

impl Camera {
    pub fn initialize(screen_width: f32, screen_height: f32) -> Self {
        let view_info = ViewInfo::initialize();
        let projection_info = ProjectionInfo::initialize(screen_width, screen_height);
        let view_projection_matrix = OPENGL_TO_WGPU_MATRIX
            * projection_info.get_projection_matrix()
            * view_info.get_view_matrix();

        Self {
            view_info,
            projection_info,
            view_projection_matrix,
        }
    }

    fn build_view_projection_matrix(&self) -> Matrix4 {
        return OPENGL_TO_WGPU_MATRIX
            * self.projection_info.get_projection_matrix()
            * self.view_info.get_view_matrix();
    }

    fn rebuild_view_projection_matrix(&mut self) {
        self.view_projection_matrix = self.build_view_projection_matrix();
    }

    pub fn set_aspect_ratio(&mut self, arg: f32) {
        self.projection_info.set_aspect_ratio(arg);
        self.rebuild_view_projection_matrix();
    }

    pub fn get_view_projection_matrix(&self) -> Matrix4 {
        self.view_projection_matrix
    }
    pub fn get_view_projection_matrix_inverse(&self) -> Matrix4 {
        self.get_view_projection_matrix().invert().unwrap()
    }

    pub fn to_rend3_camera(&self) -> rend3::types::Camera {
        rend3::types::Camera {
            projection: rend3::types::CameraProjection::Perspective {
                vfov: self.projection_info.vertical_fov,
                near: self.projection_info.z_near,
            },
            view: self.view_info.get_view_matrix().to_glam_mat4(),
        }
    }

    pub fn solidify_view_info(&mut self) {
        self.view_info.current_rotation =
            (self.view_info.current_rotation * self.view_info.rotation_modifier).normalize();
        self.view_info.rotation_modifier = Quaternion::identity();
        if Vector3::dot(self.view_info.view_matrix.y().truncate(), Vector3::unit_y()) < 0.0 {
            self.view_info.should_reverse = true;
        } else {
            self.view_info.should_reverse = false;
        }
    }

    pub fn handle_window_resize(&mut self, new_width: f32, new_height: f32) {
        self.set_aspect_ratio(new_width / new_height);
    }

    fn set_rotation_modifier(&mut self, val: Quaternion) {
        self.view_info.set_rotation_modifier(val);
        self.rebuild_view_projection_matrix();
    }

    pub fn get_z_near(&self) -> f32 {
        self.projection_info.z_near
    }

    pub fn get_z_far(&self) -> Option<f32> {
        self.projection_info.z_far
    }
}

// Rotations that the user can do with the viewport camera. The two main
// kinds we want to target are "turntable" and "trackball". The Blender
// implementation is in `source/blender/editors/space_view3d/view3d_navigate_rotate.c`.
impl Camera {
    pub fn turntable_rotate(&mut self, delta_mouse: Vector2, window_size: (f32, f32)) {
        let x_angle_scale_factor = 2.0 * (consts::PI as f32) / window_size.0;
        let y_angle_scale_factor = consts::PI as f32 / window_size.1;

        let current_rotation_matrix = Matrix3::from(self.view_info.current_rotation);
        let current_rotation_matrix_inverse = current_rotation_matrix.invert().unwrap();

        // Axis around which we rotate to get up/down rotation.
        let up_down_rotation_axis = current_rotation_matrix_inverse.x();
        // Axis around which we rotate to get side rotation.
        let side_side_rotation_axis = Vector3::unit_y();

        let up_down_rotation_angle = y_angle_scale_factor * 1.0 * delta_mouse.y();
        let side_side_rotation_angle = {
            let reverse_factor = {
                if self.view_info.should_reverse {
                    -1.0
                } else {
                    1.0
                }
            };
            x_angle_scale_factor * reverse_factor * delta_mouse.x()
        };

        let rotation_modifier = {
            let up_down_rotation = Quaternion::rotation_from_axis_angle(
                up_down_rotation_axis,
                Radians(up_down_rotation_angle),
            );

            let side_side_rotation = Quaternion::rotation_from_axis_angle(
                side_side_rotation_axis,
                Radians(side_side_rotation_angle),
            );

            (up_down_rotation * side_side_rotation).normalize()
        };
        self.set_rotation_modifier(rotation_modifier);
    }
}

/// The motivation for this abstraction was to couple the view
/// matrix to the data that it is defined by. Ideally, we would
/// like to avoid recomputing the matrix every frame (given that
/// the camera isn't moving). This would mean storing the view
/// matrix between draws. This abstraction just makes it harder to
/// accidentally change one of these data points without changing
/// the view matrix.
struct ViewInfo {
    /// How far the camera is from the look_at point, without accounting for
    /// rotation.
    z_offset: Point3,
    /// The point the camera is pointing at / the center point.
    look_at: Point3,
    /// Represents the rotation needed to get to the last set camera rotation.
    /// A camera rotation is set, for example, after releasing the keybind that
    /// allowed for a turntable rotation.
    current_rotation: Quaternion,
    /// The total rotation for the camera is current_rotation * rotation_modifier.
    rotation_modifier: Quaternion,
    /// Whether the horizontal mouse input should be reversed in the turntable
    /// camera. This makes it so that the expected controls aren't reversed
    /// when viewing the scene upside down.
    should_reverse: bool,
    /// world -> camera space
    view_matrix: Matrix4,
}

impl ViewInfo {
    fn initialize() -> Self {
        let z_offset = Point3::new(0.0, 0.0, -7.0);
        let look_at = Point3::origin();

        let current_rotation =
            Quaternion::rotation_from_axis_angle(Vector3::unit_x(), Radians(-0.5));
        let rotation_modifier = Quaternion::identity();
        let view_matrix = Self::build_view_matrix_from_rotation_and_offset(
            current_rotation,
            Vector3::from(z_offset) + Vector3::from(look_at),
        );

        Self {
            z_offset,
            look_at,
            current_rotation,
            rotation_modifier,
            should_reverse: false,
            view_matrix,
        }
    }

    fn build_view_matrix_from_rotation_and_offset(
        rotation: Quaternion,
        offset: Vector3,
    ) -> Matrix4 {
        let rotation_matrix = Matrix4::from(rotation);
        let negative_offset_matrix = Matrix4::from_translation(-offset);

        negative_offset_matrix * rotation_matrix
    }

    fn rebuild_view_matrix(&mut self) {
        self.view_matrix = Self::build_view_matrix_from_rotation_and_offset(
            self.current_rotation * self.rotation_modifier,
            Vector3::from(self.z_offset) + Vector3::from(self.look_at),
        );
    }

    fn set_rotation_modifier(&mut self, value: Quaternion) {
        self.rotation_modifier = value;
        self.rebuild_view_matrix();
    }

    fn get_view_matrix(&self) -> Matrix4 {
        self.view_matrix
    }
}

/// Encapsulates information needed to build the projection
/// matrix. Justification is described in `ViewInfo`.
struct ProjectionInfo {
    /// in degrees
    vertical_fov: f32,
    aspect_ratio: f32,
    /// distance to near clipping plane
    z_near: f32,
    /// distance to far clipping plane    
    z_far: Option<f32>,
    projection_matrix: Matrix4,
}

impl ProjectionInfo {
    fn initialize(screen_width: f32, screen_height: f32) -> Self {
        let vertical_fov = 60.0;
        let aspect_ratio = screen_width / screen_height;
        let z_near = 0.1;
        let z_far = None;
        let projection_matrix =
            Matrix4::create_perspective(Degrees(vertical_fov), aspect_ratio, z_near, z_far);

        Self {
            vertical_fov,
            aspect_ratio,
            z_near,
            z_far,
            projection_matrix,
        }
    }

    fn build_projection_matrix(&mut self) {
        self.projection_matrix = Matrix4::create_perspective(
            Degrees(self.vertical_fov),
            self.aspect_ratio,
            self.z_near,
            self.z_far,
        );
    }

    fn set_aspect_ratio(&mut self, arg: f32) {
        self.aspect_ratio = arg;
        self.build_projection_matrix();
    }
    fn get_projection_matrix(&self) -> Matrix4 {
        self.projection_matrix
    }
}
