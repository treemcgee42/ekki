pub mod node_map;
pub mod render;
pub mod scene_viewer_3d;
pub mod startup;

use std::sync::Arc;

use crate::{
    camera::Camera,
    create_mesh,
    grid::GridRenderRoutine,
    input::{self, InputState},
    ui, MyImage, WindowCloseCallbackCommand, WindowRedrawCallbackCommand,
};

struct WindowInfo {
    pub raw_window: winit::window::Window,
    pub window_size: winit::dpi::PhysicalSize<u32>,
    pub surface: Arc<wgpu::Surface>,
    pub preferred_texture_format: wgpu::TextureFormat,
    pub egui_routine: rend3_egui::EguiRenderRoutine,
    pub egui_context: egui::Context,
    pub egui_winit_state: egui_winit::State,
    pub rend3_renderer: Arc<rend3::Renderer>,
}

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
