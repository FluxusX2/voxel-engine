use std::collections::HashMap;
use crate::block::block_builder::BlockBuilder;
use crate::block::block_property::Property;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockId(pub u16);

#[derive(Debug, Clone)]
pub struct BlockDefinition {
    pub id: BlockId,
    pub name: String,

    // --- Mandatory Properties ---
    pub is_solid: bool,
    pub texture_path: String,

    // --- Optional Properties ---
    pub custom_properties: HashMap<String, Property>,
}

pub struct BlockRegistry {
    pub(crate) definitions: Vec<BlockDefinition>,
    pub(crate) name_to_id: HashMap<String, BlockId>,
}

impl Default for BlockRegistry {
    fn default() -> Self {
        let mut registry = Self {
            definitions: Vec::new(),
            name_to_id: HashMap::new(),
        };

        // Default Air
        registry.register_builder("Air", "textures/empty.png")
            .is_solid(false)
            .build();

        registry
    }
}

impl BlockRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            definitions: Vec::new(),
            name_to_id: HashMap::new(),
        };

        // Default Air
        registry.register_builder("Air", "textures/empty.png")
            .is_solid(false)
            .build();

        registry
    }

    /// Gibt uns einen Builder, um einen neuen Block zu erstellen
    pub fn register_builder(&mut self, name: &str, texture_path: &str) -> BlockBuilder {
        BlockBuilder {
            registry: self,
            name: name.to_string(),
            texture_path: texture_path.to_string(),
            is_solid: true, // Standardwert
            custom_properties: HashMap::new(),
        }
    }

    pub fn get_by_id(&self, id: BlockId) -> Option<&BlockDefinition> {
        self.definitions.get(id.0 as usize)
    }

    pub fn get_by_name(&self, name: &str) -> Option<&BlockDefinition> {
        let id = self.name_to_id.get(name)?;
        self.get_by_id(*id)
    }
}
