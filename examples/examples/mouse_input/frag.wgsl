fn circle(uv: vec2<f32>, pos: vec2<f32>, radius: f32, color: vec3<f32>) -> vec4<f32> {
	let d = length(pos - uv) - radius;
	let t = clamp(d, 0.0, 1.0);
	return vec4(color, 1.0 - t);
}

fn main_image(frag_color: vec4<f32>, frag_coord: vec2<f32>) -> vec4<f32> {
    if u.mouse_down == 0u {
        return vec4(0.3);
    }

    let radius_factor = 0.2;
    let radius = radius_factor * min(u.resolution.x, u.resolution.y);
    let circle = circle(frag_coord, u.cursor, radius, vec3(1.0));

    return mix(vec4(0.3), circle, circle.a);
}