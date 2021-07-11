#version 450

layout(location=0) in vec2 v_uv;

layout(location=0) out vec4 f_color;

layout(set=0, binding=0) uniform Viewport {
    mat4 u_view_proj;
    vec2 u_view_size;
};

layout(set=1, binding=0) uniform sampler u_depth_sampler;
layout(set=1, binding=1) uniform texture2D u_depth_texture;
layout(set=1, binding=2) uniform CursorOutline {
    vec3 u_outline_color;
};

#define DEPTH sampler2D(u_depth_texture, u_depth_sampler)
#define OUTLINE_WIDTH 2

void main() {
    vec2 pixel_width = vec2(1.0 / u_view_size.x, 1.0 / u_view_size.y);

    // TODO this doesn't technically work for larger widths.
    float neighbors = 0.0;
    neighbors += texture(DEPTH, v_uv + pixel_width * OUTLINE_WIDTH * vec2(1.0, 1.0)).r;
    neighbors += texture(DEPTH, v_uv + pixel_width * OUTLINE_WIDTH * vec2(1.0, -1.0)).r;
    neighbors += texture(DEPTH, v_uv + pixel_width * OUTLINE_WIDTH * vec2(-1.0, 1.0)).r;
    neighbors += texture(DEPTH, v_uv + pixel_width * OUTLINE_WIDTH * vec2(-1.0, -1.0)).r;

    float alpha;
    if (neighbors == 0.0) {
        alpha = 0.0;
    } else {
        alpha = 1.0;
    }

    float depth = texture(DEPTH, v_uv).r;
    if (depth == 0.0) {
        f_color = vec4(u_outline_color, alpha);
    } else {
        discard;
    }
}
