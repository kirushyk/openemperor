#version 450
// PROTOTYPE: Vulkan fragment shader, compiled to SPIR-V by build.rs on Linux.
layout(location = 0) in vec4 in_color;
layout(location = 0) out vec4 out_color;

void main() {
    out_color = in_color;
}
