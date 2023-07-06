use super::*;

pub struct StartupWindow {
    id: winit::window::WindowId,
    info: WindowInfo,
    resolution: glam::UVec2,
    texture: MyImage,
}

impl StartupWindow {
    pub fn create<T>(window_target: &winit::event_loop::EventLoopWindowTarget<T>) -> Self
    where
        T: 'static,
    {
        let (target_window_size, target_window_position) = {
            let monitor_size = window_target.primary_monitor().unwrap().size();

            if monitor_size.width == 0 || monitor_size.height == 0 {
                (
                    winit::dpi::PhysicalSize {
                        width: 600,
                        height: 500,
                    },
                    winit::dpi::PhysicalPosition { x: 100, y: 100 },
                )
            } else {
                let height_percentage = 0.4;
                let aspect_ratio = (16. / 14.) * (0.5 / 0.4);

                let height = height_percentage * (monitor_size.height as f32);
                let width = aspect_ratio * height;

                let center_x = 0.5 * (monitor_size.width as f32);
                let center_y = 0.5 * (monitor_size.height as f32);

                let tl_x = center_x - (0.5 * width);
                let tl_y = center_y - (0.5 * height);

                (
                    winit::dpi::PhysicalSize {
                        width: width as u32,
                        height: height as u32,
                    },
                    winit::dpi::PhysicalPosition {
                        x: tl_x as u32,
                        y: tl_y as u32,
                    },
                )
            }
        };

        let window = {
            let builder = winit::window::WindowBuilder::new();
            builder
                .with_title("ekki")
                .with_inner_size(target_window_size)
                .with_decorations(false)
                .with_position(target_window_position)
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
        // context.set_pixels_per_point(window.scale_factor() as f32);

        // Create the winit/egui integration.
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

        let resolution = glam::UVec2::new(
            window_info.window_size.width,
            window_info.window_size.height,
        );

        let texture = MyImage { texture: None };

        Self {
            id: window_id,
            info: window_info,
            resolution,
            texture,
        }
    }
}

impl WindowLike for StartupWindow {
    fn get_window_id(&self) -> winit::window::WindowId {
        self.id
    }

    fn handle_input_event(&mut self, _input_state: &InputState, _input_event: input::InputEvent) {}

    fn request_redraw(&self) {
        self.info.raw_window.request_redraw();
    }

    fn egui_event_consumed(&mut self, event: &winit::event::WindowEvent) -> bool {
        self.info
            .egui_winit_state
            .on_event(&self.info.egui_context, event)
            .consumed
    }

    fn resize(&mut self, physical_size: winit::dpi::PhysicalSize<u32>) {
        self.resolution = glam::UVec2::new(physical_size.width, physical_size.height);
        // Reconfigure the surface for the new size.
        rend3::configure_surface(
            &self.info.surface,
            &self.info.rend3_renderer.device,
            self.info.preferred_texture_format,
            glam::UVec2::new(self.resolution.x, self.resolution.y),
            rend3::types::PresentMode::Fifo,
        );
        // Tell the renderer about the new aspect ratio.
        self.info
            .rend3_renderer
            .set_aspect_ratio(self.resolution.x as f32 / self.resolution.y as f32);

        self.info.egui_routine.resize(
            physical_size.width,
            physical_size.height,
            self.info.raw_window.scale_factor() as f32,
        );
    }

    fn redraw(&mut self) -> Option<Vec<WindowRedrawCallbackCommand>> {
        let mut callbacks = Vec::new();

        // UI
        self.info.egui_context.begin_frame(
            self.info
                .egui_winit_state
                .take_egui_input(&self.info.raw_window),
        );

        let half_height = 0.6 * self.info.egui_context.available_rect().height();
        egui::TopBottomPanel::top("startup_picture")
            .exact_height(half_height)
            .frame(egui::Frame::none())
            .show(&self.info.egui_context, |ui| {
                self.texture.ui(ui);
            });

        egui::CentralPanel::default().show(&self.info.egui_context, |ui| {
            let padding_amount = {
                let available = ui.available_width();
                let padding_percent = 0.02;
                available * padding_percent
            };

            ui.vertical(|ui| {
                ui.add_space(padding_amount);

                ui.horizontal_centered(|ui| {
                    ui.add_space(padding_amount);

                    ui.columns(2, |columns| {
                        columns[0].heading("New file");
                        columns[0].add_space(10.);

                        if columns[0]
                            .add(
                                egui::Label::new(egui::RichText::new("🎲 3D scene").size(18.))
                                    .sense(egui::Sense::click()),
                            )
                            .clicked()
                        {
                            callbacks.push(WindowRedrawCallbackCommand::Create3DWindowAndClose);
                        }

                        if columns[0]
                            .add(
                                egui::Label::new(egui::RichText::new("☔ Node map").size(18.))
                                    .sense(egui::Sense::click()),
                            )
                            .clicked()
                        {
                            callbacks
                                .push(WindowRedrawCallbackCommand::CreateNodeMapWindowAndClose);
                        }
                        columns[0].label(egui::RichText::new("👾 2D scene").size(18.));
                        columns[0].label(egui::RichText::new("🎩 Plugin editor").size(18.));
                        columns[0].label(egui::RichText::new("🌺 Hibiscus").size(18.));

                        columns[1].heading("Recent files");
                        columns[1].add_space(10.);

                        columns[1].label(egui::RichText::new("> File 1").size(18.));
                        columns[1].label(egui::RichText::new("> File 2").size(18.));
                        columns[1].label(egui::RichText::new("> File 3").size(18.));
                        columns[1].label(egui::RichText::new("> File 4").size(18.));
                        columns[1].label(egui::RichText::new("> File 5").size(18.));
                    });

                    ui.add_space(padding_amount);
                });

                ui.add_space(padding_amount);
            });
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
            rend3::graph::ViewportRect::from_size(self.resolution),
        );

        self.info
            .egui_routine
            .add_to_graph(&mut graph, input, frame_handle);

        // Dispatch a render using the built up rendergraph!
        graph.execute(&self.info.rend3_renderer, &mut eval_output);

        // Present the frame
        frame.present();

        if callbacks.is_empty() {
            None
        } else {
            Some(callbacks)
        }
    }
}
