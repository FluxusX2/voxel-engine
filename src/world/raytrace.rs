use std::io;
use std::path::Path;
use glam::Vec3;
use crate::world::block::Block;
use crate::world::world::World;

pub struct RayHit {
    pub voxel: [i32; 3],
    pub position: Vec3,
    pub normal: Vec3,
    pub distance: f32,
}

pub fn trace_ray(world: &World, origin: Vec3, direction: Vec3, max_distance: f32) -> Option<RayHit> {
    if !origin.is_finite()
        || !direction.is_finite()
        || direction.length_squared() <= f32::EPSILON
        || !max_distance.is_finite()
        || max_distance < 0.0 {
        return None;
    }

    let dir = direction.normalize();
    let origin_components = origin.to_array();
    let dir_components = dir.to_array();

    let mut voxel = [
        origin.x.floor() as i32,
        origin.y.floor() as i32,
        origin.z.floor() as i32,
    ];

    if world.is_solid_at(voxel[0], voxel[1], voxel[2]) {
        return Some(RayHit {
            voxel,
            position: origin,
            normal: Vec3::ZERO,
            distance: 0.0,
        });
    }

    let step = dir_components.map(|d| {
        if d > 0.0 { 1 } else if d < 0.0 { -1 } else { 0 }
    });

    let delta = dir_components.map(|d| {
        if d == 0.0 { f32::INFINITY } else { 1.0 / d.abs() }
    });

    let mut next_boundary = [0.0; 3];
    for axis in 0..3 {
        next_boundary[axis] = if step[axis] > 0 {
            (voxel[axis] as f32 + 1.0 - origin_components[axis]) / dir_components[axis]
        } else if step[axis] < 0 {
            (origin_components[axis] - voxel[axis] as f32) / -dir_components[axis]
        } else {
            f32::INFINITY
        }
    }

    loop {
        let axis =
            if next_boundary[0] <= next_boundary[1]
            && next_boundary[0] <= next_boundary[2]
        {
            0
        } else if next_boundary[1] <= next_boundary[2] {
            1
        } else {
            2
        };

        let distance = next_boundary[axis];
        if distance > max_distance {
            return None;
        }

        voxel[axis] += step[axis];
        next_boundary[axis] += delta[axis];

        let normal = match axis {
            0 => Vec3::new(-(step[0] as f32), 0.0, 0.0),
            1 => Vec3::new(0.0, -(step[1] as f32), 0.0),
            _ => Vec3::new(0.0, 0.0, -(step[2] as f32)),
        };

        if world.is_solid_at(voxel[0], voxel[1], voxel[2]) {
            return Some(RayHit {
                voxel,
                position: origin + dir * distance,
                normal,
                distance,
            });
        }
    }
}

pub fn shade_hit(world: &World, hit: RayHit, albedo: Vec3, sun_direction: Vec3, ) -> Vec3 {
    let sun_direction = sun_direction.normalize_or_zero();
    let diffuse = hit.normal.dot(sun_direction).max(0.0);

    let shadowed = diffuse > 0.0
        && trace_ray(
        world,
        hit.position + hit.normal * 0.001,
        sun_direction,
        100.0,
    )
        .is_some();

    let direct_light = if shadowed { 0.0 } else { diffuse };
    albedo * (0.12 + 0.88 * direct_light)
}

pub fn render_preview(
    world: &World,
    camera_position: Vec3,
    camera_direction: Vec3,
    width: u32,
    height: u32,
    output_path: impl AsRef<Path>,
) -> io::Result<()> {
    
    if width == 0 || height == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "preview dimensions must be non-zero",
        ));
    }

    let forward = camera_direction.normalize_or_zero();
    if forward == Vec3::ZERO {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "camera direction must be non-zero",
        ));
    }

    let right = forward.cross(Vec3::Y).normalize_or_zero();
    let up = right.cross(forward).normalize_or_zero();
    let aspect_ratio = width as f32 / height as f32;
    let half_fov = 45.0_f32.to_radians().tan();

    let mut ppm = format!("P6\n{width} {height}\n255\n").into_bytes();
    ppm.reserve((width * height * 3) as usize);

    for y in 0..height {
        for x in 0..width {
            let screen_x =
                (2.0 * (x as f32 + 0.5) / width as f32 - 1.0) * aspect_ratio * half_fov;
            let screen_y =
                (1.0 - 2.0 * (y as f32 + 0.5) / height as f32) * half_fov;
            let ray_direction =
                (forward + right * screen_x + up * screen_y).normalize();

            let color = match trace_ray(world, camera_position, ray_direction, 100.0) {
                Some(hit) => {
                    let block = world.block_at(hit.voxel[0], hit.voxel[1], hit.voxel[2])
                        .unwrap_or(Block::Stone);
                    let albedo = match block {
                        Block::Grass => Vec3::new(0.25, 0.75, 0.2),
                        Block::Dirt => Vec3::new(0.45, 0.28, 0.12),
                        Block::Stone => Vec3::new(0.55, 0.57, 0.6),
                        Block::Air => Vec3::ZERO,
                    };
                    shade_hit(world, hit, albedo, Vec3::new(-0.4, 0.8, -0.3))
                }
                None => Vec3::new(0.45, 0.7, 1.0),
            };

            for channel in color.to_array() {
                ppm.push((channel.clamp(0.0, 1.0) * 255.0) as u8);
            }
        }
    }

    std::fs::write(output_path, ppm)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::{block::Block, chunk::Chunk, world::World};

    #[test]
    fn hits_block_and_reports_world_distance() {
        let mut world = World::default();
        let mut chunk = Chunk::default();
        chunk.set(2, 1, 0, Block::Stone);
        world.insert_chunk(0, 0, chunk);

        // Non-unit direction verifies distances are measured in world units.
        let hit = trace_ray(
            &world,
            Vec3::new(0.5, 1.5, 0.5),
            Vec3::new(2.0, 0.0, 0.0),
            10.0,
        )
            .expect("ray should hit the stone block");

        assert_eq!(hit.voxel, [2, 1, 0]);
        assert_eq!(hit.distance, 1.5);
        assert_eq!(hit.position, Vec3::new(2.0, 1.5, 0.5));
        assert_eq!(hit.normal, Vec3::NEG_X);
    }

    #[test]
    fn returns_none_when_block_is_beyond_max_distance() {
        let mut world = World::default();
        let mut chunk = Chunk::default();
        chunk.set(2, 1, 0, Block::Stone);
        world.insert_chunk(0, 0, chunk);

        let hit = trace_ray(
            &world,
            Vec3::new(0.5, 1.5, 0.5),
            Vec3::X,
            1.49,
        );

        assert!(hit.is_none());
    }

    #[test]
    fn reports_zero_distance_when_ray_starts_inside_block() {
        let mut world = World::default();
        let mut chunk = Chunk::default();
        chunk.set(1, 1, 1, Block::Stone);
        world.insert_chunk(0, 0, chunk);

        let hit = trace_ray(
            &world,
            Vec3::new(1.5, 1.5, 1.5),
            Vec3::X,
            10.0,
        )
            .expect("ray starts inside a solid block");

        assert_eq!(hit.voxel, [1, 1, 1]);
        assert_eq!(hit.distance, 0.0);
        assert_eq!(hit.position, Vec3::new(1.5, 1.5, 1.5));
        assert_eq!(hit.normal, Vec3::ZERO);
    }

    #[test]
    fn ray_can_hit_block_across_chunk_boundary() {
        let mut world = World::default();
        let mut next_chunk = Chunk::default();
        next_chunk.set(0, 1, 0, Block::Stone);
        world.insert_chunk(1, 0, next_chunk);

        let hit = trace_ray(
            &world,
            Vec3::new(15.5, 1.5, 0.5),
            Vec3::X,
            10.0,
        )
            .expect("ray should hit the next chunk");

        assert_eq!(hit.voxel, [16, 1, 0]);
        assert_eq!(hit.distance, 0.5);
        assert_eq!(hit.normal, Vec3::NEG_X);
    }
}
