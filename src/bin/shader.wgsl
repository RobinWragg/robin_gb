// Vertex shader

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
};

@vertex
fn vs_main(
    @builtin(vertex_index) in_vertex_index: u32,
) -> VertexOutput {
    var out: VertexOutput;
    var x = f32(in_vertex_index % 2) * 2.0 - 1.0;
    var y = f32(in_vertex_index / 2) * 2.0 - 1.0;
    out.clip_position = vec4<f32>(x, y, 0.0, 1.0);
    return out;
}

// Fragment shader

@group(0) @binding(0)
var t_0: texture_2d<f32>;
@group(0) @binding(1)
var s_0: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // convert normalized device coordinates to pixel coordinates.
    // rwtodo: Define these constants as uniforms or similar.
    let x = in.clip_position.x / 640.0;
    let y = in.clip_position.y / 576.0;
    let color = textureSample(t_0, s_0, vec2(x, y)).r;
    return vec4<f32>(color, color, color, 1.0);
}