use crate::{config::RenderUserConfig, plugins::RendererPlugin};

use super::*;

pub struct RenderWindow {
    info: WindowInfo,
    texture: RenderImage,
    renderer_plugin: Option<RendererPlugin>,
    render_settings_active: bool,
    renderer_path: String,
    render_in_progress: bool,
    should_begin_render: bool,
    should_transfer_render_data: bool,
    render_preview_update_requested: bool,
    time_of_last_render_preview_update: f64,
    preview_update_frequency: u32,
    reload_renderer: bool,
}

impl RenderWindow {
    pub fn create<T>(
        window_target: &winit::event_loop::EventLoopWindowTarget<T>,
        user_config: &Option<RenderUserConfig>,
    ) -> Self
    where
        T: 'static,
    {
        let init_info = WindowInfoInitializeInfo {
            title: "render view".to_string(),
            ..Default::default()
        };
        let info = WindowInfo::initialize(window_target, init_info);

        let renderer_path = user_config
            .as_ref()
            .and_then(|conf| conf.renderer_path.clone())
            .unwrap_or(String::new());
        let preview_update_frequency = user_config
            .as_ref()
            .and_then(|conf| conf.update_frequency)
            .unwrap_or(2);

        Self {
            info,
            texture: RenderImage::default(),
            renderer_plugin: None,
            render_settings_active: false,
            renderer_path,
            render_in_progress: false,
            should_begin_render: false,
            should_transfer_render_data: true,
            render_preview_update_requested: false,
            time_of_last_render_preview_update: f64::NEG_INFINITY,
            preview_update_frequency,
            reload_renderer: false,
        }
    }
}

struct RenderImage {
    texture: Option<egui::TextureHandle>,
    temp_texture: Option<egui::TextureHandle>,
}

impl Default for RenderImage {
    fn default() -> Self {
        Self {
            texture: None,
            temp_texture: None,
        }
    }
}

impl RenderImage {
    fn ui(&mut self, ui: &mut egui::Ui) {
        if let Some(texture) = &self.texture {
            let width: f32;
            let height: f32;

            let texture_width = texture.size()[0] as f32;
            let texture_height = texture.size()[1] as f32;
            let texture_aspect_ratio = texture_width / texture_height;

            // Determine a size to display that won't distort the image.
            // TODO: handle edge cases.
            if texture_width < ui.available_width() && texture_height < ui.available_height() {
                width = texture_width;
                height = texture_height;
            } else if texture_width >= ui.available_width() {
                width = ui.available_width();
                height = width / texture_aspect_ratio;
            } else {
                height = ui.available_height();
                width = height * texture_aspect_ratio;
            }

            ui.image(texture, egui::Vec2::new(width, height));
            return;
        }

        let texture: &egui::TextureHandle = self.temp_texture.get_or_insert_with(|| {
            // Load the texture only once.
            ui.ctx()
                .load_texture("my-image", egui::ColorImage::example(), Default::default())
        });

        ui.image(texture, ui.available_size());
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
        if self.reload_renderer && self.renderer_plugin.is_some() {
            if let Some(plug) = &mut self.renderer_plugin {
                plug.reload().unwrap();
                self.reload_renderer = false;
                self.should_begin_render = true;
            }
        }

        if self.should_begin_render {
            self.should_begin_render = false;

            let renderer_plugin =
                RendererPlugin::load_plugin(std::ffi::OsStr::new(&self.renderer_path), 512, 512);
            if renderer_plugin.is_err() {
                log::error!("failed to load renderer plugin");
            } else {
                self.renderer_plugin = Some(renderer_plugin.unwrap());
                self.renderer_plugin
                    .as_mut()
                    .unwrap()
                    .begin_incremental_render();

                self.render_in_progress = true;
            }
        }

        if let Some(plug) = &mut self.renderer_plugin {
            if self.render_in_progress {
                if self.info.egui_context.input(|i| i.time)
                    - self.time_of_last_render_preview_update
                    > (self.preview_update_frequency as f64)
                    && !self.render_preview_update_requested
                {
                    plug.request_read();
                    self.render_preview_update_requested = true;
                }

                if self.render_preview_update_requested {
                    if plug.poll_read_request() {
                        self.should_transfer_render_data = true;
                    }
                }

                if self.should_transfer_render_data || plug.render_is_finished() {
                    let egui_color_image = plug.convert_rgb_data_to_egui_image();
                    self.texture.texture = Some(self.info.egui_context.load_texture(
                        "render",
                        egui_color_image,
                        Default::default(),
                    ));

                    // Reset flags.
                    self.should_transfer_render_data = false;

                    if self.render_preview_update_requested {
                        self.time_of_last_render_preview_update =
                            self.info.egui_context.input(|i| i.time);
                        self.render_preview_update_requested = false;
                    }

                    if plug.render_is_finished() {
                        plug.join_thread();
                        self.render_preview_update_requested = false;
                        self.render_in_progress = false;
                    }
                }
            }
        }

        let render_progress = self
            .renderer_plugin
            .as_ref()
            .map(|plug| plug.get_render_progress())
            .unwrap_or(0.)
            .clamp(0., 1.);

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

                    if ui.button("Reload").clicked() {
                        self.reload_renderer = true;
                    }
                });
            });
        });

        egui::TopBottomPanel::bottom("render_info").show(&self.info.egui_context, |ui| {
            ui.add(egui::ProgressBar::new(render_progress).show_percentage());
        });

        egui::CentralPanel::default().show(&self.info.egui_context, |ui| {
            ui.centered_and_justified(|ui| {
                self.texture.ui(ui);
            })
        });

        draw_render_settings_window(
            &self.info.egui_context,
            &mut self.render_settings_active,
            &mut self.renderer_path,
            &mut self.preview_update_frequency,
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
    preview_update_frequency: &mut u32,
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
                    ui.end_row();

                    ui.label("Preview frequency");
                    ui.add(egui::Slider::new(preview_update_frequency, 1..=10));
                });
        });
}
