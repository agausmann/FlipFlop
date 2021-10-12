struct VertexInput {
    [[location(0)]] position: vec2<f32>;
    [[location(1)]] rect_position: vec2<f32>;
    [[location(2)]] z_index: f32;
    [[location(3)]] size: vec2<f32>;
    [[location(4)]] color: vec4<f32>;
    [[location(5)]] cluster_index: u32;
};

struct VertexOutput {
    [[builtin(position)]] position: vec4<f32>;
    [[location(0)]] color: vec4<f32>;
};

[[block]] struct Viewport {
    view_proj: mat4x4<f32>;
    view_size: vec2<f32>;
};
[[group(0), binding(0)]] var<uniform> viewport: Viewport;

[[block]] struct ClusterStates {
    buffer: array<u32, 1024>;
};
[[group(1), binding(0)]] var<uniform> cluster_states: ClusterStates;

[[stage(vertex)]]
fn main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    let rect_coordinate: vec2<f32> = in.rect_position + in.size * in.position;
    out.position = viewport.view_proj * vec4<f32>(rect_coordinate, in.z_index, 1.0);

    // 0xffffffffu - thanks WGSL
    if (in.cluster_index == 4294967295u) {
        out.color = in.color;
    } else {
        let array_index: u32 = in.cluster_index >> 6u;
        let bit_index: u32 = (in.cluster_index >> 1u) & 31u;
        let is_on: bool = (cluster_states.buffer[array_index] & (1u << bit_index)) != 0u;
        let invert: bool = (in.cluster_index & 1u) != 0u;

        if (is_on != invert) {
            // wire on
            out.color = vec4<f32>(1.0, 0.0, 0.0, 1.0);
        } else {
            out.color = vec4<f32>(0.0, 0.0, 0.0, 1.0);
        }
    }
    
    return out;
}

[[stage(fragment)]]
fn main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    return in.color;
}