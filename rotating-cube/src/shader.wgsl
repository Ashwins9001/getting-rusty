// 1. Camera uniform (view + projection)
struct Camera {
    view_proj: mat4x4<f32> // 4x4 matrix for view-projection
};
@group(0) @binding(0)
var<uniform> camera: Camera;

// 2. Model uniform (rotation)
struct Model {
    model: mat4x4<f32> // 4x4 matrix for model transform (rotation, scaling, translation)
};
@group(0) @binding(1)
var<uniform> model: Model;

// 3. Vertex input
struct VertexInput {
    @location(0) position: vec3<f32> // vertex position
    @location(1) color: vec3<f32>    // vertex color
};

// 4. Vertex output to fragment shader
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32> // where GPU draws vertex in clip-space
    @location(0) frag_color: vec3<f32>         // pass color to fragment shader
};

// 5. Vertex shader
@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    // Transform vertex: model -> world -> camera -> clip
    output.clip_position = camera.view_proj * model.model * vec4<f32>(input.position, 1.0);
    output.frag_color = input.color; // pass color to fragment shader
    return output;
}

// 6. Fragment shader
@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(input.frag_color, 1.0); // final pixel color
}
