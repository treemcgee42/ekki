use std::sync::Arc;

use grid::GridRenderRoutine;
use math::vector::Vector2;

mod base;
mod camera;
mod custom_renderer;
mod grid;
mod input;
mod math;
mod ui;

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
    // Setup logging
    ui::console::init(log::LevelFilter::Warn).unwrap();

    // Create event loop and window
    let event_loop = winit::event_loop::EventLoop::new();
    let window = {
        let mut builder = winit::window::WindowBuilder::new();
        builder = builder.with_title("rend3 cube");
        builder.build(&event_loop).expect("Could not build window")
    };

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
    let mut egui_routine = rend3_egui::EguiRenderRoutine::new(
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
    let mut platform = egui_winit::State::new(&event_loop);
    platform.set_pixels_per_point(window.scale_factor() as f32);

    // Create the shader preprocessor with all the default shaders added.
    let mut spp = rend3::ShaderPreProcessor::new();
    rend3_routine::builtin_shaders(&mut spp);

    // Create the base rendergraph.
    let base_rendergraph = base::BaseRenderGraph::new(&renderer, &spp);

    let mut data_core = renderer.data_core.lock();
    let pbr_routine = rend3_routine::pbr::PbrRoutine::new(
        &renderer,
        &mut data_core,
        &spp,
        &base_rendergraph.interfaces,
    );
    drop(data_core);
    let tonemapping_routine = rend3_routine::tonemapping::TonemappingRoutine::new(
        &renderer,
        &spp,
        &base_rendergraph.interfaces,
        preferred_format,
    );

    // Create mesh and calculate smooth normals based on vertices
    let mesh = create_mesh();

    // Add mesh to renderer's world.
    //
    // All handles are refcounted, so we only need to hang onto the handle until we
    // make an object.
    let mesh_handle = renderer.add_mesh(mesh);

    // Add PBR material with all defaults except a single color.
    let material = rend3_routine::pbr::PbrMaterial {
        albedo: rend3_routine::pbr::AlbedoComponent::Value(glam::Vec4::new(0.0, 0.5, 0.5, 1.0)),
        ..rend3_routine::pbr::PbrMaterial::default()
    };
    let material_handle = renderer.add_material(material);

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
    let _object_handle = renderer.add_object(object);

    let view_location = glam::Vec3::new(3.0, 3.0, -5.0);
    let view = glam::Mat4::from_euler(glam::EulerRot::XYZ, -0.55, 0.5, 0.0);
    let view = view * glam::Mat4::from_translation(-view_location);

    // Set camera's location
    let mut cam = camera::Camera::initialize(window_size.width as f32, window_size.height as f32);
    let camera = cam.to_rend3_camera();

    // rend3::types::Camera {
    //     projection: rend3::types::CameraProjection::Perspective {
    //         vfov: 60.0,
    //         near: 0.1,
    //     },
    //     view,
    // };
    // let camera_manager = rend3::managers::CameraManager::new(
    //     camera,
    //     renderer.handedness,
    //     Some(window_size.width as f32 / window_size.height as f32)
    // );
    renderer.set_camera_data(camera);

    let mut grid_render_routine = GridRenderRoutine::new(&renderer, preferred_format.clone());

    // Create a single directional light
    //
    // We need to keep the directional light handle alive.
    let _directional_handle = renderer.add_directional_light(rend3::types::DirectionalLight {
        color: glam::Vec3::ONE,
        intensity: 10.0,
        // Direction will be normalized
        direction: glam::Vec3::new(-1.0, -4.0, 2.0),
        distance: 400.0,
        resolution: 2048,
    });

    let mut resolution = glam::UVec2::new(window_size.width, window_size.height);

    let mut input_state = input::InputState::default();
    event_loop.run(move |event, _, control| {
        match event {
            winit::event::Event::WindowEvent { event, .. } => {
                // Pass the window events to the egui integration.
                if platform.on_event(&context, &event).consumed {
                    return;
                }

                match event {
                    // Close button was clicked, we should close.
                    winit::event::WindowEvent::CloseRequested => {
                        *control = winit::event_loop::ControlFlow::Exit;
                    }
                    // Window was resized, need to resize renderer.
                    winit::event::WindowEvent::Resized(physical_size) => {
                        resolution = glam::UVec2::new(physical_size.width, physical_size.height);
                        // Reconfigure the surface for the new size.
                        rend3::configure_surface(
                            &surface,
                            &renderer.device,
                            preferred_format,
                            glam::UVec2::new(resolution.x, resolution.y),
                            rend3::types::PresentMode::Fifo,
                        );
                        // Tell the renderer about the new aspect ratio.
                        renderer.set_aspect_ratio(resolution.x as f32 / resolution.y as f32);

                        cam.handle_window_resize(
                            physical_size.width as f32,
                            physical_size.height as f32,
                        );
                        renderer.set_camera_data(cam.to_rend3_camera());

                        egui_routine.resize(
                            physical_size.width,
                            physical_size.height,
                            window.scale_factor() as f32,
                        );
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
                window.request_redraw();
            }

            // Render!
            winit::event::Event::RedrawRequested(..) => {
                // UI
                context.begin_frame(platform.take_egui_input(&window));

                egui::Window::new("Change color")
                    .resizable(true)
                    .show(&context, |ui| {
                        ui.label("Change the color of the cube");
                    });

                egui::Window::new("Console")
                    .resizable(true)
                    .show(&context, |ui| {
                        ui::console::draw_egui_console_menu(ui);
                        ui::console::draw_egui_logging_lines(ui);
                    });

                let egui::FullOutput {
                    shapes,
                    textures_delta,
                    ..
                } = context.end_frame();

                let clipped_meshes = &context.tessellate(shapes);

                let input = rend3_egui::Input {
                    clipped_meshes,
                    textures_delta,
                    context: context.clone(),
                };

                // Get a frame
                let frame = surface.get_current_texture().unwrap();

                // Swap the instruction buffers so that our frame's changes can be processed.
                renderer.swap_instruction_buffers();
                // Evaluate our frame's world-change instructions
                let mut eval_output = renderer.evaluate_instructions();

                // Build a rendergraph
                let mut graph = rend3::graph::RenderGraph::new();

                // Import the surface texture into the render graph.
                let frame_handle = graph.add_imported_render_target(
                    &frame,
                    0..1,
                    rend3::graph::ViewportRect::from_size(resolution),
                );
                // Add the default rendergraph without a skybox
                let depth_target_handle = base_rendergraph.add_to_graph(
                    &mut graph,
                    &eval_output,
                    &pbr_routine,
                    None,
                    &tonemapping_routine,
                    frame_handle,
                    resolution,
                    rend3::types::SampleCount::One,
                    glam::Vec4::ZERO,
                    glam::Vec4::new(0.10, 0.05, 0.10, 1.0), // Nice scene-referred purple
                );

                grid_render_routine.add_to_graph(&mut graph, depth_target_handle, frame_handle);
                egui_routine.add_to_graph(&mut graph, input, frame_handle);

                // Dispatch a render using the built up rendergraph!
                graph.execute(&renderer, &mut eval_output);

                // Present the frame
                frame.present();

                control.set_poll(); // default behavior
            }

            // Other events we don't care about
            _ => {}
        }

        for input_event in input_state.get_input_events() {
            match input_event {
                input::InputEvent::DoViewportOrbit => {
                    cam.turntable_rotate(
                        &input_state.mouse.curr_cursor_pos
                            - input_state.mouse.cursor_pos_on_pressed.as_ref().unwrap(),
                        window_size.into(),
                    );
                    renderer.set_camera_data(cam.to_rend3_camera());
                    log::trace!("(event) do viewport orbit");
                }

                input::InputEvent::FinishViewportOrbit => {
                    cam.solidify_view_info();
                    renderer.set_camera_data(cam.to_rend3_camera());
                    log::trace!("(event) finish viewport orbit");
                }
            }
        }
        input_state.reset_release_events();
    });
}
