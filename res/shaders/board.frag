#version 450

layout(location=0) in vec2 v_uv;
layout(location=1) in vec4 v_color;

layout(location=0) out vec4 f_color;

layout(set=1, binding=0) uniform texture2D board_texture;
layout(set=1, binding=1) uniform sampler board_sampler;

void main() {
    f_color = texture(sampler2D(board_texture, board_sampler), v_uv) * v_color;
}
