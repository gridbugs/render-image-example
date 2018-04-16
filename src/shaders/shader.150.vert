#version 150 core

in vec2 a_CornerZeroToOne;

uniform Properties {
    vec2 u_WindowSizeInPixels;
};

out vec2 v_TexCoord;

vec2 coord_to_screen_space(vec2 coord) {
    return vec2(coord.x * 2 - 1, 1 - coord.y * 2);
}

void main() {
    v_TexCoord = a_CornerZeroToOne;
    gl_Position = vec4(coord_to_screen_space(a_CornerZeroToOne), 0, 1);
}
