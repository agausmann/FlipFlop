#version 450

layout(location=0) in vec2 a_position;

layout(location=1) in vec2 i_rect_position;
layout(location=2) in float i_rect_z_index;
layout(location=3) in vec2 i_rect_size;
layout(location=4) in vec4 i_rect_color;
layout(location=5) in uint i_cluster_index;

layout(location=0) out vec4 v_color;

layout(set=0, binding=0) uniform Viewport {
    mat4 u_view_proj;
};

layout(set=1, binding=0) readonly buffer ClusterStates {
    uint u_cluster_states[];
};

void main() {
    vec2 rect_coordinate = i_rect_position + i_rect_size * a_position;
    gl_Position = u_view_proj * vec4(rect_coordinate, i_rect_z_index, 1.0);

    if (i_cluster_index == 0xffffffff) {
        v_color = i_rect_color;
    } else if ((u_cluster_states[i_cluster_index >> 5] & (1 << (i_cluster_index & 0x1f))) != 0) {
        // wire on
        v_color = vec4(1.0, 0.0, 0.0, 1.0);
    } else {
        // wire off
        v_color = vec4(0.0, 0.0, 0.0, 1.0);
    }
}

