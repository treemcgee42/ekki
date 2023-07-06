use super::*;

pub struct RenderWindow {
    window_id: winit::window::WindowId,
    window_info: WindowInfo,
    texture: MyImage,
    resolution: glam::UVec2,
}

impl RenderWindow {
    pub fn create<T>(window_target: &winit::event_loop::EventLoopWindowTarget<T>) -> Self
    where
        T: 'static,
    {
        let window = {
            let mut builder = winit::window::WindowBuilder::new();
            builder = builder.with_title("render");
            builder
                .build(window_target)
                .expect("Could not build window")
        };
        let window_id = window.id();
        let window_size = window.inner_size();

        // Create the Instance, Adapter, and Device. We can specify preferred backend,
        // device name, or rendering profile. In this case we let rend3 choose for us.
        let iad = pollster::block_on(rend3::create_iad(None, None, None, None)).unwrap();

        // The one line of unsafe needed. We just need to guarentee that the window
        // outlives the use of the surface.
        //
        // SAFETY: this surface _must_ not be used after the `window` dies. Both the
        // event loop and the renderer are owned by the `run` closure passed to winit,
        // so rendering work will stop after the window dies.
        let surface = Arc::new(unsafe { iad.instance.create_surface(&window) }.unwrap());
        // Get the preferred format for the surface.
        let caps = surface.get_capabilities(&iad.adapter);
        let preferred_format = caps.formats[0];

        // Configure the surface to be ready for rendering.
        rend3::configure_surface(
            &surface,
            &iad.device,
            preferred_format,
            glam::UVec2::new(window_size.width, window_size.height),
            rend3::types::PresentMode::Fifo,
        );

        // Make us a renderer.
        let renderer = rend3::Renderer::new(
            iad,
            rend3::types::Handedness::Left,
            Some(window_size.width as f32 / window_size.height as f32),
        )
        .unwrap();

        // Create the egui render routine
        let egui_routine = rend3_egui::EguiRenderRoutine::new(
            &renderer,
            preferred_format,
            rend3::types::SampleCount::One,
            window_size.width,
            window_size.height,
            window.scale_factor() as f32,
        );

        // Create the egui context
        let context = egui::Context::default();

        // Create the winit/egui integration.
        //let mut platform = egui_winit::State::new_with_wayland_display(None);
        let mut platform = egui_winit::State::new(window_target);
        platform.set_pixels_per_point(window.scale_factor() as f32);

        let window_info = WindowInfo {
            raw_window: window,
            window_size,
            surface,
            preferred_texture_format: preferred_format,
            egui_routine,
            egui_context: context,
            egui_winit_state: platform,
            rend3_renderer: renderer,
        };

        let texture = MyImage { texture: None };

        let resolution = glam::UVec2::new(
            window_info.window_size.width,
            window_info.window_size.height,
        );

        Self {
            window_id,
            window_info,
            texture,
            resolution,
        }
    }
}

impl WindowLike for RenderWindow {
    fn get_window_id(&self) -> winit::window::WindowId {
        self.window_id
    }

    fn request_redraw(&self) {
        self.window_info.raw_window.request_redraw();
    }

    fn egui_event_consumed(&mut self, event: &winit::event::WindowEvent) -> bool {
        self.window_info
            .egui_winit_state
            .on_event(&self.window_info.egui_context, event)
            .consumed
    }

    fn resize(&mut self, physical_size: winit::dpi::PhysicalSize<u32>) {
        self.resolution = glam::UVec2::new(physical_size.width, physical_size.height);
        // Reconfigure the surface for the new size.
        rend3::configure_surface(
            &self.window_info.surface,
            &self.window_info.rend3_renderer.device,
            self.window_info.preferred_texture_format,
            glam::UVec2::new(self.resolution.x, self.resolution.y),
            rend3::types::PresentMode::Fifo,
        );
        // Tell the renderer about the new aspect ratio.
        self.window_info
            .rend3_renderer
            .set_aspect_ratio(self.resolution.x as f32 / self.resolution.y as f32);

        self.window_info.egui_routine.resize(
            physical_size.width,
            physical_size.height,
            self.window_info.raw_window.scale_factor() as f32,
        );
    }

    fn redraw(&mut self) -> Option<Vec<WindowRedrawCallbackCommand>> {
        // UI
        self.window_info.egui_context.begin_frame(
            self.window_info
                .egui_winit_state
                .take_egui_input(&self.window_info.raw_window),
        );

        egui::TopBottomPanel::top("my_panel").show(&self.window_info.egui_context, |ui| {
            ui.label("Hello World! From `TopBottomPanel`, that must be before `CentralPanel`!");
        });
        egui::CentralPanel::default().show(&self.window_info.egui_context, |ui| {
            self.texture.ui(ui);
        });

        let egui::FullOutput {
            shapes,
            textures_delta,
            ..
        } = self.window_info.egui_context.end_frame();

        let clipped_meshes = &self.window_info.egui_context.tessellate(shapes);

        let input = rend3_egui::Input {
            clipped_meshes,
            textures_delta,
            context: self.window_info.egui_context.clone(),
        };

        // Get a frame
        let frame = self.window_info.surface.get_current_texture().unwrap();

        // Swap the instruction buffers so that our frame's changes can be processed.
        self.window_info.rend3_renderer.swap_instruction_buffers();
        // Evaluate our frame's world-change instructions
        let mut eval_output = self.window_info.rend3_renderer.evaluate_instructions();

        // Build a rendergraph
        let mut graph = rend3::graph::RenderGraph::new();

        // Import the surface texture into the render graph.
        let frame_handle = graph.add_imported_render_target(
            &frame,
            0..1,
            rend3::graph::ViewportRect::from_size(self.resolution),
        );

        self.window_info
            .egui_routine
            .add_to_graph(&mut graph, input, frame_handle);

        // Dispatch a render using the built up rendergraph!
        graph.execute(&self.window_info.rend3_renderer, &mut eval_output);

        // Present the frame
        frame.present();

        None
    }

    fn handle_input_event(&mut self, _input_state: &InputState, input_event: input::InputEvent) {
        match input_event {
            input::InputEvent::DoViewportOrbit => {}
            input::InputEvent::FinishViewportOrbit => {}
        }
    }
}
