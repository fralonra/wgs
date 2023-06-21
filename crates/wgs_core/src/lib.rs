//! The core implementation of a wgs file.

mod data;
mod meta;
mod texture;

pub use data::WgsData;

/// The extension of a wgs file.
pub const EXTENSION: &'static str = "wgs";
/// The version ofa `wgs file.
pub const VERSION: u32 = 1;

/// The content of the default editable part in a fragment shader for a wgs file.
pub const FRAG_DEFAULT: &'static str = include_str!("./assets/frag.default.wgsl");
/// The content of the default vertex shader for a wgs file.
pub const VERT_DEFAULT: &'static str = include_str!("./assets/vert.wgsl");

const FRAG_PREFIX: &'static str = include_str!("./assets/frag.prefix.wgsl");
#[cfg(target_arch = "wasm32")]
const FRAG_SUFFIX: &'static str = include_str!("./assets/frag.suffix.gl.wgsl");
#[cfg(not(target_arch = "wasm32"))]
const FRAG_SUFFIX: &'static str = include_str!("./assets/frag.suffix.wgsl");

/// A util function helps to generate a complete fragment shader.
pub fn concat_shader_frag(main_image: &str, texture_count: usize) -> String {
    let mut texture2ds = String::new();
    for index in 0..texture_count {
        texture2ds.push_str(&format!("@group({}) @binding(0)\n", index + 1,));
        texture2ds.push_str(&format!("var texture{}: texture_2d<f32>;\n", index));
        texture2ds.push_str(&format!("@group({}) @binding(1)\n", index + 1,));
        texture2ds.push_str(&format!("var sampler{}: sampler;\n", index));
    }

    format!(
        "{}\n{}\n{}\n{}",
        FRAG_PREFIX, texture2ds, main_image, FRAG_SUFFIX
    )
}
