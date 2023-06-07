mod data;
mod meta;
mod texture;

pub use data::WgsData;

pub const EXTENSION: &'static str = "wgs";
pub const FRAG_DEFAULT: &'static str = include_str!("./assets/frag.default.wgsl");
pub const VERT_DEFAULT: &'static str = include_str!("./assets/vert.wgsl");

const FRAG_PREFIX: &'static str = include_str!("./assets/frag.prefix.wgsl");
const FRAG_SUFFIX: &'static str = include_str!("./assets/frag.suffix.wgsl");

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
