struct VertexInput {
    [[location(0)]] position: vec2<f32>;
    [[location(1)]] board_position: vec2<f32>;
    [[location(2)]] board_dims: vec2<f32>;
    [[location(3)]] board_color: vec4<f32>;
    [[location(4)]] z_index: f32;
};

struct VertexOutput {
    [[builtin(position)]] position: vec4<f32>;
    [[location(0)]] uv: vec2<f32>;
    [[location(1)]] color: vec4<f32>;
};

[[block]] struct Viewport {
    view_proj: mat4x4<f32>;
    view_size: vec2<f32>;
};
[[group(0), binding(0)]] var<uniform> viewport: Viewport;

[[stage(vertex)]]
fn main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    var board_coordinate: vec2<f32> = in.board_position + in.board_dims * in.position;
    out.position = viewport.view_proj * vec4<f32>(board_coordinate, in.z_index, 1.0);
    out.uv = board_coordinate;
    out.color = in.board_color;
    return out;
}

[[group(1), binding(0)]] var board_texture: texture_2d<f32>;
[[group(1), binding(1)]] var board_sampler: sampler;

[[stage(fragment)]]
fn main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    return textureSample(board_texture, board_sampler, in.uv) * in.color;
}