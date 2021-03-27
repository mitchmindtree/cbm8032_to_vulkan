// NOTE: This shader requires being manually compiled to SPIR-V. If you update
// this shader, be sure to also re-compile it and update `frag.spv`. You can do
// so using `glslangValidator` with the following command:
// `glslangValidator -V decay_shader.frag -o decay_frag.spv`

#version 450

layout(location = 0) in vec2 v_char_sheet_tex_coords;
layout(location = 0) out vec4 f_color;

layout(set = 0, binding = 0) uniform Data {
    vec4 colouration;
    float sustain;
} uniforms;
layout(set = 0, binding = 1) uniform texture2D char_sheet;
layout(set = 0, binding = 2) uniform sampler texture_sampler;

void main() {
    float l = texture(sampler2D(char_sheet, texture_sampler), v_char_sheet_tex_coords).r;
    float a = uniforms.colouration.a;
    if (l > 0.5) {
        a = 1.0;
    }
    f_color = vec4(l, l, l, a);
}
