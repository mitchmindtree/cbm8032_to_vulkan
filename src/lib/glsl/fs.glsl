#version 450

layout(location = 0) in vec2 v_tex_coords;
layout(location = 0) out vec4 f_color;

// Uniform data (same value for every pixel).
layout(set = 0, binding = 0) uniform Data {
    vec4 colouration;
} uniforms;

layout(set = 0, binding = 1) uniform sampler2D char_sheet;

void main() {
    vec4 tex_color = texture(char_sheet, v_tex_coords);
    f_color = uniforms.colouration * tex_color;
}
