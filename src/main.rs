use std::collections::HashMap;

use math::vector::Vector2;
use window::WindowLike;

mod base;
mod camera;
mod custom_renderer;
mod grid;
mod input;
mod math;
mod ui;
mod window;

struct MyImage {
    texture: Option<egui::TextureHandle>,
}

impl MyImage {
    fn ui(&mut self, ui: &mut egui::Ui) {
        let texture: &egui::TextureHandle = self.texture.get_or_insert_with(|| {
            // Load the texture only once.
            ui.ctx()
                .load_texture("my-image", egui::ColorImage::example(), Default::default())
        });

        // Shorter version:
        // ui.image(texture, texture.size_vec2());
        ui.image(texture, ui.available_size());
    }
}

fn vertex(pos: [f32; 3]) -> glam::Vec3 {
    glam::Vec3::from(pos)
}

fn create_mesh() -> rend3::types::Mesh {
    let vertex_positions = [
        // far side (0.0, 0.0, 1.0)
        vertex([-1.0, -1.0, 1.0]),
        vertex([1.0, -1.0, 1.0]),
        vertex([1.0, 1.0, 1.0]),
        vertex([-1.0, 1.0, 1.0]),
        // near side (0.0, 0.0, -1.0)
        vertex([-1.0, 1.0, -1.0]),
        vertex([1.0, 1.0, -1.0]),
        vertex([1.0, -1.0, -1.0]),
        vertex([-1.0, -1.0, -1.0]),
        // right side (1.0, 0.0, 0.0)
        vertex([1.0, -1.0, -1.0]),
        vertex([1.0, 1.0, -1.0]),
        vertex([1.0, 1.0, 1.0]),
        vertex([1.0, -1.0, 1.0]),
        // left side (-1.0, 0.0, 0.0)
        vertex([-1.0, -1.0, 1.0]),
        vertex([-1.0, 1.0, 1.0]),
        vertex([-1.0, 1.0, -1.0]),
        vertex([-1.0, -1.0, -1.0]),
        // top (0.0, 1.0, 0.0)
        vertex([1.0, 1.0, -1.0]),
        vertex([-1.0, 1.0, -1.0]),
        vertex([-1.0, 1.0, 1.0]),
        vertex([1.0, 1.0, 1.0]),
        // bottom (0.0, -1.0, 0.0)
        vertex([1.0, -1.0, 1.0]),
        vertex([-1.0, -1.0, 1.0]),
        vertex([-1.0, -1.0, -1.0]),
        vertex([1.0, -1.0, -1.0]),
    ];

    let index_data: &[u32] = &[
        0, 1, 2, 2, 3, 0, // far
        4, 5, 6, 6, 7, 4, // near
        8, 9, 10, 10, 11, 8, // right
        12, 13, 14, 14, 15, 12, // left
        16, 17, 18, 18, 19, 16, // top
        20, 21, 22, 22, 23, 20, // bottom
    ];

    rend3::types::MeshBuilder::new(vertex_positions.to_vec(), rend3::types::Handedness::Left)
        .with_indices(index_data.to_vec())
        .build()
        .unwrap()
}

fn main() {
    // State
    let mut render_window_active = false;

    // Setup logging
    ui::console::init(log::LevelFilter::Warn).unwrap();

    // Create event loop and window
    let event_loop = winit::event_loop::EventLoop::new();

    let mut windows: HashMap<winit::window::WindowId, Box<dyn window::WindowLike>> = HashMap::new();
    let main_window = {
        let window = window::SceneViewer3D::create(&event_loop);
        let window_id = window.get_window_id();
        windows.insert(window.get_window_id(), Box::new(window));

        windows.get_mut(&window_id).unwrap()
    };
    let main_window_id = main_window.get_window_id();

    let mut input_state = input::InputState::default();
    event_loop.run(move |event, window_target, control| {
        match event {
            winit::event::Event::WindowEvent { window_id, event } => {
                let this_window = windows.get_mut(&window_id).unwrap();

                // Pass the window events to the egui integration.
                if this_window.egui_event_consumed(&event) {
                    return;
                }

                match event {
                    // Close button was clicked, we should close.
                    winit::event::WindowEvent::CloseRequested => {
                        *control = winit::event_loop::ControlFlow::Exit;
                    }
                    // Window was resized, need to resize renderer.
                    winit::event::WindowEvent::Resized(physical_size) => {
                        this_window.resize(physical_size);
                    }

                    winit::event::WindowEvent::KeyboardInput {
                        device_id: _,
                        input,
                        is_synthetic: _,
                    } => {
                        let state = input.state;
                        let keycode = input.virtual_keycode;

                        if keycode == Some(winit::event::VirtualKeyCode::LShift) {
                            match state {
                                winit::event::ElementState::Pressed => {
                                    input_state.keyboard.shift_pressed = true;
                                }

                                winit::event::ElementState::Released => {
                                    input_state.keyboard.shift_pressed = false;
                                    input_state.keyboard.shift_released = true;
                                }
                            }
                        }

                        if keycode == Some(winit::event::VirtualKeyCode::R) && !render_window_active
                        {
                            let new_window = window::RenderWindow::create(window_target);
                            windows.insert(new_window.get_window_id(), Box::new(new_window));
                            render_window_active = true;
                        }
                    }

                    winit::event::WindowEvent::MouseInput {
                        device_id: _,
                        state,
                        button,
                        modifiers: _,
                    } => {
                        if button == winit::event::MouseButton::Left {
                            match state {
                                winit::event::ElementState::Pressed => {
                                    input_state.mouse.lmb_pressed = true;
                                    if input_state.mouse.cursor_pos_on_pressed.is_none() {
                                        input_state.mouse.cursor_pos_on_pressed =
                                            Some(input_state.mouse.curr_cursor_pos.clone());
                                    }
                                }
                                winit::event::ElementState::Released => {
                                    input_state.mouse.lmb_pressed = false;
                                    input_state.mouse.lmb_released = true;
                                    input_state.mouse.cursor_pos_on_pressed = None;
                                }
                            }
                        }
                    }

                    _ => {}
                }
            }

            winit::event::Event::DeviceEvent {
                device_id: _,
                event,
            } => match event {
                winit::event::DeviceEvent::MouseMotion { delta } => {
                    input_state.mouse.curr_cursor_pos +=
                        Vector2::new(-delta.0 as f32, -delta.1 as f32);
                }

                _ => {}
            },

            winit::event::Event::MainEventsCleared => {
                for w in windows.values_mut() {
                    w.request_redraw();
                }
            }

            // Render!
            winit::event::Event::RedrawRequested(window_id) => {
                let w = windows.get_mut(&window_id).unwrap();
                w.redraw();

                control.set_poll(); // default behavior
            }

            // Other events we don't care about
            _ => {}
        }

        let w = windows.get_mut(&main_window_id).unwrap();
        for input_event in input_state.get_input_events() {
            w.handle_input_event(&input_state, input_event)
        }
        input_state.reset_release_events();
    });
}
