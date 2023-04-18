use libloading;

pub struct ExternalRenderer {
    library: libloading::Library,
}

impl ExternalRenderer {
    fn load_renderer(path: &std::ffi::OsStr) -> Result<Self, anyhow::Error> {
        let library = unsafe { libloading::Library::new(path)? };

        Ok(Self { library })
    }

    fn get_name(&self) -> Result<&'static str, anyhow::Error> {
        let symbol: libloading::Symbol<FnGetName> = unsafe { self.library.get(b"get_name\0")? };

        Ok((symbol)())
    }

    /// This function is called whenever the system wants to update the rendering preview, whether 
    /// that be in the viewport or in a seperate window. The system is responsible for deciding how
    /// frequently to request updates, blocking, etc. For example, an update may be requested every 
    /// second. If the scene itself changes, or the resolution of the surface being rendered to 
    /// changes, this function is not called.
    ///
    /// ## Parameters:
    /// - `image_width`: width of the rendering surface in pixels
    /// - `image_height`: height of the rendering surface in pixels
    /// - `rgb_data`: a Vector with capacity at least `3*image_width*image_height`
    ///
    /// ## Returns:
    /// - on success, a vector with data laid out as follows: the first `3*image_width` values correspond 
    /// to the topmost row of pixels of the image, from left to right, divided into triples (0,1,2), 
    /// (3,4,5), ... representing the RGB values of the pixel, e.g. the RGB values of the pixel in the top row, 
    /// second column from the left are the values in indices (3,4,5) of the returned vector.
    fn update_render_preview(
        &self,
        image_width: u32,
        image_height: u32,
        mut rgb_data: Vec<f32>,
    ) -> Result<Vec<f32>, anyhow::Error> {
        let ptr = rgb_data.as_mut_ptr();
        std::mem::forget(rgb_data);

        let symbol: libloading::Symbol<FnUpdateRenderPreview> =
            unsafe { self.library.get(b"update_render_preview\0")? };
        (symbol)(image_width, image_height, ptr);

        let new_vec = unsafe {
            Vec::from_raw_parts(
                ptr,
                (image_width * image_height) as usize,
                (image_width * image_height) as usize,
            )
        };

        Ok(new_vec)
    }
}

type FnGetName = extern "C" fn() -> &'static str;
type FnUpdateRenderPreview =
    extern "C" fn(std::ffi::c_uint, std::ffi::c_uint, *mut std::ffi::c_float);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t1() {
        let dummy_renderer =
            ExternalRenderer::load_renderer(std::ffi::OsStr::new("ffi-tests/libdummy_renderer.so"))
                .unwrap();

        let rgb_data: Vec<f32> = Vec::with_capacity(35);

        let updated_data = dummy_renderer
            .update_render_preview(5, 7, rgb_data)
            .unwrap();

        println!("data: {:?}", updated_data);
    }
}
