use bytemuck::{Pod, Zeroable};
use crate::world::block::Block;
use crate::world::chunk::Chunk;
use crate::world::world::World;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct MeshVertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
    pub normal: [f32; 3],
    pub ao: f32,
}

#[derive(Debug, Default)]
pub struct Mesh {
    pub vertices: Vec<MeshVertex>,
    pub indices: Vec<u32>,
}

///            offset     corners        normal
const FACES: [([i32; 3], [[f32; 3]; 4], [f32; 3]); 6] = [
    ([ 1,  0,  0], [[1., 0., 0.], [1., 1., 0.], [1., 1., 1.], [1., 0., 1.]], [ 1.,  0.,  0.]),
    ([-1,  0,  0], [[0., 0., 1.], [0., 1., 1.], [0., 1., 0.], [0., 0., 0.]], [-1.,  0.,  0.]),
    ([ 0,  1,  0], [[0., 1., 1.], [1., 1., 1.], [1., 1., 0.], [0., 1., 0.]], [ 0.,  1.,  0.]),
    ([ 0, -1,  0], [[0., 0., 0.], [1., 0., 0.], [1., 0., 1.], [0., 0., 1.]], [ 0., -1.,  0.]),
    ([ 0,  0,  1], [[1., 0., 1.], [1., 1., 1.], [0., 1., 1.], [0., 0., 1.]], [ 0.,  0.,  1.]),
    ([ 0,  0, -1], [[0., 0., 0.], [0., 1., 0.], [1., 1., 0.], [1., 0., 0.]], [ 0.,  0., -1.]),
];

pub fn build_mesh(chunk: &Chunk) -> Mesh {
    let mut mesh = Mesh::default();

    for z in 0..16 {
        for y in 0..32 {
            for x in 0..16 {
                let Some(block) = chunk.get(x, y, z) else {
                    continue;
                };
                if !block.is_solid() {
                    continue;
                }

                for (neighbor_offset, corners, normal) in FACES {
                    let block_pos = [x as i32, y as i32, z as i32];
                    let neighbor = [
                        block_pos[0] + neighbor_offset[0],
                        block_pos[1] + neighbor_offset[1],
                        block_pos[2] + neighbor_offset[2],
                    ];

                    if is_solid_at(chunk, neighbor) {
                        continue;
                    }

                    let base = mesh.vertices.len() as u32;
                    let color = block_color(block);

                    for corner in corners {
                        mesh.vertices.push(MeshVertex {
                            position: [
                                x as f32 + corner[0],
                                y as f32 + corner[1],
                                z as f32 + corner[2],
                            ],
                            color,
                            normal,
                            ao: face_ao(chunk, block_pos, normal, corner),
                        });
                    }

                    mesh.indices.extend_from_slice(&[
                        base, base + 1, base + 2,
                        base, base + 2, base + 3,
                    ]);
                }
            }
        }
    }

    mesh
}

fn block_color(block: Block) -> [f32; 3] {
    match block { 
        Block::Air => [0.0, 0.0, 0.0],
        Block::Grass => [0.25, 0.75, 0.2],
        Block::Dirt => [0.45, 0.28, 0.12],
        Block::Stone => [0.55, 0.57, 0.6],
    }
}

fn is_solid_at(chunk: &Chunk, pos: [i32; 3]) -> bool {
    if pos.iter().any(|&coord| coord < 0) {
        return false;
    }
    let [x, y, z] = pos;
    chunk.get(x as usize, y as usize, z as usize).is_some_and(Block::is_solid)
}

pub fn build_world_mesh(world: &World) -> Mesh {
    let mut world_mesh = Mesh::default();

    world.chunks_iter().for_each(|((chunk_x, chunk_z), chunk)| {
        let mut chunk_mesh = build_mesh(chunk);
        let vertex_offset = world_mesh.vertices.len() as u32;

        for vertex in &mut chunk_mesh.vertices {
            vertex.position[0] += (*chunk_x * 16) as f32;
            vertex.position[2] += (*chunk_z * 16) as f32;
        }

        world_mesh.vertices.extend(chunk_mesh.vertices);
        world_mesh.indices.extend(
            chunk_mesh.indices.into_iter().map(|index| index + vertex_offset),
        );
    });

    world_mesh
}

fn face_ao(
    chunk: &Chunk,
    block_pos: [i32; 3],
    normal: [f32; 3],
    corner: [f32; 3],
) -> f32 {
    let tangent_axes = match normal {
        [1.0, 0.0, 0.0] | [-1.0, 0.0, 0.0] => [1, 2],
        [0.0, 1.0, 0.0] | [0.0, -1.0, 0.0] => [0, 2],
        _ => [0, 1],
    };

    let sign_a = if corner[tangent_axes[0]] < 0.5 { -1 } else { 1 };
    let sign_b = if corner[tangent_axes[1]] < 0.5 { -1 } else { 1 };

    let mut side_a = [
        block_pos[0] + normal[0] as i32,
        block_pos[1] + normal[1] as i32,
        block_pos[2] + normal[2] as i32,
    ];
    side_a[tangent_axes[0]] += sign_a;

    let mut side_b = [
        block_pos[0] + normal[0] as i32,
        block_pos[1] + normal[1] as i32,
        block_pos[2] + normal[2] as i32,
    ];
    side_b[tangent_axes[1]] += sign_b;

    let mut corner_pos = [
        block_pos[0] + normal[0] as i32,
        block_pos[1] + normal[1] as i32,
        block_pos[2] + normal[2] as i32,
    ];
    corner_pos[tangent_axes[0]] += sign_a;
    corner_pos[tangent_axes[1]] += sign_b;

    let side_a_solid = is_solid_at(chunk, side_a);
    let side_b_solid = is_solid_at(chunk, side_b);
    let corner_solid = is_solid_at(chunk, corner_pos);

    let occlusion = if side_a_solid && side_b_solid {
        3
    } else {
        i32::from(side_a_solid) + i32::from(side_b_solid) + i32::from(corner_solid)
    };

    1.0 - occlusion as f32 * 0.15
}
