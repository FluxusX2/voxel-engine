#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[repr(u8)]
pub enum Block {
    #[default]
    Air,
    Grass,
    Dirt,
    Stone,
}

impl Block {
    pub const fn is_solid(self) -> bool {
        !matches!(self, Self::Air)
    }
}