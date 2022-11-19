struct Viewport {
    view_proj: mat4x4<f32>,
    view_size: vec2<f32>,
    view_size_tiles: vec2<f32>,
    view_offset_tiles: vec2<f32>,
};
@group(0) @binding(0) var<uniform> viewport: Viewport;

@group(1) @binding(0) var board_texture: texture_2d<f32>;
@group(1) @binding(1) var board_sampler: sampler;
struct Uniforms {
    color: vec4<f32>,
}
@group(1) @binding(2) var<uniform> uniforms: Uniforms;

@fragment
fn main(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    let centered = vec2(uv.x - 0.5, 0.5 - uv.y);
    let board_uv = centered * viewport.view_size_tiles + viewport.view_offset_tiles;
    return textureSample(board_texture, board_sampler, board_uv) * uniforms.color;
}