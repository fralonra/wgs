# Wgs

[![Latest version](https://img.shields.io/crates/v/wgs_core.svg)](https://crates.io/crates/wgs_core)
[![Documentation](https://docs.rs/wgs_core/badge.svg)](https://docs.rs/wgs_core)
![MIT](https://img.shields.io/badge/license-MIT-blue.svg)

`wgs` is a binary format that represents pixel shader programs. Inspired by [Shadertoy](https://www.shadertoy.com/) but uses [WGSL](https://www.w3.org/TR/WGSL/) instead. It can now runs on native platforms and Web as well thanks to [wgpu-rs](https://wgpu.rs/).

## File strcuture

A `wgs` file mainly consists of three parts:

- **meta** which contains the meta info of the file, including:
  - **name** project name.
  - **texture_count** the count of the texture used by the file.
  - **version** the wgs version used in the file.
- **frag** the shader program in WGSL format.
- **textures** the textures used by the file. Each texture consists of it's width and height and color data in 8bit RGBA format.

## Version

The latest version of `wgs` is **wgs 1**.

_Notice_ The very first version of `wgs` does not include `version` field and uses a `texture` function to render textures which is conflicting with the keyword in `GLSL`. Thus, this first version is not compatible with any later versions.

## How to write wgs

[WgShadertoy](https://github.com/fralonra/wgshadertoy) is a cross-platform program helps you read and write your `wgs` files.

Maybe Web-based editors in the future.

### Uniforms

A `wgs` program receives six parameters passed from the runtime as a uniform variable:

- `cursor`: _vec2<f32>_
  - The mouse position in pixels.
- `mouse_down`: _u32_
  - Whether the left button of the mouse is down.
  - `0`: left button is up.
  - `1`: left button is down.
- `mouse_press`: _vec2<f32>_
  - The mouse position in pixels when the left button is pressed.
- `mouse_release`: _vec2<f32>_
  - The mouse position in pixels when the left button is released.
- `resolution`: _vec2<f32>_
  - The resolution of the canvas in pixels (width \* height).
- `time`: _f32_
  - The elapsed time since the shader first ran, in seconds.

You can use the above uniform like the following:

```wgsl
fn main_image(frag_color: vec4<f32>, frag_coord: vec2<f32>) -> vec4<f32> {
    let uv = frag_coord / u.resolution;
    let color = 0.5 + 0.5 * cos(u.time + uv.xyx + vec3(0.0, 2.0, 4.0));
    return vec4(color, 1.0);
}
```

### Built-in functions

`wgs` currently provides one built-in function:

- **image** helps you play with textures:

  ```wgsl
  fn image(t: texture_2d<f32>, spl: sampler, uv: vec2<f32>) -> vec4<f32>
  ```

  Check this [example](https://github.com/fralonra/wgs/tree/master/examples/examples/texture) for usage.

## How to run wgs

### Native

[wgs_runtime_wgpu](https://github.com/fralonra/wgs/tree/master/crates/wgs_runtime_wgpu) is all you need to run `wgs` file on a native platform.

Here's an [example](https://github.com/fralonra/wgs/tree/master/crates/winit_demo) about how to integrate `wgs` with [winit](https://github.com/rust-windowing/winit).

You can write your own runtime implementation as long as it implements [`RuntimeExt`](https://github.com/fralonra/wgs/blob/master/crates/wgs_runtime_base/src/runtime.rs).

### Web

`wgs_runtime_wgpu` also compiles for Wasm32 architecture.

You can install it from [npm](https://www.npmjs.com/package/wgs-runtime-wgpu) or use a high-level library [`wgs-player`](https://github.com/fralonra/wgs-player).
