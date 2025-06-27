struct Globals {
    model: mat4x4<f32>,
    // vp: mat4x4<f32>,
};

@group(0) @binding(0) var<uniform> globals : Globals;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec3<f32>,
    // Each instance
    @location(2) offset: vec3<f32>,
    @location(3) scale: f32,
    @location(4) color: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    // @location(1) @interpolate(flat) uv: vec3<f32>,
    // @location(1) @interpolate(perspective) uv: vec3<f32>,
    @location(1) @interpolate(perspective) color: vec3<f32>,
};

@vertex
fn vs_main(
    in: VertexInput,
) -> VertexOutput {
    let pos = vec4<f32>(in.position * in.scale + in.offset, 1.0);
    let world_pos = globals.model * pos;
    var out: VertexOutput;
    // out.position = globals.vp * world_pos;
    out.position = world_pos;
    // out.uv = in.uv;
    out.color = in.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}
