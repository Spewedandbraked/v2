// Все структуры в начале
struct CameraUniform {
    view_proj: mat4x4<f32>,
}

struct ScreenUniform {
    width: f32,
    height: f32,
}

struct Light {
    position: vec3<f32>,
    color: vec3<f32>,
    intensity: f32,
    radius: f32,
}

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) world_position: vec3<f32>,
}

@group(0) @binding(0) var<uniform> camera: CameraUniform;
@group(1) @binding(0) var t_diffuse: texture_2d<f32>;
@group(1) @binding(1) var s_diffuse: sampler;
@group(2) @binding(0) var<storage> light_grid: array<u32>;
@group(2) @binding(1) var<storage> light_index_count: array<u32>;
@group(2) @binding(2) var<storage> lights: array<Light, 256>;
@group(2) @binding(3) var<uniform> screen: ScreenUniform;

const TILE_SIZE: u32 = 16u;
const MAX_LIGHTS_PER_TILE: u32 = 64u;

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.tex_coords = model.tex_coords;
    out.world_position = model.position;
    out.clip_position = camera.view_proj * vec4<f32>(model.position, 1.0);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let albedo = textureSample(t_diffuse, s_diffuse, in.tex_coords).rgb;
    
    let num_tiles_x = u32(ceil(screen.width / f32(TILE_SIZE)));
    let tile_x = u32(floor(floor(in.clip_position.x) / f32(TILE_SIZE)));
    let tile_y = u32(floor(floor(in.clip_position.y) / f32(TILE_SIZE)));
    let tile_index = tile_y * num_tiles_x + tile_x;
    
    let base = tile_index * MAX_LIGHTS_PER_TILE;
    let count = light_index_count[tile_index];
    
    var lighting = vec3<f32>(0.05, 0.05, 0.05);
    
    for (var i: u32 = 0u; i < count; i++) {
        let light_idx = light_grid[base + i];
        let light = lights[light_idx];
        
        let dist = distance(in.world_position, light.position);
        if (dist <= light.radius) {
            lighting += light.color * light.intensity;
        }
    }
    
    return vec4<f32>(albedo * lighting, 1.0);
}