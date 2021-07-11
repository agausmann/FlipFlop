#version 450

layout(location=0) in vec2 a_position;

layout(location=0) out vec2 v_uv;

void main() {
    gl_Position = vec4(a_position * 2.0 - 1.0, 0.0, 1.0);
    v_uv = vec2(a_position.x, 1 - a_position.y);
}

