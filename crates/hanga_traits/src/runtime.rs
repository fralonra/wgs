use anyhow::Result;
use wgs_core::WgsData;

/// A basic trait for wgs runtime.
pub trait RuntimeExt {
    /// Adds a texture to wgs.
    fn add_texture(&mut self, width: u32, height: u32, buffer: Vec<u8>);

    /// Changes the texture of the given index in wgs.
    fn change_texture(&mut self, index: usize, width: u32, height: u32, buffer: Vec<u8>);

    /// Compiles wgs manually.
    fn compile(&mut self) -> Result<()>;

    /// Loads a wgs file.
    fn load(&mut self, wgs: WgsData) -> Result<()>;

    /// Pauses the runtime.
    fn pause(&mut self);

    /// Removes a texture from wgs.
    fn remove_texture(&mut self, index: usize);

    /// Do the rendering.
    fn render(&mut self) -> Result<()>;

    /// Resize the runtime.
    fn resize(&mut self, width: f32, height: f32);

    /// Restarts the rendering proccess of wgs.
    fn restart(&mut self);

    /// Resumes the runtime.
    fn resume(&mut self);

    /// Sets the content of the editable part of the fragment shader in wgs.
    fn set_wgs_frag(&mut self, shader_frag: &str);

    /// Sets the name for wgs.
    fn set_wgs_name(&mut self, name: &str);

    /// Calls when the cursor position changes.
    fn update_cursor(&mut self, cursor: [f32; 2]);

    /// Calls when the mouse is pressed.
    fn update_mouse_press(&mut self);

    /// Calls when the mouse is released.
    fn update_mouse_release(&mut self);

    /// Returns the wgs data.
    fn wgs(&self) -> &WgsData;
}
