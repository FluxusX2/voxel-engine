use std::collections::HashMap;
use crate::block::block::{BlockDefinition, BlockId, BlockRegistry};
use crate::block::block_property::Property;

// 5. Das Builder-Pattern macht das Registrieren sicher und lesbar
pub struct BlockBuilder<'a> {
    pub(crate) registry: &'a mut BlockRegistry,
    pub(crate) name: String,
    pub(crate) texture_path: String,
    pub(crate) is_solid: bool,
    pub(crate) custom_properties: HashMap<String, Property>,
}

impl<'a> BlockBuilder<'a> {
    pub fn is_solid(mut self, solid: bool) -> Self {
        self.is_solid = solid;
        self
    }

    pub fn with_property(mut self, key: &str, value: Property) -> Self {
        self.custom_properties.insert(key.to_string(), value);
        self
    }

    pub fn build(self) -> BlockId {
        let id = BlockId(self.registry.definitions.len() as u16);

        let definition = BlockDefinition {
            id,
            name: self.name.clone(),
            is_solid: self.is_solid,
            texture_path: self.texture_path,
            custom_properties: self.custom_properties,
        };

        self.registry.definitions.push(definition);
        self.registry.name_to_id.insert(self.name, id);

        id
    }
}