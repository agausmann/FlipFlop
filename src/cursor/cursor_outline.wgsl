struct Viewport {
    view_proj: mat4x4<f32>,
    view_size: vec2<f32>,
    view_size_tiles: vec2<f32>,
    view_offset_tiles: vec2<f32>,
};
@group(0) @binding(0) var<uniform> viewport: Viewport;

struct CursorOutline {
    color: vec3<f32>,
};
@group(1) @binding(0) var depth_sampler: sampler;
@group(1) @binding(1) var depth_texture: texture_depth_2d;
@group(1) @binding(2) var<uniform> cursor_outline: CursorOutline;

let OUTLINE_WIDTH: f32 = 2.0;

fn depth(coordinate: vec2<f32>) -> f32 {
    return textureSample(depth_texture, depth_sampler, coordinate);
}

@fragment
fn fs_main(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    let pixel_width = vec2<f32>(1.0 / viewport.view_size.x, 1.0 / viewport.view_size.y);
    let neighbor_distance = pixel_width * OUTLINE_WIDTH;

    // TODO this doesn't technically work for larger widths.
    var neighbors: f32 = 0.0;
    neighbors = neighbors + depth(uv + neighbor_distance * vec2<f32>(1.0, 1.0));
    neighbors = neighbors + depth(uv + neighbor_distance * vec2<f32>(1.0, -1.0));
    neighbors = neighbors + depth(uv + neighbor_distance * vec2<f32>(-1.0, 1.0));
    neighbors = neighbors + depth(uv + neighbor_distance * vec2<f32>(-1.0, -1.0));

    var alpha: f32;
    if (neighbors == 0.0) {
        alpha = 0.0;
    } else {
        alpha = 1.0;
    }
    
    if (depth(uv) == 0.0) {
        return vec4<f32>(cursor_outline.color, alpha);
    } else {
        discard;
    }
}
