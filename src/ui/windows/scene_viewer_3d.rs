use super::*;

pub struct SceneViewer3D {
    info: WindowInfo,
    base_rendergraph: crate::base::BaseRenderGraph,
    pbr_routine: rend3_routine::pbr::PbrRoutine,
    tonemapping_routine: rend3_routine::tonemapping::TonemappingRoutine,
    grid_render_routine: GridRenderRoutine,
    cam: Camera,
    _object_handle: rend3::types::ResourceHandle<rend3::types::Object>,
    _directional_handle: rend3::types::DirectionalLightHandle,
}

impl SceneViewer3D {
    pub fn create<T>(window_target: &winit::event_loop::EventLoopWindowTarget<T>) -> Self
    where
        T: 'static,
    {
        let window_init_info = WindowInfoInitializeInfo {
            title: "3d scene editor".to_string(),
            ..Default::default()
        };
        let info = WindowInfo::initialize(window_target, window_init_info);

        // Create the shader preprocessor with all the default shaders added.
        let mut spp = rend3::ShaderPreProcessor::new();
        rend3_routine::builtin_shaders(&mut spp);

        // Create the base rendergraph.
        let base_rendergraph = crate::base::BaseRenderGraph::new(&info.rend3_renderer, &spp);

        let grid_render_routine =
            GridRenderRoutine::new(&info.rend3_renderer, info.preferred_texture_format.clone());

        let mut data_core = info.rend3_renderer.data_core.lock();
        let pbr_routine = rend3_routine::pbr::PbrRoutine::new(
            &info.rend3_renderer,
            &mut data_core,
            &spp,
            &base_rendergraph.interfaces,
        );
        drop(data_core);
        let tonemapping_routine = rend3_routine::tonemapping::TonemappingRoutine::new(
            &info.rend3_renderer,
            &spp,
            &base_rendergraph.interfaces,
            info.preferred_texture_format,
        );

        let cam = Camera::initialize(
            info.window_size.width as f32,
            info.window_size.height as f32,
        );

        info.rend3_renderer.set_camera_data(cam.to_rend3_camera());

        // Create mesh and calculate smooth normals based on vertices
        let mesh = create_mesh();

        // Add mesh to renderer's world.
        //
        // All handles are refcounted, so we only need to hang onto the handle until we
        // make an object.
        let mesh_handle = info.rend3_renderer.add_mesh(mesh);

        // Add PBR material with all defaults except a single color.
        let material = rend3_routine::pbr::PbrMaterial {
            albedo: rend3_routine::pbr::AlbedoComponent::Value(glam::Vec4::new(0.0, 0.5, 0.5, 1.0)),
            ..rend3_routine::pbr::PbrMaterial::default()
        };
        let material_handle = info.rend3_renderer.add_material(material);

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
        let _object_handle = info.rend3_renderer.add_object(object);

        // Create a single directional light
        //
        // We need to keep the directional light handle alive.
        let _directional_handle =
            info.rend3_renderer
                .add_directional_light(rend3::types::DirectionalLight {
                    color: glam::Vec3::ONE,
                    intensity: 10.0,
                    // Direction will be normalized
                    direction: glam::Vec3::new(-1.0, -4.0, 2.0),
                    distance: 400.0,
                    resolution: 2048,
                });

        Self {
            info,
            base_rendergraph,
            pbr_routine,
            tonemapping_routine,
            grid_render_routine,
            cam,
            _object_handle,
            _directional_handle,
        }
    }
}

impl WindowLike for SceneViewer3D {
    fn get_window_id(&self) -> winit::window::WindowId {
        self.info.window_id
    }

    fn request_redraw(&self) {
        self.info.raw_window.request_redraw()
    }

    fn egui_event_consumed(&mut self, event: &winit::event::WindowEvent) -> bool {
        self.info
            .egui_winit_state
            .on_event(&self.info.egui_context, event)
            .consumed
    }

    fn resize(&mut self, physical_size: winit::dpi::PhysicalSize<u32>) {
        self.info.resize_default(physical_size);

        self.cam
            .handle_window_resize(physical_size.width as f32, physical_size.height as f32);
        self.info
            .rend3_renderer
            .set_camera_data(self.cam.to_rend3_camera());
    }

    fn redraw(&mut self) -> Option<Vec<WindowRedrawCallbackCommand>> {
        // UI
        self.info.egui_context.begin_frame(
            self.info
                .egui_winit_state
                .take_egui_input(&self.info.raw_window),
        );

        egui::Window::new("Change color")
            .resizable(true)
            .show(&self.info.egui_context, |ui| {
                ui.label("Change the color of the cube");
            });

        egui::Window::new("Console")
            .resizable(true)
            .show(&self.info.egui_context, |ui| {
                ui::console::draw_egui_console_menu(ui);
                ui::console::draw_egui_logging_lines(ui);
            });

        let egui::FullOutput {
            shapes,
            textures_delta,
            ..
        } = self.info.egui_context.end_frame();

        let clipped_meshes = &self.info.egui_context.tessellate(shapes);

        let input = rend3_egui::Input {
            clipped_meshes,
            textures_delta,
            context: self.info.egui_context.clone(),
        };

        // Get a frame
        let frame = self.info.surface.get_current_texture().unwrap();

        // Swap the instruction buffers so that our frame's changes can be processed.
        self.info.rend3_renderer.swap_instruction_buffers();
        // Evaluate our frame's world-change instructions
        let mut eval_output = self.info.rend3_renderer.evaluate_instructions();

        // Build a rendergraph
        let mut graph = rend3::graph::RenderGraph::new();

        // Import the surface texture into the render graph.
        let frame_handle = graph.add_imported_render_target(
            &frame,
            0..1,
            rend3::graph::ViewportRect::from_size(self.info.resolution),
        );
        // Add the default rendergraph without a skybox
        let depth_target_handle = self.base_rendergraph.add_to_graph(
            &mut graph,
            &eval_output,
            &self.pbr_routine,
            None,
            &self.tonemapping_routine,
            frame_handle,
            self.info.resolution,
            rend3::types::SampleCount::One,
            glam::Vec4::ZERO,
            glam::Vec4::new(0.10, 0.05, 0.10, 1.0), // Nice scene-referred purple
        );

        self.grid_render_routine
            .add_to_graph(&mut graph, depth_target_handle, frame_handle);
        self.info
            .egui_routine
            .add_to_graph(&mut graph, input, frame_handle);

        // Dispatch a render using the built up rendergraph!
        graph.execute(&self.info.rend3_renderer, &mut eval_output);

        // Present the frame
        frame.present();

        None
    }

    fn handle_input_event(&mut self, input_state: &InputState, input_event: input::InputEvent) {
        match input_event {
            input::InputEvent::DoViewportOrbit => {
                self.cam.turntable_rotate(
                    &input_state.mouse.curr_cursor_pos
                        - input_state.mouse.cursor_pos_on_pressed.as_ref().unwrap(),
                    self.info.window_size.into(),
                );
                self.info
                    .rend3_renderer
                    .set_camera_data(self.cam.to_rend3_camera());
                log::trace!("(event) do viewport orbit");
            }

            input::InputEvent::FinishViewportOrbit => {
                self.cam.solidify_view_info();
                self.info
                    .rend3_renderer
                    .set_camera_data(self.cam.to_rend3_camera());
                log::trace!("(event) finish viewport orbit");
            }
        }
    }

    fn close_requested(&mut self) -> WindowCloseCallbackCommand {
        WindowCloseCallbackCommand::QuitProgram
    }
}
