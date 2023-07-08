use std::sync::Arc;

use crate::camera::Camera;

pub struct SceneData {
    pub camera: Camera,
    pub objects: Vec<SceneObject>,
    rend3_object_handles: Vec<rend3::types::ResourceHandle<rend3::types::Object>>,
    rend3_directional_handles: Vec<rend3::types::ResourceHandle<rend3::types::DirectionalLight>>,
}

impl SceneData {
    pub fn initialize(
        window_size: winit::dpi::PhysicalSize<u32>,
        rend3_renderer: &Arc<rend3::Renderer>,
    ) -> Self {
        let camera = Camera::initialize(window_size.width as f32, window_size.height as f32);

        // Initialize scene: basic cube and directional light.
        let basic_cube = SceneObject::create_basic_cube();
        let basic_cube_handle = basic_cube.add_to_rend3_renderer(rend3_renderer).unwrap();

        let objects = vec![basic_cube];
        let rend3_object_handles = vec![basic_cube_handle];

        // Create a single directional light
        //
        // We need to keep the directional light handle alive.
        let direction_handle =
            rend3_renderer.add_directional_light(rend3::types::DirectionalLight {
                color: glam::Vec3::ONE,
                intensity: 10.0,
                // Direction will be normalized
                direction: glam::Vec3::new(-1.0, -4.0, 2.0),
                distance: 400.0,
                resolution: 2048,
            });
        let rend3_directional_handles = vec![direction_handle];

        Self {
            camera,
            objects,
            rend3_object_handles,
            rend3_directional_handles,
        }
    }
}

pub struct SceneObject {
    mesh: RawMesh,
}

impl SceneObject {
    pub fn create_basic_cube() -> Self {
        let vertex_positions = [
            // far side (0.0, 0.0, 1.0)
            glam::Vec3::from([-1.0, -1.0, 1.0]),
            glam::Vec3::from([1.0, -1.0, 1.0]),
            glam::Vec3::from([1.0, 1.0, 1.0]),
            glam::Vec3::from([-1.0, 1.0, 1.0]),
            // near side (0.0, 0.0, -1.0)
            glam::Vec3::from([-1.0, 1.0, -1.0]),
            glam::Vec3::from([1.0, 1.0, -1.0]),
            glam::Vec3::from([1.0, -1.0, -1.0]),
            glam::Vec3::from([-1.0, -1.0, -1.0]),
            // right side (1.0, 0.0, 0.0)
            glam::Vec3::from([1.0, -1.0, -1.0]),
            glam::Vec3::from([1.0, 1.0, -1.0]),
            glam::Vec3::from([1.0, 1.0, 1.0]),
            glam::Vec3::from([1.0, -1.0, 1.0]),
            // left side (-1.0, 0.0, 0.0)
            glam::Vec3::from([-1.0, -1.0, 1.0]),
            glam::Vec3::from([-1.0, 1.0, 1.0]),
            glam::Vec3::from([-1.0, 1.0, -1.0]),
            glam::Vec3::from([-1.0, -1.0, -1.0]),
            // top (0.0, 1.0, 0.0)
            glam::Vec3::from([1.0, 1.0, -1.0]),
            glam::Vec3::from([-1.0, 1.0, -1.0]),
            glam::Vec3::from([-1.0, 1.0, 1.0]),
            glam::Vec3::from([1.0, 1.0, 1.0]),
            // bottom (0.0, -1.0, 0.0)
            glam::Vec3::from([1.0, -1.0, 1.0]),
            glam::Vec3::from([-1.0, -1.0, 1.0]),
            glam::Vec3::from([-1.0, -1.0, -1.0]),
            glam::Vec3::from([1.0, -1.0, -1.0]),
        ];

        let index_data: &[u32] = &[
            0, 1, 2, 2, 3, 0, // far
            4, 5, 6, 6, 7, 4, // near
            8, 9, 10, 10, 11, 8, // right
            12, 13, 14, 14, 15, 12, // left
            16, 17, 18, 18, 19, 16, // top
            20, 21, 22, 22, 23, 20, // bottom
        ];

        Self {
            mesh: RawMesh {
                vertices: vertex_positions.to_vec(),
                indices: index_data.to_vec(),
            },
        }
    }

    pub fn add_to_rend3_renderer(
        &self,
        rend3_renderer: &Arc<rend3::Renderer>,
    ) -> anyhow::Result<rend3::types::ResourceHandle<rend3::types::Object>> {
        // Create mesh and calculate smooth normals based on vertices
        let mesh = rend3::types::MeshBuilder::new(
            self.mesh.vertices.clone(),
            rend3::types::Handedness::Left,
        )
        .with_indices(self.mesh.indices.clone())
        .build()?;

        // Add mesh to renderer's world.
        //
        // All handles are refcounted, so we only need to hang onto the handle until we
        // make an object.
        let mesh_handle = rend3_renderer.add_mesh(mesh);

        // Add PBR material with all defaults except a single color.
        let material = rend3_routine::pbr::PbrMaterial {
            albedo: rend3_routine::pbr::AlbedoComponent::Value(glam::Vec4::new(0.0, 0.5, 0.5, 1.0)),
            ..rend3_routine::pbr::PbrMaterial::default()
        };
        let material_handle = rend3_renderer.add_material(material);

        // Combine the mesh and the material with a location to give an object.
        let object = rend3::types::Object {
            mesh_kind: rend3::types::ObjectMeshKind::Static(mesh_handle),
            material: material_handle,
            transform: glam::Mat4::IDENTITY,
        };

        // Creating an object will hold onto both the mesh and the material
        // even if they are deleted.
        //
        // We need to keep the object handle alive.
        let object_handle = rend3_renderer.add_object(object);

        Ok(object_handle)
    }
}

struct RawMesh {
    vertices: Vec<glam::Vec3>,
    indices: Vec<u32>,
}
