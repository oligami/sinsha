# version 450

// STATIC CONSTANTS //
// 0. Vertices (Triangle Fan and Counter Clockwise)
vec2 VERTICES[4] = vec2[] (
    vec2 (-1.0, -1.0), // top left
    vec2 (-1.0,  1.0), // bottom left
    vec2 ( 1.0,  1.0), // bottom right
    vec2 ( 1.0, -1.0)  // top right
);

// SPECIALIZATION CONSTANTS //
// Those constants are dynamically determined in run-time when creating pipeline.
// 0. Extent2D of Surface
layout (constant_id = 0) const ivec2 EXTENT_2D = ivec2(1280, 720);

// UNIFORM //
// 0-0. Properties about Position of Rect2D
layout(set = 0, binding = 0) uniform Uniform {
    // Where the anchor of this drawing object is. Same as vulkan coordinates.
    // This anchor is on object coordinates and is the center of object's scaling;
    // normalized coordinates to pixel coordinate.
    vec2 object_anchor;

    // Where the anchor of rendering surface is. Same as vulkan coordinates.
    // This anchor is on surface coordinates and is a pin to render the object to surface.
    vec2 surface_anchor;

    // Difference between object anchor and surface anchor in rendering surface pixel size.
    // This vector is from surface anchor to object anchor.
    vec2 delta_of_anchor;

    // Scale of drawing object in rendering surface pixel size.
    vec2 scale;
} UNI;

// INPUTS //
// 0. Texture Coordinates
layout(location = 0) in vec2 tex_in;

// OUTPUTS //
// 0. Texture Coordinates
layout(location = 0) out vec2 tex_out;


// MAIN //
void main() {
    // MODIFY VERTEX POTITION //
    // 1. Get the current Vertex.
    vec2 vertex = VERTICES[gl_VertexIndex];
    // 2. Move against object anchor.
    vec2 moved_by_object_anchor = vertex - UNI.object_anchor;
    // 3. Scale vertex in pixel size.
    vec2 scaled = moved_by_object_anchor * UNI.scale;
    // 4. Move by delta of anchor.
    vec2 moved_by_delta_of_anchor = scaled + UNI.delta_of_anchor;
    // 5. Map vertex position into normalized surface coordinates.
    vec2 mapped_into_surface = moved_by_delta_of_anchor / (0.5 * EXTENT_2D);
    // 6. Finally, write vertex position into built-in parameter.
    gl_Position = vec4(mapped_into_surface, 0.0, 1.0);

    // PASS TEXTURE COORDINATES TO FRAGMENT SHADER //
    tex_out = tex_in;
}