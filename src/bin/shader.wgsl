// Vertex shader

struct VertToFrag {
    @builtin(position) position: vec4<f32>,
    @location(0) tex_coord: vec2<f32>,
};

@vertex
fn vs_main(
    @builtin(vertex_index) vert_index: u32,
) -> VertToFrag {
    let x = f32(vert_index % 2);
    let y = f32(vert_index / 2);

    var out: VertToFrag;
    out.position = vec4<f32>(x * 2.0 - 1.0, y * 2.0 - 1.0, 0.0, 1.0);
    out.tex_coord = vec2<f32>(x, 1.0 - y);
    return out;
}

// Fragment shader

@group(0) @binding(0)
var t_0: texture_2d<f32>;
@group(0) @binding(1)
var s_0: sampler;

@fragment
fn fs_main(in: VertToFrag) -> @location(0) vec4<f32> {
    // convert normalized device coordinates to pixel coordinates.
    // rwtodo: Define these constants as uniforms or similar.
    let x = in.tex_coord.x;
    let y = in.tex_coord.y;
    let color = textureSample(t_0, s_0, vec2(x, y)).r;
    return vec4<f32>(color, color, color, 1.0);
}