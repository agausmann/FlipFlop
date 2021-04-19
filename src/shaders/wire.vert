#version 450

layout(location=0) in vec2 a_position_const;
layout(location=1) in vec2 a_position_lin;

layout(location=2) in uint i_wire_cluster_index;
layout(location=3) in vec2 i_wire_position;
layout(location=4) in vec2 i_wire_size;

layout(location=0) out vec4 v_color;

layout(set=0, binding=0) uniform Viewport {
    mat4 u_view_proj;
};
layout(set=1, binding=0) uniform WireColor {
    vec4 u_off_color;
    vec4 u_on_color;
};
layout(set=1, binding=1) uniform WireState {
    uint u_cluster_states[1024];
};

void main() {
    vec2 wire_coordinate = i_wire_position + a_position_const + i_wire_size * a_position_lin;

    gl_Position = u_view_proj * vec4(wire_coordinate, 0.0, 1.0);

    if ((u_cluster_states[i_wire_cluster_index / 32] & (1 << (i_wire_cluster_index % 32))) != 0) {
        v_color = u_on_color;
    } else {
        v_color = u_off_color;
    }
}

