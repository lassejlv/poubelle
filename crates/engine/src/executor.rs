use parser::{ArithmeticOp, CompareOp, Expr, OutputFormat, Statement, WhereClause};
use std::collections::HashMap;
use storage::{ColumnType, Row, Storage, StorageError, Value};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ExecutorError {
    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),
    #[error("Type mismatch for column {0}")]
    TypeMismatch(String),
    #[error("Column count mismatch")]
    ColumnCountMismatch,
    #[error("Division by zero")]
    DivisionByZero,
    #[error("Invalid operation: cannot perform arithmetic on non-integer values")]
    InvalidArithmetic,
}

#[derive(Debug)]
pub enum QueryResult {
    Rows(Vec<Row>),
    RowsJson(String),
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
            Statement::Drop(drop) => {
                self.storage.drop_table(&drop.name)?;
                Ok(QueryResult::Success(format!("Table {} dropped", drop.name)))
            }
            Statement::SelectExpr(select_expr) => {
                // Evaluate each expression and build a result row
                let mut row = Row::new();
                for (i, item) in select_expr.expressions.iter().enumerate() {
                    let value = self.evaluate_expr(&item.expr, None)?;
                    let col_name = item
                        .alias
                        .clone()
                        .unwrap_or_else(|| format!("?column{}", i + 1));
                    row.insert(col_name, value);
                }

                match select_expr.format {
                    OutputFormat::Json => {
                        let mut map = serde_json::Map::new();
                        for (key, value) in &row.data {
                            let json_val = match value {
                                Value::Int(i) => serde_json::Value::Number((*i).into()),
                                Value::Text(s) => serde_json::Value::String(s.clone()),
                                Value::Null => serde_json::Value::Null,
                            };
                            map.insert(key.clone(), json_val);
                        }
                        let json_string = serde_json::to_string_pretty(&[serde_json::Value::Object(map)])
                            .unwrap_or_else(|_| "[]".to_string());
                        Ok(QueryResult::RowsJson(json_string))
                    }
                    OutputFormat::Debug => Ok(QueryResult::Rows(vec![row])),
                }
            }
            Statement::Select(select) => {
                let mut rows = self.storage.scan_table(&select.table)?;

                if let Some(where_clause) = &select.where_clause {
                    rows = rows
                        .into_iter()
                        .filter(|row| self.evaluate_where(row, where_clause))
                        .collect();
                }

                if let Some(limit) = select.limit {
                    rows.truncate(limit);
                }

                let result_rows = if select.columns.contains(&"*".to_string()) {
                    rows
                } else {
                    rows.into_iter()
                        .map(|row| {
                            let mut new_row = Row::new();
                            for col in &select.columns {
                                if let Some(val) = row.get(col) {
                                    new_row.insert(col.clone(), val.clone());
                                }
                            }
                            new_row
                        })
                        .collect()
                };

                match select.format {
                    OutputFormat::Json => {
                        let json_rows: Vec<serde_json::Value> = result_rows
                            .iter()
                            .map(|row| {
                                let mut map = serde_json::Map::new();
                                for (key, value) in &row.data {
                                    let json_val = match value {
                                        Value::Int(i) => serde_json::Value::Number((*i).into()),
                                        Value::Text(s) => serde_json::Value::String(s.clone()),
                                        Value::Null => serde_json::Value::Null,
                                    };
                                    map.insert(key.clone(), json_val);
                                }
                                serde_json::Value::Object(map)
                            })
                            .collect();

                        let json_string = serde_json::to_string_pretty(&json_rows)
                            .unwrap_or_else(|_| "[]".to_string());
                        Ok(QueryResult::RowsJson(json_string))
                    }
                    OutputFormat::Debug => Ok(QueryResult::Rows(result_rows)),
                }
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

    fn evaluate_where(&self, row: &Row, where_clause: &WhereClause) -> bool {
        let row_value = match row.get(&where_clause.column) {
            Some(v) => v,
            None => return false,
        };

        let compare_value = match &where_clause.value {
            Expr::Int(n) => Value::Int(*n),
            Expr::Text(s) => Value::Text(s.clone()),
            Expr::Null => Value::Null,
            Expr::Column(_) | Expr::BinaryOp { .. } => return false, // Not supported in WHERE yet
        };

        match where_clause.operator {
            CompareOp::Equal => row_value == &compare_value,
            CompareOp::NotEqual => row_value != &compare_value,
            CompareOp::LessThan => match (row_value, &compare_value) {
                (Value::Int(a), Value::Int(b)) => a < b,
                _ => false,
            },
            CompareOp::LessThanOrEqual => match (row_value, &compare_value) {
                (Value::Int(a), Value::Int(b)) => a <= b,
                _ => false,
            },
            CompareOp::GreaterThan => match (row_value, &compare_value) {
                (Value::Int(a), Value::Int(b)) => a > b,
                _ => false,
            },
            CompareOp::GreaterThanOrEqual => match (row_value, &compare_value) {
                (Value::Int(a), Value::Int(b)) => a >= b,
                _ => false,
            },
        }
    }

    /// Evaluate an expression, optionally with a row context for column references
    fn evaluate_expr(&self, expr: &Expr, row: Option<&Row>) -> Result<Value, ExecutorError> {
        match expr {
            Expr::Int(n) => Ok(Value::Int(*n)),
            Expr::Text(s) => Ok(Value::Text(s.clone())),
            Expr::Null => Ok(Value::Null),
            Expr::Column(name) => {
                if let Some(row) = row {
                    row.get(name)
                        .cloned()
                        .ok_or_else(|| ExecutorError::TypeMismatch(name.clone()))
                } else {
                    // No row context - column reference in expression-only SELECT
                    Err(ExecutorError::TypeMismatch(format!(
                        "column '{}' does not exist",
                        name
                    )))
                }
            }
            Expr::BinaryOp { left, op, right } => {
                let left_val = self.evaluate_expr(left, row)?;
                let right_val = self.evaluate_expr(right, row)?;

                match (left_val, right_val) {
                    (Value::Int(a), Value::Int(b)) => {
                        let result = match op {
                            ArithmeticOp::Add => a + b,
                            ArithmeticOp::Subtract => a - b,
                            ArithmeticOp::Multiply => a * b,
                            ArithmeticOp::Divide => {
                                if b == 0 {
                                    return Err(ExecutorError::DivisionByZero);
                                }
                                a / b
                            }
                        };
                        Ok(Value::Int(result))
                    }
                    _ => Err(ExecutorError::InvalidArithmetic),
                }
            }
        }
    }
}
