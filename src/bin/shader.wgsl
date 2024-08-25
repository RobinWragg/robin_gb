// Vertex shader

struct VertInput {
    @location(0) pos: vec2<f32>,
    @location(1) texcoord: vec2<f32>,
};

struct VertToFrag {
    @builtin(position) pos: vec4<f32>,
    @location(0) tex_coord: vec2<f32>,
};

@group(0) @binding(0)
var<uniform> matrix: mat4x4<f32>;

@vertex
fn vs_main(@builtin(vertex_index) vert_index: u32, vert: VertInput) -> VertToFrag {
    var out: VertToFrag;
    out.pos = vec4<f32>(vert.pos.x, vert.pos.y, 0.0, 1.0) * matrix;
    out.tex_coord = vec2<f32>(vert.pos.x, 1.0 - vert.pos.y);
    return out;
}

// Fragment shader

@group(0) @binding(1)
var t_0: texture_2d<f32>;
@group(0) @binding(2)
var s_0: sampler;

@fragment
fn fs_main(in: VertToFrag) -> @location(0) vec4<f32> {
    let x = in.tex_coord.x;
    let y = in.tex_coord.y;
    let color = textureSample(t_0, s_0, vec2(x, y)).r;
    return vec4<f32>(x + y, color, color, 1.0);
}