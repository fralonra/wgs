fn main_image(frag_color: vec4<f32>, frag_coord: vec2<f32>) -> vec4<f32> {
    let uv = frag_coord / u.resolution;
    let uv0 = vec2(uv.x, (uv.y - 0.5) * 2.0);
    let uv1 = vec2(uv.x, uv.y * 2.0);
    let t0 = image(texture0, sampler0, uv0);
    let t1 = image(texture1, sampler1, uv1);
    return mix(t0, t1, step(0.5, 1.0 - uv.y));
}