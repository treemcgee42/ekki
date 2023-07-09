use super::*;

pub struct StartupWindow {
    info: WindowInfo,
    texture: MyImage,
}

impl StartupWindow {
    pub fn create<T>(window_target: &winit::event_loop::EventLoopWindowTarget<T>) -> Self
    where
        T: 'static,
    {
        // Center window, specify size
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

        let window_init_info = WindowInfoInitializeInfo {
            title: "ekki".to_string(),
            inner_size: Some(target_window_size),
            with_decorations: false,
            with_position: Some(target_window_position),
        };
        let window_info = WindowInfo::initialize(window_target, window_init_info);

        let texture = MyImage::default();

        Self {
            info: window_info,
            texture,
        }
    }
}

impl WindowLike for StartupWindow {
    fn get_window_id(&self) -> winit::window::WindowId {
        self.info.window_id
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
        self.info.resize_default(physical_size);
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

                        if columns[0]
                            .add(
                                egui::Label::new(egui::RichText::new("👾 Render").size(18.))
                                    .sense(egui::Sense::click()),
                            )
                            .clicked()
                        {
                            callbacks.push(WindowRedrawCallbackCommand::CreateRenderWindowAndClose);
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
            rend3::graph::ViewportRect::from_size(self.info.resolution),
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
