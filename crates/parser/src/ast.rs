#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Select(SelectQuery),
    Insert(InsertStatement),
    Create(CreateTable),
    Drop(DropTable),
}

#[derive(Debug, Clone, PartialEq)]
pub struct DropTable {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SelectQuery {
    pub columns: Vec<String>,
    pub table: String,
    pub where_clause: Option<WhereClause>,
    pub limit: Option<usize>,
    pub format: OutputFormat,
}

#[derive(Debug, Clone, PartialEq)]
pub enum OutputFormat {
    Debug,
    Json,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WhereClause {
    pub column: String,
    pub operator: CompareOp,
    pub value: Expr,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CompareOp {
    Equal,
    NotEqual,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CreateTable {
    pub name: String,
    pub columns: Vec<Column>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Column {
    pub name: String,
    pub column_type: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InsertStatement {
    pub table: String,
    pub columns: Vec<String>,
    pub values: Vec<Expr>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Int(i64),
    Text(String),
    Null,
}
