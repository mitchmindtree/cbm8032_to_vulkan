#version 450

layout(location = 0) in vec2 v_tex_coords;
layout(location = 0) out vec4 f_color;

// Uniform data (same value for every pixel).
layout(set = 0, binding = 0) uniform Data {
    vec4 colouration;
} uniforms;

layout(set = 0, binding = 1) uniform sampler2D char_sheet;

float luminance(vec3 col) {
    return (col.r + col.g + col.b) / 3.0;
}

void main() {
    vec4 tex_color = texture(char_sheet, v_tex_coords);
    float l = luminance(tex_color.rgb);
    float alpha = uniforms.colouration.a;
    if (l > 0.5) {
        alpha = 1.0;
    }
    f_color = vec4(uniforms.colouration.rgb, alpha) * tex_color;
}
