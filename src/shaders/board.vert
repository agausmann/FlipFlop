#version 450

layout(location=0) in vec2 a_position;

layout(location=1) in vec2 i_board_position;
layout(location=2) in vec2 i_board_dims;
layout(location=3) in vec4 i_board_color;
layout(location=4) in float i_z_index;

layout(location=0) out vec2 v_uv;
layout(location=1) out vec4 v_color;

layout(set=0, binding=0) uniform Viewport {
    mat4 u_view_proj;
    vec2 u_view_size;
};

void main() {
    vec2 board_coordinate = i_board_position + i_board_dims * a_position;

    gl_Position = u_view_proj * vec4(board_coordinate, i_z_index, 1.0);
    v_uv = board_coordinate;
    v_color = i_board_color;
}

