// NOTE: This shader requires being manually compiled to SPIR-V. If you update
// this shader, be sure to also re-compile it and update `frag.spv`. You can do
// so using `glslangValidator` with the following command:
// `glslangValidator -V shader.frag`

#version 450

layout(location = 0) in vec2 v_char_sheet_tex_coords;
layout(location = 1) in vec2 v_decay_tex_coords;
layout(location = 0) out vec4 f_color;

layout(set = 0, binding = 0) uniform Data {
    vec4 colouration;
    float sustain;
} uniforms;
layout(set = 0, binding = 1) uniform texture2D char_sheet;
layout(set = 0, binding = 2) uniform texture2D decay;
layout(set = 0, binding = 3) uniform sampler texture_sampler;

void main() {
    float char_sheet_color = texture(sampler2D(char_sheet, texture_sampler), v_char_sheet_tex_coords).r;
    float decay_color = texture(sampler2D(decay, texture_sampler), v_decay_tex_coords).r * uniforms.sustain;
    vec3 rgb = uniforms.colouration.rgb * max(char_sheet_color, decay_color);
    f_color = vec4(rgb, 1.0);
}
