use crate::plugins::RendererPlugin;

use super::*;

pub struct RenderWindow {
    info: WindowInfo,
    texture: MyImage,
    renderer_plugin: Option<RendererPlugin>,
    render_settings_active: bool,
    renderer_path: String,
    should_begin_render: bool,
}

impl RenderWindow {
    pub fn create<T>(window_target: &winit::event_loop::EventLoopWindowTarget<T>) -> Self
    where
        T: 'static,
    {
        let init_info = WindowInfoInitializeInfo {
            title: "render view".to_string(),
            ..Default::default()
        };
        let info = WindowInfo::initialize(window_target, init_info);

        let texture = MyImage { texture: None };

        Self {
            info,
            texture,
            renderer_plugin: None,
            render_settings_active: false,
            renderer_path: String::new(),
            should_begin_render: false,
        }
    }
}

impl WindowLike for RenderWindow {
    fn get_window_id(&self) -> winit::window::WindowId {
        self.info.window_id
    }

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
        if self.should_begin_render {
            self.should_begin_render = false;

            let renderer_plugin =
                RendererPlugin::load_plugin(std::ffi::OsStr::new(&self.renderer_path), 400, 400);
            if renderer_plugin.is_err() {
                log::error!("failed to load renderer plugin");
            } else {
                self.renderer_plugin = Some(renderer_plugin.unwrap());

                self.renderer_plugin
                    .as_mut()
                    .unwrap()
                    .begin_incremental_render();
            }
        }

        // UI
        self.info.egui_context.begin_frame(
            self.info
                .egui_winit_state
                .take_egui_input(&self.info.raw_window),
        );

        egui::TopBottomPanel::top("my_panel").show(&self.info.egui_context, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| if ui.button("Save as").clicked() {});

                ui.menu_button("Render", |ui| {
                    if ui.button("Settings").clicked() {
                        self.render_settings_active = true;
                    }

                    if ui.button("Render").clicked() {
                        if self.renderer_plugin.is_none() {
                            self.should_begin_render = true;
                        }
                    }
                });
            });
        });

        egui::CentralPanel::default().show(&self.info.egui_context, |ui| {
            self.texture.ui(ui);
        });

        draw_render_settings_window(
            &self.info.egui_context,
            &mut self.render_settings_active,
            &mut self.renderer_path,
        );

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

        None
    }

    fn handle_input_event(&mut self, _input_state: &InputState, input_event: input::InputEvent) {
        match input_event {
            input::InputEvent::DoViewportOrbit => {}
            input::InputEvent::FinishViewportOrbit => {}
        }
    }
}

fn draw_render_settings_window(
    ctx: &egui::Context,
    is_active: &mut bool,
    renderer_path: &mut String,
) {
    egui::Window::new("Render settings")
        .open(is_active)
        .resizable(true)
        .show(ctx, |ui| {
            egui::Grid::new("render_settings_grid")
                .num_columns(2)
                .spacing([40.0, 4.0])
                .striped(true)
                .show(ui, |ui| {
                    ui.label("Path");
                    ui.horizontal(|ui| {
                        ui.add(egui::TextEdit::singleline(renderer_path).hint_text("No path set"));
                        if ui.button("Open").clicked() {
                            if let Some(path) = rfd::FileDialog::new().pick_file() {
                                *renderer_path = path.display().to_string();
                            }
                        };
                    });
                });
        });
}
