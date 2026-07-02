struct CameraUniform {
    view_proj: mat4x4<f32>,
    view: mat4x4<f32>,
    aspect: f32,
}

struct Light {
    position: vec3<f32>,
    color: vec3<f32>,
    intensity: f32,
    radius: f32,
}

@group(0) @binding(0) var<uniform> camera: CameraUniform;
@group(0) @binding(1) var<storage> lights: array<Light, 256>;
@group(0) @binding(2) var<storage, read_write> light_grid: array<u32>;
@group(0) @binding(3) var<storage, read_write> light_index_count: array<u32>;

override TILE_SIZE: u32 = 16u;
override MAX_LIGHTS_PER_TILE: u32 = 64u;

@compute @workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>,
        @builtin(num_workgroups) num_groups: vec3<u32>) {
    let tile_x = global_id.x;
    let tile_y = global_id.y;
    let tile_index = tile_y * num_groups.x + tile_x;
    
    let base = tile_index * MAX_LIGHTS_PER_TILE;
    var count: u32 = 0u;
    
    let tile_min_x = f32(tile_x * TILE_SIZE);
    let tile_min_y = f32(tile_y * TILE_SIZE);
    let tile_max_x = f32((tile_x + 1u) * TILE_SIZE);
    let tile_max_y = f32((tile_y + 1u) * TILE_SIZE);
    
    for (var i: u32 = 0u; i < 256u; i++) {
        let light = lights[i];
        if (light.intensity == 0.0) { continue; }
        
        let in_tile = true;
        
        if (in_tile && count < MAX_LIGHTS_PER_TILE) {
            light_grid[base + count] = i;
            count++;
        }
    }
    
    light_index_count[tile_index] = count;
}