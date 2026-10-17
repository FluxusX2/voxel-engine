use crate::block::block::BlockId;

pub const CHUNK_SIZE: usize = 16;
pub const CHUNK_HEIGHT: usize = 32;
const CHUNK_VOLUME: usize = CHUNK_SIZE * CHUNK_SIZE * CHUNK_HEIGHT;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chunk {
    blocks: [BlockId; CHUNK_VOLUME],
}

impl Default for Chunk {
    fn default() -> Self {
        Self {
            blocks: [Block::Air; CHUNK_VOLUME]
        }
    }
}

impl Chunk {
    pub fn get(&self, x: usize, y: usize, z: usize) -> Option<Block> {
        let index = Self::index(x, y, z)?;
        Some(self.blocks[index])
    }

    pub fn set(&mut self, x: usize, y:usize, z:usize, block: Block) -> bool {
        let Some(index) = Self::index(x, y, z) else {
            return false;
        };

        self.blocks[index] = block;
        true
    }

    fn index(x: usize, y: usize, z: usize) -> Option<usize> {
        if x >= CHUNK_SIZE || y >= CHUNK_HEIGHT || z >= CHUNK_SIZE {
            return None;
        }
        Some(x + CHUNK_SIZE * (z + CHUNK_SIZE * y))
    }
    
    pub fn is_solid_at(&self, x: i32, y: i32, z: i32) -> bool {
        let (Ok(x), Ok(y), Ok(z)) = (
            usize::try_from(x),
            usize::try_from(y),
            usize::try_from(z)
        ) else { 
            return false;
        };
        
        self.get(x, y, z).is_some_and(Block::is_solid)
    }
}


/**
Tests for Chunks
**/
#[cfg(test)]
mod tests {
    use super::*;
    use super::super::block::Block;

    #[test]
    fn new_chunk_is_all_air() {
        let chunk = Chunk::default();

        assert!(chunk.blocks.iter().all(|&block| block == Block::Air));
    }

    #[test]
    fn set_and_get_voxel() {
        let mut chunk = Chunk::default();

        assert!(chunk.set(2, 3, 4, Block::Stone));

        assert_eq!(chunk.get(2, 3, 4), Some(Block::Stone));
        assert_eq!(chunk.get(3, 3, 4), Some(Block::Air));
    }

    #[test]
    fn accepts_last_valid_coordinate() {
        let mut chunk = Chunk::default();

        assert!(chunk.set(15, 31, 15, Block::Grass));
        assert_eq!(chunk.get(15, 31, 15), Some(Block::Grass));
    }

    #[test]
    fn rejects_coordinates_outside_chunk() {
        let mut chunk = Chunk::default();

        for (x, y, z) in [(16, 0, 0), (0, 32, 0), (0, 0, 16)] {
            assert!(!chunk.set(x, y, z, Block::Stone));
            assert_eq!(chunk.get(x, y, z), None);
        }
    }
}
