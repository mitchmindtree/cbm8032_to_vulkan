// NOTE: This shader requires being manually compiled to SPIR-V. If you update
// this shader, be sure to also re-compile it and update `vert.spv`. You can do
// so using `glslangValidator` with the following command:
// `glslangValidator -V shader.vert`

#version 450

// The quad vertex positions.
layout(location = 0) in vec2 position;
layout(location = 1) in vec2 tex_coords;

// The per-instance data.
layout(location = 2) in vec2 position_offset;
layout(location = 3) in vec2 tex_coords_offset;

// Feed the offset texture coordinatees through to the frag shader.
layout(location = 0) out vec2 v_tex_coords;

void main() {
    // Apply the tex coord offset for the instance.
    v_tex_coords = tex_coords + tex_coords_offset;
    // Apply the position offset for the instance.
    vec2 pos = position + position_offset;
    pos.y *= -1;
    gl_Position = vec4(pos, 0.0, 1.0);
}
