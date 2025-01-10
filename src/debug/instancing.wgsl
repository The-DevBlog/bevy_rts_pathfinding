#import bevy_pbr::mesh_functions::{get_world_from_local, mesh_position_local_to_clip}

struct Vertex {
    @location(0) position: vec3<f32>,
    @location(3) pos_scale: vec4<f32>,
    @location(4) rotation: vec4<f32>,  // x, y, z, w
    @location(5) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

fn rotate_by_quat(pos: vec3<f32>, q: vec4<f32>) -> vec3<f32> {
    // q.xyz is the imaginary part (x, y, z)
    let q_xyz = vec3<f32>(q.x, q.y, q.z);
    let t = 2.0 * cross(q_xyz, pos);
    return pos + q.w * t + cross(q_xyz, t);
}

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    // Rotate the mesh vertex by the quaternion
    let rotated = rotate_by_quat(vertex.position, vertex.rotation);

    // Then apply scale and translation
    let position = rotated * vertex.pos_scale.w + vertex.pos_scale.xyz;

    var out: VertexOutput;
    out.clip_position = mesh_position_local_to_clip(
        get_world_from_local(0u),
        vec4<f32>(position, 1.0)
    );
    out.color = vertex.color;
    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}