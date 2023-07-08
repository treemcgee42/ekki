//! Handles window creation and behavior.
//!
//! Windows must implement the `WindowLike` trait, which is what the
//! event loop uses to inform the windows about updates and input. Windows
//! themselves dictate the interface through which they are created;
//! all our windows will implement a `create()` method.

pub mod node_map;
pub mod render;
pub mod scene_viewer_3d;
pub mod startup;

use std::sync::Arc;

use crate::{
    camera::Camera,
    grid::GridRenderRoutine,
    input::{self, InputState},
    ui, MyImage, WindowCloseCallbackCommand, WindowRedrawCallbackCommand,
};

// ===== WindowInfo {{{1

/// Contains common data required by all windows.
pub(crate) struct WindowInfo {
    pub raw_window: winit::window::Window,
    pub window_id: winit::window::WindowId,
    pub window_size: winit::dpi::PhysicalSize<u32>,
    pub resolution: glam::UVec2,
    pub surface: Arc<wgpu::Surface>,
    pub preferred_texture_format: wgpu::TextureFormat,
    pub egui_routine: rend3_egui::EguiRenderRoutine,
    pub egui_context: egui::Context,
    pub egui_winit_state: egui_winit::State,
    pub rend3_renderer: Arc<rend3::Renderer>,
}

pub(crate) struct WindowInfoInitializeInfo {
    pub title: String,
    pub inner_size: Option<winit::dpi::PhysicalSize<u32>>,
    pub with_decorations: bool,
    pub with_position: Option<winit::dpi::PhysicalPosition<u32>>,
}

impl Default for WindowInfoInitializeInfo {
    fn default() -> Self {
        Self {
            title: "untitled window".to_string(),
            inner_size: None,
            with_decorations: true,
            with_position: None,
        }
    }
}

impl WindowInfo {
    pub fn initialize<T>(
        window_target: &winit::event_loop::EventLoopWindowTarget<T>,
        info: WindowInfoInitializeInfo,
    ) -> Self
    where
        T: 'static,
    {
        let window = {
            let builder = winit::window::WindowBuilder::new();
            let w = builder
                .with_title(info.title)
                .with_decorations(info.with_decorations)
                .build(window_target)
                .expect("Could not build window");

            if let Some(inner_size) = info.inner_size {
                w.set_inner_size(inner_size);
            }

            if let Some(with_position) = info.with_position {
                w.set_outer_position(with_position)
            }

            w
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
        let mut platform = egui_winit::State::new(window_target);
        platform.set_pixels_per_point(window.scale_factor() as f32);

        let resolution = glam::UVec2::new(window_size.width, window_size.height);

        Self {
            raw_window: window,
            window_id,
            window_size,
            resolution,
            surface,
            preferred_texture_format: preferred_format,
            egui_routine,
            egui_context: context,
            egui_winit_state: platform,
            rend3_renderer: renderer,
        }
    }

    /// Convenience function that modifies fields for a resize. For most purposes,
    /// callinng this function is all a window needs to do to handle a resize.
    pub fn resize_default(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.resolution = glam::UVec2::new(new_size.width, new_size.height);

        // Reconfigure the surface for the new size.
        rend3::configure_surface(
            &self.surface,
            &self.rend3_renderer.device,
            self.preferred_texture_format,
            glam::UVec2::new(self.resolution.x, self.resolution.y),
            rend3::types::PresentMode::Fifo,
        );

        // Tell the renderer about the new aspect ratio.
        let aspect_ratio = self.resolution.x as f32 / self.resolution.y as f32;
        self.rend3_renderer.set_aspect_ratio(aspect_ratio);

        self.egui_routine.resize(
            new_size.width,
            new_size.height,
            self.raw_window.scale_factor() as f32,
        );
    }
}

// ===== WindowInfo }}}1

pub trait WindowLike {
    fn get_window_id(&self) -> winit::window::WindowId;

    fn egui_event_consumed(&mut self, event: &winit::event::WindowEvent) -> bool;
    fn resize(&mut self, physical_size: winit::dpi::PhysicalSize<u32>);

    fn request_redraw(&self);
    fn redraw(&mut self) -> Option<Vec<WindowRedrawCallbackCommand>>;
    fn close_requested(&mut self) -> WindowCloseCallbackCommand {
        WindowCloseCallbackCommand::Close
    }

    fn handle_input_event(&mut self, input_state: &InputState, input_event: input::InputEvent);
}
