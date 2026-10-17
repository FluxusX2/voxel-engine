#[derive(Debug, Clone, PartialEq)]
pub enum Property {
    Bool(bool),
    Int(i32),
    Float(f32),
    String(String),
}