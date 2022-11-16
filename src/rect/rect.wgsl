struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) rect_position: vec2<f32>,
    @location(2) z_index: f32,
    @location(3) size: vec2<f32>,
    @location(4) color: vec4<f32>,
    @location(5) cluster_index: u32,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

struct Viewport {
    view_proj: mat4x4<f32>,
    view_size: vec2<f32>,
};
@group(0) @binding(0) var<uniform> viewport: Viewport;

struct ClusterStates {
    buffer: array<vec4<u32>, 1024>,
};
@group(1) @binding(0) var<uniform> cluster_states: ClusterStates;
struct WirePalette {
    buffer: array<vec4<f32>, 2>,
};
@group(1) @binding(1) var<uniform> wire_palette: WirePalette;

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    let rect_coordinate: vec2<f32> = in.rect_position + in.size * in.position;
    out.position = viewport.view_proj * vec4<f32>(rect_coordinate, in.z_index, 1.0);

    if (in.cluster_index == 0xffffffffu) {
        out.color = in.color;
    } else {
        let array_index: u32 = in.cluster_index >> 8u;
        let component_index: u32 = (in.cluster_index >> 6u) & 3u;
        let bit_index: u32 = (in.cluster_index >> 1u) & 31u;
        let is_on: u32 = (cluster_states.buffer[array_index][component_index] >> bit_index) & 1u;
        let invert: u32 = in.cluster_index & 1u;

        out.color = wire_palette.buffer[is_on ^ invert];
    }
    
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
