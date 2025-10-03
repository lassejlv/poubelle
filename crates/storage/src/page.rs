use crate::types::Row;
use serde::{Deserialize, Serialize};

pub const PAGE_SIZE: usize = 4096;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page {
    pub rows: Vec<Row>,
}

impl Page {
    pub fn new() -> Self {
        Self { rows: Vec::new() }
    }

    pub fn add_row(&mut self, row: Row) {
        self.rows.push(row);
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, bincode::Error> {
        bincode::serialize(self)
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self, bincode::Error> {
        bincode::deserialize(data)
    }
}

impl Default for Page {
    fn default() -> Self {
        Self::new()
    }
}
