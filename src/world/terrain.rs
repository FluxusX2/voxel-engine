use noise::{NoiseFn, Perlin};
use crate::world::block::Block;
use crate::world::chunk::{Chunk, CHUNK_HEIGHT, CHUNK_SIZE};

const BASE_HEIGHT: f64 = 14.0;
const HEIGHT_VARIATION: f64 = 7.0;
const TERRAIN_SCALE: f64 = 20.0;
const DIRT_DEPTH: i32 = 3;
const CAVE_SCALE: f64 = 12.0;
const CAVE_THRESHOLD: f64 = 0.1; // Value < 1
const ENTRANCE_SCALE: f64 = 24.0;
const ENTRANCE_THRESHOLD: f64 = 0.45;
const ENTRANCE_DEPTH: i32 = 10;

pub fn generate_chunk(chunk_x: i32, chunk_z: i32, seed: u32) -> Chunk {
    let perlin = Perlin::new(seed);
    let cave_noise = Perlin::new(seed.wrapping_add(1));
    let entrance_noise = Perlin::new(seed.wrapping_add(2));
    let mut chunk = Chunk::default();
    
    for z in 0..CHUNK_SIZE {
        for x in 0..CHUNK_SIZE {
            let world_x = chunk_x * CHUNK_SIZE as i32 + x as i32;
            let world_z = chunk_z * CHUNK_SIZE as i32 + z as i32;
            
            let noise = perlin.get([
                world_x as f64 / TERRAIN_SCALE,
                world_z as f64 / TERRAIN_SCALE,
            ]);
            
            let height = (BASE_HEIGHT + noise * HEIGHT_VARIATION)
                .round()
                .clamp(1.0, (CHUNK_HEIGHT - 1) as f64) as i32;
            
            for y in 0..=height {
                let depth = height - y;

                let opens_to_surface = depth <= ENTRANCE_DEPTH
                    && entrance_noise.get([
                    world_x as f64 / ENTRANCE_SCALE,
                    world_z as f64 / ENTRANCE_SCALE,
                ]) > ENTRANCE_THRESHOLD;

                let in_cave = opens_to_surface
                    || (depth > DIRT_DEPTH + 1
                    && cave_noise
                    .get([
                        world_x as f64 / CAVE_SCALE,
                        y as f64 / CAVE_SCALE,
                        world_z as f64 / CAVE_SCALE,
                    ])
                    .abs()
                    < CAVE_THRESHOLD);

                if in_cave {
                    continue;
                }

                let block = if depth == 0 {
                    Block::Grass
                } else if depth <= DIRT_DEPTH {
                    Block::Dirt
                } else {
                    Block::Stone
                };
                chunk.set(x, y as usize, z, block);
            }
        }
    }

    for z in 0..CHUNK_SIZE {
        for x in 0..CHUNK_SIZE {
            chunk.set(x, 0, z, Block::Stone);
        }
    }

    chunk
}