use std::collections::hash_map::Entry;
use std::collections::HashMap;
use crate::world::block::Block;
use crate::world::chunk::{Chunk, CHUNK_SIZE};
use crate::world::terrain;

#[derive(Default)]
pub struct World {
    pub chunks: HashMap<(i32, i32), Chunk>,
}

impl World {
    pub fn insert_chunk(&mut self, chunk_x: i32, chunk_z: i32, chunk: Chunk) {
        self.chunks.insert((chunk_x, chunk_z), chunk);
    }

    pub fn chunks_iter(&self) -> impl Iterator<Item = (&(i32, i32), &Chunk)> {
        self.chunks.iter()
    }

    pub fn is_solid_at(&self, x: i32, y: i32, z: i32) -> bool {
        let Ok(local_y) = usize::try_from(y) else {
            return false;
        };

        let chunk_size = CHUNK_SIZE as i32;
        let chunk_coords = (x.div_euclid(chunk_size), z.div_euclid(chunk_size));
        let local_x = x.rem_euclid(chunk_size) as usize;
        let local_z = z.rem_euclid(chunk_size) as usize;

        self.chunks
            .get(&chunk_coords)
            .and_then(|chunk| chunk.get(local_x, local_y, local_z))
            .is_some_and(Block::is_solid)
    }

    pub fn block_at(&self, x: i32, y: i32, z: i32) -> Option<Block> {
        let local_y = usize::try_from(y).ok()?;
        let chunk_size = CHUNK_SIZE as i32;
        let chunk_coords = (x.div_euclid(chunk_size), z.div_euclid(chunk_size));
        let local_x = x.rem_euclid(chunk_size) as usize;
        let local_z = z.rem_euclid(chunk_size) as usize;

        self.chunks
            .get(&chunk_coords)?
            .get(local_x, local_y, local_z)
    }

    pub fn ensure_chunks_around(
        &mut self,
        center_x: i32,
        center_z: i32,
        radius: i32,
        seed: u32,
    ) -> usize {
        assert!(radius >= 0, "chunk radius must not be negative");

        let mut generated = 0;

        for dx in -radius..=radius {
            for dz in -radius..=radius {
                let coords = (
                    center_x.saturating_add(dx),
                    center_z.saturating_add(dz),
                );

                if let Entry::Vacant(entry) = self.chunks.entry(coords) {
                    entry.insert(terrain::generate_chunk(coords.0, coords.1, seed));
                    generated += 1;
                }
            }
        }

        generated
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collision_lookup_crosses_chunk_boundaries() {
        let mut world = World::default();

        let mut right_chunk = Chunk::default();
        right_chunk.set(0, 5, 3, Block::Stone);
        world.insert_chunk(1, 0, right_chunk);

        // World x=16 is local x=0 in chunk x=1.
        assert!(world.is_solid_at(16, 5, 3));
        assert!(!world.is_solid_at(15, 5, 3));
    }

    #[test]
    fn collision_lookup_handles_negative_coordinates() {
        let mut world = World::default();

        let mut left_chunk = Chunk::default();
        left_chunk.set(15, 5, 3, Block::Stone);
        world.insert_chunk(-1, 0, left_chunk);

        // World x=-1 is local x=15 in chunk x=-1.
        assert!(world.is_solid_at(-1, 5, 3));
    }
}
