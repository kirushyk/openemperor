#version 450
// PROTOTYPE: Vulkan vertex shader, compiled to SPIR-V by build.rs on Linux.
layout(location = 0) in vec2 in_pos;
layout(location = 1) in vec4 in_color;
layout(location = 0) out vec4 out_color;
layout(push_constant) uniform Push { vec2 size; } push;

void main() {
    // Vulkan clip space already has y pointing down, matching our pixel coordinates.
    gl_Position = vec4(in_pos / push.size * 2.0 - 1.0, 0.0, 1.0);
    out_color = in_color;
}
