use std::{
    sync::Arc,
    thread::{self, JoinHandle},
};

use libloading;

pub struct RendererPlugin {
    library: Arc<libloading::Library>,
    thread_handle: Option<JoinHandle<anyhow::Result<()>>>,
    read_request: Arc<bool>,
    ready_to_read: Arc<bool>,
    render_width: u32,
    render_height: u32,
    render_rgb_data: Arc<Vec<f32>>,
}

impl RendererPlugin {
    pub fn load_plugin(
        path: &std::ffi::OsStr,
        render_width: u32,
        render_height: u32,
    ) -> anyhow::Result<Self> {
        let library = unsafe { Arc::new(libloading::Library::new(path)?) };

        Ok(Self {
            library,
            thread_handle: None,
            ready_to_read: Arc::new(false),
            read_request: Arc::new(false),
            render_width,
            render_height,
            render_rgb_data: Arc::new(vec![1.; (3 * render_width * render_height) as usize]),
        })
    }

    pub fn join_thread(&mut self) {
        let handle = std::mem::replace(&mut self.thread_handle, None);
        let _ = handle.unwrap().join();
    }

    pub fn render_is_finished(&self) -> bool {
        if let Some(handle) = &self.thread_handle {
            return handle.is_finished();
        }

        false
    }

    pub fn request_read(&mut self) {
        let read_request = Arc::as_ptr(&self.read_request).cast_mut();
        unsafe {
            *read_request = true;
        }
    }

    pub fn poll_read_request(&mut self) -> bool {
        if *self.ready_to_read {
            let read_request = Arc::as_ptr(&self.read_request).cast_mut();
            let ready_to_read = Arc::as_ptr(&self.ready_to_read).cast_mut();

            unsafe {
                *read_request = false;
                *ready_to_read = false;
            }

            return true;
        }

        false
    }

    /// Starts an incremental render. This spins up the plugin on another thread and then
    /// returns (without waiting for the plugin to finish rendering).
    ///
    /// Communication with the plugin is achieved by giving the plugin access to pointers
    /// whose memory is owned by the system. To achieve synchronization, a few flags are
    /// available to be set. Ultimately, it is the responsibility of the plugin (and the
    /// program) to properly update these flags and use them appropriately.
    ///
    /// ## Parameters:
    /// - `read_request`: the program sets this to `true` when it wants to read the
    /// current state of the render from `rgb_data`. The plugin should detect this and
    /// complete the necessary tasks for the incremental render to be read. The program
    /// should set this back to `false` when it is done reading the data.
    /// - `ready_to_read`: the plugin sets this to `true` to indicate that the program is
    /// free to read the data in `rgb_data`. When the program is done reading the data, it
    /// is responsible for setting this flag back to `false`.
    /// - `image_width`: width of the rendering surface in pixels
    /// - `image_height`: height of the rendering surface in pixels
    /// - `rgb_data`: the data for the render, represented as a vector with capacity (at
    /// least) `3*image_width*image_height`. The data is expected to be laid out as follows:
    /// the first `3*image_width` values correspond to the topmost row of pixels of the
    /// image, from left to right, divided into triples (0,1,2), (3,4,5), ... representing
    /// the RGB values of the pixel, e.g. the RGB values of the pixel in the top row,
    /// second column from the left are the values in indices (3,4,5) of the returned vector.
    /// **The program is responsible for keeping the data valid.**
    ///
    /// ## Returns
    /// - A `JoinHandle` to the spawned thread which called the render routine. The program
    /// can, for example, call `is_finished()` on this to see if the rendering is done.
    pub fn begin_incremental_render(&mut self) {
        let read_request_threaddata = self.read_request.clone();
        let ready_to_read_threaddata = self.ready_to_read.clone();
        let rgb_data_threaddata = self.render_rgb_data.clone();
        let image_width = self.render_width;
        let image_height = self.render_height;

        let lib_thread = self.library.clone();
        self.thread_handle = Some(thread::spawn(move || -> anyhow::Result<()> {
            unsafe {
                let read_request_param = Arc::as_ptr(&read_request_threaddata).cast_mut();
                let ready_to_read_param = Arc::as_ptr(&ready_to_read_threaddata).cast_mut();

                let rgb_data_param = (*Arc::as_ptr(&rgb_data_threaddata).cast_mut()).as_mut_ptr();

                let symbol: libloading::Symbol<FnBeginIncrementalRender> =
                    lib_thread.get(b"begin_incremental_render\0")?;
                (symbol)(
                    read_request_param,
                    ready_to_read_param,
                    image_width,
                    image_height,
                    rgb_data_param,
                );
            }

            Ok(())
        }));
    }

    pub fn convert_rgb_data_to_egui_image(&self) -> egui::ColorImage {
        let mut colors = vec![
            egui::Color32::from_rgb(255, 255, 255);
            (self.render_width * self.render_height) as usize
        ];

        for x in 0..self.render_width {
            for y in 0..self.render_height {
                let start_idx = (3 * x + 3 * y * self.render_width) as usize;

                let color = egui::Color32::from_rgb(
                    (self.render_rgb_data.get(start_idx).unwrap() * 255.999) as u8,
                    (self.render_rgb_data.get(start_idx + 1).unwrap() * 255.999) as u8,
                    (self.render_rgb_data.get(start_idx + 2).unwrap() * 255.999) as u8,
                );

                let idx = (x + y * self.render_width) as usize;
                colors[idx] = color;
            }
        }

        egui::ColorImage {
            size: [self.render_width as usize, self.render_height as usize],
            pixels: colors,
        }
    }
}

type FnBeginIncrementalRender = extern "C" fn(
    *mut bool,              // read_request
    *mut bool,              // ready_to_read
    std::ffi::c_uint,       // image_width
    std::ffi::c_uint,       // image_height
    *mut std::ffi::c_float, // rgb_data
);
