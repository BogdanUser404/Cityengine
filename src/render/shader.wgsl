struct Camera {
	offset: vec2<f32>,
	zoom:   f32,
	_pad0:  f32,
	screen: vec2<f32>,
	atlas:  vec2<u32>,
};
@group(0) @binding(0) var<uniform> cam: Camera;
@group(0) @binding(1) var atlas_tex: texture_2d<f32>;
@group(0) @binding(2) var atlas_samp: sampler;

struct InstIn {
	@location(0) pos: u32,
	@location(1) tile_flags: u32,
};

struct VOut {
	@builtin(position) clip: vec4<f32>,
	@location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vi: u32, inst: InstIn) -> VOut {
	var corners = array<vec2<f32>, 6>(
		vec2(0.0, 0.0), vec2(1.0, 0.0), vec2(0.0, 1.0),
		vec2(1.0, 0.0), vec2(1.0, 1.0), vec2(0.0, 1.0),
	);
	let c = corners[vi];

	let wx = f32(inst.pos & 0xFFFFu) + c.x;
	let wy = f32(inst.pos >> 16u)    + c.y;

	let sx = (wx - cam.offset.x) * cam.zoom;
	let sy = (wy - cam.offset.y) * cam.zoom;
	let clip = vec2<f32>(
		(sx / cam.screen.x) * 2.0 - 1.0,
		1.0 - (sy / cam.screen.y) * 2.0,
	);

	let tile_id = inst.tile_flags & 0xFFFFu;
	let col = f32(tile_id % cam.atlas.x);
	let row = f32(tile_id / cam.atlas.x);
	let dims = vec2<f32>(f32(cam.atlas.x), f32(cam.atlas.y));
	let uv0 = vec2<f32>(col, row) / dims;
	let uv1 = uv0 + vec2<f32>(1.0, 1.0) / dims;

	return VOut(vec4<f32>(clip, 0.0, 1.0), mix(uv0, uv1, c));
}

@fragment
fn fs_main(in: VOut) -> @location(0) vec4<f32> {
	return textureSample(atlas_tex, atlas_samp, in.uv);
}