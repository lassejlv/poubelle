use crate::executor::{Executor, ExecutorError, QueryResult};
use parser::{ParseError, Parser};
use storage::Storage;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum EngineError {
    #[error("Parse error: {0}")]
    Parse(#[from] ParseError),
    #[error("Execution error: {0}")]
    Execution(#[from] ExecutorError),
}

pub struct Engine {
    storage: Storage,
}

impl Engine {
    pub fn new(storage: Storage) -> Self {
        Self { storage }
    }

    pub fn execute_query(&mut self, query: &str) -> Result<QueryResult, EngineError> {
        let mut parser = Parser::new(query);
        let statement = parser.parse()?;

        let mut executor = Executor::new(&mut self.storage);
        let result = executor.execute(statement)?;

        Ok(result)
    }

    pub fn list_tables(&self) -> Vec<String> {
        self.storage.list_tables()
    }
}
