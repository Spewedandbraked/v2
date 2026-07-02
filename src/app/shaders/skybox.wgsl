struct CameraUniform {
    view_proj: mat4x4<f32>,
    view: mat4x4<f32>,
    inv_view: mat4x4<f32>,
    aspect: f32,
    fov: f32,
}

@group(0) @binding(0) var<uniform> camera: CameraUniform;

@group(1) @binding(0) var skybox_texture: texture_cube<f32>;
@group(1) @binding(1) var skybox_sampler: sampler;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_dir: vec3<f32>,
}

@vertex
fn vs_main(@location(0) position: vec3<f32>) -> VertexOutput {
    var out: VertexOutput;
    
    var vp = camera.view_proj;
    vp[3] = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    
    let scale = tan(camera.fov * 0.5 * 3.14159 / 180.0);
    let pos = position * scale * 2.0;
    
    let clip = vp * vec4<f32>(pos, 1.0);
    out.clip_position = clip.xyww;
    out.world_dir = position;
    
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(skybox_texture, skybox_sampler, normalize(in.world_dir));
}

