use std::{collections::HashMap, sync::Arc};

use grid::GridRenderRoutine;
use math::vector::Vector2;

mod base;
mod camera;
mod custom_renderer;
mod grid;
mod input;
mod math;
mod ui;

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

struct WindowInfo {
    raw_window: winit::window::Window,
    window_size: winit::dpi::PhysicalSize<u32>,
    surface: Arc<wgpu::Surface>,
    preferred_texture_format: wgpu::TextureFormat,
    egui_routine: rend3_egui::EguiRenderRoutine,
    egui_context: egui::Context,
    egui_winit_state: egui_winit::State,
    rend3_renderer: Arc<rend3::Renderer>,
}

fn create_3d_scene_window(
    event_loop: &winit::event_loop::EventLoop<()>,
    iad: rend3::InstanceAdapterDevice,
) -> (winit::window::WindowId, WindowInfo) {
    let window = {
        let mut builder = winit::window::WindowBuilder::new();
        builder = builder.with_title("rend3 cube");
        builder.build(event_loop).expect("Could not build window")
    };

    let window_size = window.inner_size();

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
    let mut platform = egui_winit::State::new(&event_loop);
    platform.set_pixels_per_point(window.scale_factor() as f32);

    let window_id = window.id();
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

    (window_id, window_info)
}

fn create_render_window<T>(
    window_target: &winit::event_loop::EventLoopWindowTarget<T>,
) -> (winit::window::WindowId, WindowInfo)
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

    (window_id, window_info)
}

fn main() {
    // State
    let mut render_window_active = false;
    let mut render_texture = MyImage { texture: None };

    // Setup logging
    ui::console::init(log::LevelFilter::Warn).unwrap();

    // Create event loop and window
    let event_loop = winit::event_loop::EventLoop::new();

    // Create the Instance, Adapter, and Device. We can specify preferred backend,
    // device name, or rendering profile. In this case we let rend3 choose for us.
    let iad = pollster::block_on(rend3::create_iad(None, None, None, None)).unwrap();

    let mut windows = HashMap::new();
    let main_window = {
        let window = create_3d_scene_window(&event_loop, iad.clone());
        let window_id = window.0;
        windows.insert(window.0, window.1);

        windows.get_mut(&window_id).unwrap()
    };
    let main_window_id = main_window.raw_window.id();

    // Create the shader preprocessor with all the default shaders added.
    let mut spp = rend3::ShaderPreProcessor::new();
    rend3_routine::builtin_shaders(&mut spp);

    // Create the base rendergraph.
    let base_rendergraph = base::BaseRenderGraph::new(&main_window.rend3_renderer, &spp);

    let mut data_core = main_window.rend3_renderer.data_core.lock();
    let pbr_routine = rend3_routine::pbr::PbrRoutine::new(
        &main_window.rend3_renderer,
        &mut data_core,
        &spp,
        &base_rendergraph.interfaces,
    );
    drop(data_core);
    let tonemapping_routine = rend3_routine::tonemapping::TonemappingRoutine::new(
        &main_window.rend3_renderer,
        &spp,
        &base_rendergraph.interfaces,
        main_window.preferred_texture_format,
    );

    // Create mesh and calculate smooth normals based on vertices
    let mesh = create_mesh();

    // Add mesh to renderer's world.
    //
    // All handles are refcounted, so we only need to hang onto the handle until we
    // make an object.
    let mesh_handle = main_window.rend3_renderer.add_mesh(mesh);

    // Add PBR material with all defaults except a single color.
    let material = rend3_routine::pbr::PbrMaterial {
        albedo: rend3_routine::pbr::AlbedoComponent::Value(glam::Vec4::new(0.0, 0.5, 0.5, 1.0)),
        ..rend3_routine::pbr::PbrMaterial::default()
    };
    let material_handle = main_window.rend3_renderer.add_material(material);

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
    let _object_handle = main_window.rend3_renderer.add_object(object);

    let view_location = glam::Vec3::new(3.0, 3.0, -5.0);
    let view = glam::Mat4::from_euler(glam::EulerRot::XYZ, -0.55, 0.5, 0.0);
    let view = view * glam::Mat4::from_translation(-view_location);

    // Set camera's location
    let mut cam = camera::Camera::initialize(
        main_window.window_size.width as f32,
        main_window.window_size.height as f32,
    );
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
    main_window.rend3_renderer.set_camera_data(camera);

    let mut grid_render_routine = GridRenderRoutine::new(
        &main_window.rend3_renderer,
        main_window.preferred_texture_format.clone(),
    );

    // Create a single directional light
    //
    // We need to keep the directional light handle alive.
    let _directional_handle =
        main_window
            .rend3_renderer
            .add_directional_light(rend3::types::DirectionalLight {
                color: glam::Vec3::ONE,
                intensity: 10.0,
                // Direction will be normalized
                direction: glam::Vec3::new(-1.0, -4.0, 2.0),
                distance: 400.0,
                resolution: 2048,
            });

    let mut resolution = glam::UVec2::new(
        main_window.window_size.width,
        main_window.window_size.height,
    );

    let mut input_state = input::InputState::default();
    event_loop.run(move |event, window_target, control| {
        match event {
            winit::event::Event::WindowEvent { window_id, event } => {
                let this_window = windows.get_mut(&window_id).unwrap();

                // Pass the window events to the egui integration.
                if this_window
                    .egui_winit_state
                    .on_event(&this_window.egui_context, &event)
                    .consumed
                {
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
                            &this_window.surface,
                            &this_window.rend3_renderer.device,
                            this_window.preferred_texture_format,
                            glam::UVec2::new(resolution.x, resolution.y),
                            rend3::types::PresentMode::Fifo,
                        );
                        // Tell the renderer about the new aspect ratio.
                        this_window
                            .rend3_renderer
                            .set_aspect_ratio(resolution.x as f32 / resolution.y as f32);

                        cam.handle_window_resize(
                            physical_size.width as f32,
                            physical_size.height as f32,
                        );
                        this_window
                            .rend3_renderer
                            .set_camera_data(cam.to_rend3_camera());

                        this_window.egui_routine.resize(
                            physical_size.width,
                            physical_size.height,
                            this_window.raw_window.scale_factor() as f32,
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

                        if keycode == Some(winit::event::VirtualKeyCode::R) && !render_window_active
                        {
                            let new_window = create_render_window(window_target);
                            windows.insert(new_window.0, new_window.1);
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
                    w.raw_window.request_redraw();
                }
            }

            // Render!
            winit::event::Event::RedrawRequested(window_id) => {
                if (window_id == main_window_id) {
                    let w = windows.get_mut(&window_id).unwrap();

                    // UI
                    w.egui_context
                        .begin_frame(w.egui_winit_state.take_egui_input(&w.raw_window));

                    egui::Window::new("Change color")
                        .resizable(true)
                        .show(&w.egui_context, |ui| {
                            ui.label("Change the color of the cube");
                        });

                    egui::Window::new("Console")
                        .resizable(true)
                        .show(&w.egui_context, |ui| {
                            ui::console::draw_egui_console_menu(ui);
                            ui::console::draw_egui_logging_lines(ui);
                        });

                    let egui::FullOutput {
                        shapes,
                        textures_delta,
                        ..
                    } = w.egui_context.end_frame();

                    let clipped_meshes = &w.egui_context.tessellate(shapes);

                    let input = rend3_egui::Input {
                        clipped_meshes,
                        textures_delta,
                        context: w.egui_context.clone(),
                    };

                    // Get a frame
                    let frame = w.surface.get_current_texture().unwrap();

                    // Swap the instruction buffers so that our frame's changes can be processed.
                    w.rend3_renderer.swap_instruction_buffers();
                    // Evaluate our frame's world-change instructions
                    let mut eval_output = w.rend3_renderer.evaluate_instructions();

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
                    w.egui_routine.add_to_graph(&mut graph, input, frame_handle);

                    // Dispatch a render using the built up rendergraph!
                    graph.execute(&w.rend3_renderer, &mut eval_output);

                    // Present the frame
                    frame.present();

                    control.set_poll(); // default behavior
                } else {
                    let w = windows.get_mut(&window_id).unwrap();

                    // UI
                    w.egui_context
                        .begin_frame(w.egui_winit_state.take_egui_input(&w.raw_window));

                    egui::TopBottomPanel::top("my_panel").show(&w.egui_context, |ui| {
                       ui.label("Hello World! From `TopBottomPanel`, that must be before `CentralPanel`!");
                    });
                    egui::CentralPanel::default().show(&w.egui_context, |ui| {
                        render_texture.ui(ui);
                    });

                    let egui::FullOutput {
                        shapes,
                        textures_delta,
                        ..
                    } = w.egui_context.end_frame();

                    let clipped_meshes = &w.egui_context.tessellate(shapes);

                    let input = rend3_egui::Input {
                        clipped_meshes,
                        textures_delta,
                        context: w.egui_context.clone(),
                    };

                    // Get a frame
                    let frame = w.surface.get_current_texture().unwrap();

                    // Swap the instruction buffers so that our frame's changes can be processed.
                    w.rend3_renderer.swap_instruction_buffers();
                    // Evaluate our frame's world-change instructions
                    let mut eval_output = w.rend3_renderer.evaluate_instructions();

                    // Build a rendergraph
                    let mut graph = rend3::graph::RenderGraph::new();

                    // Import the surface texture into the render graph.
                    let frame_handle = graph.add_imported_render_target(
                        &frame,
                        0..1,
                        rend3::graph::ViewportRect::from_size(resolution),
                    );

                    w.egui_routine.add_to_graph(&mut graph, input, frame_handle);

                    // Dispatch a render using the built up rendergraph!
                    graph.execute(&w.rend3_renderer, &mut eval_output);

                    // Present the frame
                    frame.present();

                    control.set_poll(); // default behavior
                }
            }

            // Other events we don't care about
            _ => {}
        }

        let w = windows.get_mut(&main_window_id).unwrap();
        for input_event in input_state.get_input_events() {
            match input_event {
                input::InputEvent::DoViewportOrbit => {
                    cam.turntable_rotate(
                        &input_state.mouse.curr_cursor_pos
                            - input_state.mouse.cursor_pos_on_pressed.as_ref().unwrap(),
                        w.window_size.into(),
                    );
                    w.rend3_renderer.set_camera_data(cam.to_rend3_camera());
                    log::trace!("(event) do viewport orbit");
                }

                input::InputEvent::FinishViewportOrbit => {
                    cam.solidify_view_info();
                    w.rend3_renderer.set_camera_data(cam.to_rend3_camera());
                    log::trace!("(event) finish viewport orbit");
                }
            }
        }
        input_state.reset_release_events();
    });
}
