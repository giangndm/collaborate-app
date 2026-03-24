#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldKind {
    String,
    Counter,
    Text,
    List,
    Map,
    Object,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldSchema {
    pub name: String,
    pub kind: FieldKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StateSchema {
    pub fields: Vec<FieldSchema>,
}

impl StateSchema {
    pub fn new(fields: Vec<FieldSchema>) -> Self {
        Self { fields }
    }
}
