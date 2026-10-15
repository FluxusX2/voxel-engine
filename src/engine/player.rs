use glam::Vec3;
use crate::world::chunk::{Chunk, CHUNK_HEIGHT, CHUNK_SIZE};
use crate::world::world::World;
use crate::utilities::spiral::Spiral;

const RADIUS: f32 = 0.3;
const HEIGHT: f32 = 1.8;
const WALK_SPEED: f32 = 5.0;
const GRAVITY: f32 = 24.0;
const JUMP_SPEED: f32 = 8.0;
const COLLISION_STEP: f32 = 0.2;
const EPS: f32 = 0.00001;

pub struct Player {
    pub pos: Vec3,
    velocity_y: f32,
    grounded: bool,
}

impl Default for Player {
    fn default() -> Self {
        Self {
            pos: Vec3::new(5.0, 20.0, 5.0),
            velocity_y: 0.0,
            grounded: false,
        }
    }
}

impl Player {
    pub fn update(&mut self, world: &World, movement: Vec3, jump: bool, dt: f32) {
        let movement = movement.normalize_or_zero();
        self.move_axis(world, Vec3::X, movement.x * WALK_SPEED * dt);
        self.move_axis(world, Vec3::Z, movement.z * WALK_SPEED * dt);

        if jump && self.grounded {
            self.velocity_y = JUMP_SPEED;
            self.grounded = false;
        }

        self.velocity_y -= GRAVITY * dt;
        self.grounded = false;

        if self.move_axis(world, Vec3::Y, self.velocity_y * dt) {
            if self.velocity_y < 0.0 {
                self.grounded = true;
            }
            self.velocity_y = 0.0;
        }
    }

    pub fn eye_pos(&self) -> Vec3 {
        self.pos + Vec3::Y * HEIGHT * 0.9
    }

    fn move_axis(&mut self, world: &World, axis: Vec3, distance: f32) -> bool {
        if distance == 0.0 {
            return false;
        }

        let steps = ((distance.abs() / COLLISION_STEP).ceil() as u32).max(1);
        let step = distance / steps as f32;

        for _ in 0..steps {
            let start = self.pos;
            self.pos += axis * step;

            if self.collides(world) {
                let mut safe = 0.0;
                let mut blocked = 1.0;

                for _ in 0..24 {
                    let mid = (safe + blocked) * 0.5;
                    self.pos = start + axis * (step * mid);

                    if self.collides(world) {
                        blocked = mid;
                    } else {
                        safe = mid;
                    }
                }

                self.pos = start + axis * (step * safe);
                return true;
            }
        }

        false
    }

    fn collides(&self, world: &World) -> bool {
        let min_x = (self.pos.x - RADIUS).floor() as i32;
        let max_x = (self.pos.x + RADIUS - EPS).floor() as i32;
        let min_y = self.pos.y.floor() as i32;
        let max_y = (self.pos.y + HEIGHT - EPS).floor() as i32;
        let min_z = (self.pos.z - RADIUS).floor() as i32;
        let max_z = (self.pos.z + RADIUS - EPS).floor() as i32;

        (min_x..=max_x).any(|x| {
            (min_y..=max_y).any(|y| {
                (min_z..=max_z).any(|z| world.is_solid_at(x, y, z))
            })
        })
    }

    pub fn find_spawn_coords(world: &World) -> Option<Vec3> {
        println!("Searching suitable spawn...)");
        let max_chunk_radius = world
            .chunks
            .keys()
            .map(|&(chunk_x, chunk_z)| chunk_x.unsigned_abs().max(chunk_z.unsigned_abs()))
            .max()?;

        for (chunk_x, chunk_z) in Spiral::new() {
            if chunk_x.unsigned_abs().max(chunk_z.unsigned_abs()) > max_chunk_radius {
                eprintln!("Error occurred while generating chunks.");
                return None;
            }

            let Some(chunk) = world.chunks.get(&(chunk_x, chunk_z)) else {
                continue;
            };

            for local_x in 0..CHUNK_SIZE {
                for local_z in 0..CHUNK_SIZE {
                    for ground_y in (0..(CHUNK_HEIGHT - 2)).rev() {
                        let has_ground = chunk
                            .get(local_x, ground_y, local_z)
                            .is_some_and(|block| block.is_solid());
                        let feet_clear = chunk
                            .get(local_x, ground_y + 1, local_z)
                            .is_some_and(|block| !block.is_solid());
                        let head_clear = chunk
                            .get(local_x, ground_y + 2, local_z)
                            .is_some_and(|block| !block.is_solid());

                        if has_ground && feet_clear && head_clear {
                            let world_x = chunk_x * CHUNK_SIZE as i32 + local_x as i32;
                            let world_z = chunk_z * CHUNK_SIZE as i32 + local_z as i32;

                            println!("Found suitable spawn at coordinates: (x: {world_x}, y: {ground_y}, z: {world_z})");
                            return Some(Vec3::new(
                                world_x as f32 + 0.5,
                                (ground_y + 1) as f32,
                                world_z as f32 + 0.5,
                            ));
                        }
                    }
                }
            }
        }

        eprintln!("Error occurred: No suitable spawn found.");
        None
    }
}