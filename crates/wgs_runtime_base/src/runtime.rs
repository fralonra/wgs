use anyhow::Result;
use wgs_core::WgsData;

pub trait RuntimeExt {
    fn add_texture(&mut self, width: u32, height: u32, buffer: Vec<u8>);

    fn change_texture(&mut self, index: usize, width: u32, height: u32, buffer: Vec<u8>);

    fn compile(&mut self) -> Result<()>;

    fn load(&mut self, wgs: WgsData) -> Result<()>;

    fn pause(&mut self);

    fn remove_texture(&mut self, index: usize);

    fn render(&mut self) -> Result<()>;

    fn resize(&mut self, width: f32, height: f32);

    fn restart(&mut self);

    fn resume(&mut self);

    fn set_wgs_frag(&mut self, shader_frag: &str);

    fn set_wgs_name(&mut self, name: &str);

    fn update_cursor(&mut self, cursor: [f32; 2]);

    fn update_mouse_press(&mut self);

    fn update_mouse_release(&mut self);

    fn wgs(&self) -> &WgsData;
}
