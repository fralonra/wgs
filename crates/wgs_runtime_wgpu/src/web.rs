use crate::{runtime::Runtime, RuntimeExt};
use js_sys::Promise;
use std::io::Cursor;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::future_to_promise;
use web_sys::HtmlCanvasElement;
use wgs_core::WgsData;

#[wasm_bindgen(js_name = Runtime)]
pub struct WebRuntime {
    inner: Runtime,
}

#[wasm_bindgen(js_class = Runtime)]
impl WebRuntime {
    pub fn add_texture(&mut self, width: u32, height: u32, buffer: &[u8]) {
        self.inner.add_texture(width, height, buffer.to_vec());
    }

    pub fn change_texture(&mut self, index: usize, width: u32, height: u32, buffer: &[u8]) {
        self.inner
            .change_texture(index, width, height, buffer.to_vec());
    }

    pub fn compile(&mut self) {
        self.inner.compile().unwrap();
    }

    pub fn load(&mut self, raw_wgs: &[u8]) {
        let mut cursor = Cursor::new(raw_wgs);

        let wgs = WgsData::load(&mut cursor).unwrap();

        self.inner.load(wgs);
    }

    pub fn pause(&mut self) {
        self.inner.pause();
    }

    pub fn remove_texture(&mut self, index: usize) {
        self.inner.remove_texture(index);
    }

    pub fn render(&mut self) {
        self.inner.frame_start().unwrap();

        self.inner.render();

        self.inner.frame_finish().unwrap();
    }

    pub fn resize(&mut self, width: f32, height: f32) {
        self.inner.resize(width, height);
    }

    pub fn restart(&mut self) {
        self.inner.restart();
    }

    pub fn resume(&mut self) {
        self.inner.resume();
    }

    pub fn set_wgs_frag(&mut self, shader_frag: &str) {
        self.inner.set_wgs_frag(shader_frag);
    }

    pub fn set_wgs_name(&mut self, name: &str) {
        self.inner.set_wgs_name(name);
    }

    pub fn update_cursor(&mut self, cursor_x: f32, cursor_y: f32) {
        self.inner.update_cursor([cursor_x, cursor_y]);
    }

    pub fn update_mouse_press(&mut self) {
        self.inner.update_mouse_press();
    }

    pub fn update_mouse_release(&mut self) {
        self.inner.update_mouse_release();
    }
}

#[wasm_bindgen]
pub fn setup(canvas: HtmlCanvasElement) -> Promise {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));

    let width = canvas.width() as f32;
    let height = canvas.height() as f32;

    future_to_promise(async move {
        match Runtime::new(canvas, WgsData::default(), None).await {
            Ok(inner) => {
                let mut runtime = WebRuntime { inner };

                runtime.resize(width, height);

                Ok(runtime.into())
            }
            Err(err) => Err(err.to_string().into()),
        }
    })
}
