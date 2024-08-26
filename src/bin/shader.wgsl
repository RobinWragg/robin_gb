// Vertex shader

struct VertInput {
    @location(0) pos: vec2<f32>,
    @location(1) uv: vec2<f32>,
};

struct VertToFrag {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@group(0) @binding(0)
var<uniform> matrix: mat4x4<f32>;

@vertex
fn vs_main(@builtin(vertex_index) vert_index: u32, vert: VertInput) -> VertToFrag {
    var out: VertToFrag;
    out.pos = vec4<f32>(vert.pos.x, vert.pos.y, 0.0, 1.0) * matrix;
    out.uv = vert.uv;
    return out;
}

// Fragment shader

@group(1) @binding(0)
var t_0: texture_2d<f32>;
@group(1) @binding(1)
var s_0: sampler;

@fragment
fn fs_main(in: VertToFrag) -> @location(0) vec4<f32> {
    let x = in.uv.x;
    let y = in.uv.y;
    let color = textureSample(t_0, s_0, vec2(x, y)).r;
    return vec4<f32>(color, color, color, color);
}