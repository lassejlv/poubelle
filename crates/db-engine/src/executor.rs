use parser::{Expr, Statement};
use std::collections::HashMap;
use storage_engine::{ColumnType, Row, Storage, StorageError, Value};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ExecutorError {
    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),
    #[error("Type mismatch for column {0}")]
    TypeMismatch(String),
    #[error("Column count mismatch")]
    ColumnCountMismatch,
}

#[derive(Debug)]
pub enum QueryResult {
    Rows(Vec<Row>),
    Success(String),
}

pub struct Executor<'a> {
    storage: &'a mut Storage,
}

impl<'a> Executor<'a> {
    pub fn new(storage: &'a mut Storage) -> Self {
        Self { storage }
    }

    pub fn execute(&mut self, stmt: Statement) -> Result<QueryResult, ExecutorError> {
        match stmt {
            Statement::Select(select) => {
                let rows = self.storage.scan_table(&select.table)?;

                if select.columns.contains(&"*".to_string()) {
                    return Ok(QueryResult::Rows(rows));
                }

                let filtered: Vec<Row> = rows
                    .into_iter()
                    .map(|row| {
                        let mut new_row = Row::new();
                        for col in &select.columns {
                            if let Some(val) = row.get(col) {
                                new_row.insert(col.clone(), val.clone());
                            }
                        }
                        new_row
                    })
                    .collect();

                Ok(QueryResult::Rows(filtered))
            }
            Statement::Insert(insert) => {
                if insert.columns.len() != insert.values.len() {
                    return Err(ExecutorError::ColumnCountMismatch);
                }

                let meta = self
                    .storage
                    .get_table_meta(&insert.table)
                    .ok_or_else(|| StorageError::TableNotFound(insert.table.clone()))?;

                let mut row = Row::new();
                for (col_name, expr) in insert.columns.iter().zip(insert.values.iter()) {
                    let col_type = meta
                        .columns
                        .get(col_name)
                        .ok_or_else(|| ExecutorError::TypeMismatch(col_name.clone()))?;

                    let value = match (expr, col_type) {
                        (Expr::Int(n), ColumnType::Int) => Value::Int(*n),
                        (Expr::Text(s), ColumnType::Text) => Value::Text(s.clone()),
                        (Expr::Null, _) => Value::Null,
                        _ => return Err(ExecutorError::TypeMismatch(col_name.clone())),
                    };

                    row.insert(col_name.clone(), value);
                }

                self.storage.insert_row(&insert.table, row)?;
                Ok(QueryResult::Success("Row inserted".to_string()))
            }
            Statement::Create(create) => {
                let mut columns = HashMap::new();
                for col in create.columns {
                    let col_type = match col.column_type.as_str() {
                        "INT" => ColumnType::Int,
                        "TEXT" => ColumnType::Text,
                        _ => return Err(ExecutorError::TypeMismatch(col.name)),
                    };
                    columns.insert(col.name, col_type);
                }

                self.storage.create_table(create.name.clone(), columns)?;
                Ok(QueryResult::Success(format!(
                    "Table {} created",
                    create.name
                )))
            }
        }
    }
}
