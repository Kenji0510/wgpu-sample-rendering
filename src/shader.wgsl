struct ViewProj { vp: mat4x4<f32>, };
@group(0) @binding(0) var<uniform> viewproj : ViewProj;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    // Each instance
    @location(2) inst_model_0 : vec4<f32>,
    @location(3) inst_model_1 : vec4<f32>,
    @location(4) inst_model_2 : vec4<f32>,
    @location(5) inst_model_3 : vec4<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(1) @interpolate(flat) uv: vec2<f32>,
    // @location(1) @interpolate(perspective) uv: vec2<f32>,
};

@vertex
fn vs_main(
    in: VertexInput,
) -> VertexOutput {
    let model = mat4x4<f32>(
        in.inst_model_0,
        in.inst_model_1,
        in.inst_model_2,
        in.inst_model_3,
    );
    let world_pos = model * vec4<f32>(in.position, 1.0);
    var out: VertexOutput;
    out.position = viewproj.vp * world_pos;
    out.uv = in.uv;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.uv, 0.2, 1.0);
}
