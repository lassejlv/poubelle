use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Value {
    Int(i64),
    Text(String),
    Null,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ColumnType {
    Int,
    Text,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Row {
    pub data: HashMap<String, Value>,
}

impl Row {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    pub fn insert(&mut self, column: String, value: Value) {
        self.data.insert(column, value);
    }

    pub fn get(&self, column: &str) -> Option<&Value> {
        self.data.get(column)
    }
}

impl Default for Row {
    fn default() -> Self {
        Self::new()
    }
}
