use crate::ast::{
    ArithmeticOp, Column, CompareOp, CreateTable, DropTable, Expr, InsertStatement, OutputFormat,
    SelectExprQuery, SelectItem, SelectQuery, Statement, WhereClause,
};
use crate::lexer::{Lexer, Token};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Unexpected token: {0:?}")]
    UnexpectedToken(Token),
    #[error("Expected token: {0}")]
    ExpectedToken(String),
}

pub struct Parser {
    lexer: Lexer,
    current: Token,
}

impl Parser {
    pub fn new(input: &str) -> Self {
        let mut lexer = Lexer::new(input);
        let current = lexer.next_token();
        Self { lexer, current }
    }

    pub fn parse(&mut self) -> Result<Statement, ParseError> {
        match &self.current {
            Token::Select => self.parse_select(),
            Token::Insert => self.parse_insert(),
            Token::Create => self.parse_create(),
            Token::Drop => self.parse_drop(),
            tok => Err(ParseError::UnexpectedToken(tok.clone())),
        }
    }

    fn advance(&mut self) {
        self.current = self.lexer.next_token();
    }

    fn expect(&mut self, expected: Token) -> Result<(), ParseError> {
        if std::mem::discriminant(&self.current) == std::mem::discriminant(&expected) {
            self.advance();
            Ok(())
        } else {
            Err(ParseError::ExpectedToken(format!("{:?}", expected)))
        }
    }

    fn parse_select(&mut self) -> Result<Statement, ParseError> {
        self.expect(Token::Select)?;

        // Check if this is a simple column SELECT (with FROM) or an expression SELECT
        // Try parsing as expressions first
        let mut items = Vec::new();

        // Handle SELECT *
        if self.current == Token::Asterisk {
            self.advance();
            // This must be a table SELECT
            self.expect(Token::From)?;
            return self.parse_table_select(vec!["*".to_string()]);
        }

        // Parse expression list
        loop {
            let expr = self.parse_expression()?;

            // Check for alias
            let alias = if self.current == Token::As {
                self.advance();
                if let Token::Ident(name) = &self.current {
                    let name = name.clone();
                    self.advance();
                    Some(name)
                } else {
                    return Err(ParseError::ExpectedToken("alias name".to_string()));
                }
            } else {
                None
            };

            items.push(SelectItem { expr, alias });

            if self.current != Token::Comma {
                break;
            }
            self.advance();
        }

        // Check if there's a FROM clause
        if self.current == Token::From {
            self.advance();
            // This is a table select - extract column names from expressions
            let columns: Vec<String> = items
                .into_iter()
                .map(|item| match item.expr {
                    Expr::Column(name) => name,
                    _ => "?column?".to_string(), // Fallback for complex expressions in table selects
                })
                .collect();
            return self.parse_table_select(columns);
        }

        // No FROM clause - this is an expression-only SELECT
        let format = self.parse_output_format()?;

        // Skip optional semicolon
        if self.current == Token::Semicolon {
            self.advance();
        }

        Ok(Statement::SelectExpr(SelectExprQuery {
            expressions: items,
            format,
        }))
    }

    fn parse_table_select(&mut self, columns: Vec<String>) -> Result<Statement, ParseError> {
        let table = if let Token::Ident(name) = &self.current {
            let name = name.clone();
            self.advance();
            name
        } else {
            return Err(ParseError::ExpectedToken("table name".to_string()));
        };

        let where_clause = if self.current == Token::Where {
            self.advance();
            Some(self.parse_where()?)
        } else {
            None
        };

        let limit = if self.current == Token::Limit {
            self.advance();
            if let Token::Number(n) = self.current {
                self.advance();
                Some(n as usize)
            } else {
                return Err(ParseError::ExpectedToken("number".to_string()));
            }
        } else {
            None
        };

        let format = self.parse_output_format()?;

        Ok(Statement::Select(SelectQuery {
            columns,
            table,
            where_clause,
            limit,
            format,
        }))
    }

    fn parse_output_format(&mut self) -> Result<OutputFormat, ParseError> {
        if self.current == Token::Format {
            self.advance();
            if self.current == Token::Json {
                self.advance();
                Ok(OutputFormat::Json)
            } else {
                Err(ParseError::ExpectedToken("JSON".to_string()))
            }
        } else {
            Ok(OutputFormat::Debug)
        }
    }

    /// Parse an expression with operator precedence
    fn parse_expression(&mut self) -> Result<Expr, ParseError> {
        self.parse_additive()
    }

    /// Parse additive expressions (+, -)
    fn parse_additive(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_multiplicative()?;

        loop {
            let op = match &self.current {
                Token::Plus => ArithmeticOp::Add,
                Token::Minus => ArithmeticOp::Subtract,
                _ => break,
            };
            self.advance();
            let right = self.parse_multiplicative()?;
            left = Expr::BinaryOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    /// Parse multiplicative expressions (*, /)
    fn parse_multiplicative(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_primary()?;

        loop {
            let op = match &self.current {
                Token::Asterisk => ArithmeticOp::Multiply,
                Token::Slash => ArithmeticOp::Divide,
                _ => break,
            };
            self.advance();
            let right = self.parse_primary()?;
            left = Expr::BinaryOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    /// Parse primary expressions (literals, identifiers, parenthesized expressions)
    fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        match &self.current {
            Token::Number(n) => {
                let n = *n;
                self.advance();
                Ok(Expr::Int(n))
            }
            Token::String(s) => {
                let s = s.clone();
                self.advance();
                Ok(Expr::Text(s))
            }
            Token::Null => {
                self.advance();
                Ok(Expr::Null)
            }
            Token::Ident(name) => {
                let name = name.clone();
                self.advance();
                Ok(Expr::Column(name))
            }
            Token::LeftParen => {
                self.advance();
                let expr = self.parse_expression()?;
                self.expect(Token::RightParen)?;
                Ok(expr)
            }
            tok => Err(ParseError::UnexpectedToken(tok.clone())),
        }
    }

    fn parse_insert(&mut self) -> Result<Statement, ParseError> {
        self.expect(Token::Insert)?;
        self.expect(Token::Into)?;

        let table = if let Token::Ident(name) = &self.current {
            let name = name.clone();
            self.advance();
            name
        } else {
            return Err(ParseError::ExpectedToken("table name".to_string()));
        };

        self.expect(Token::LeftParen)?;

        let mut columns = Vec::new();
        loop {
            if let Token::Ident(name) = &self.current {
                columns.push(name.clone());
                self.advance();
            } else {
                return Err(ParseError::ExpectedToken("column name".to_string()));
            }

            if self.current != Token::Comma {
                break;
            }
            self.advance();
        }

        self.expect(Token::RightParen)?;
        self.expect(Token::Values)?;
        self.expect(Token::LeftParen)?;

        let mut values = Vec::new();
        loop {
            let value = match &self.current {
                Token::Number(n) => {
                    let v = Expr::Int(*n);
                    self.advance();
                    v
                }
                Token::String(s) => {
                    let v = Expr::Text(s.clone());
                    self.advance();
                    v
                }
                Token::Null => {
                    self.advance();
                    Expr::Null
                }
                _ => return Err(ParseError::ExpectedToken("value".to_string())),
            };
            values.push(value);

            if self.current != Token::Comma {
                break;
            }
            self.advance();
        }

        self.expect(Token::RightParen)?;

        Ok(Statement::Insert(InsertStatement {
            table,
            columns,
            values,
        }))
    }

    fn parse_create(&mut self) -> Result<Statement, ParseError> {
        self.expect(Token::Create)?;
        self.expect(Token::Table)?;

        let name = if let Token::Ident(n) = &self.current {
            let n = n.clone();
            self.advance();
            n
        } else {
            return Err(ParseError::ExpectedToken("table name".to_string()));
        };

        self.expect(Token::LeftParen)?;

        let mut columns = Vec::new();
        loop {
            let col_name = if let Token::Ident(n) = &self.current {
                let n = n.clone();
                self.advance();
                n
            } else {
                return Err(ParseError::ExpectedToken("column name".to_string()));
            };

            let col_type = match &self.current {
                Token::Int => {
                    self.advance();
                    "INT".to_string()
                }
                Token::Text => {
                    self.advance();
                    "TEXT".to_string()
                }
                _ => return Err(ParseError::ExpectedToken("column type".to_string())),
            };

            columns.push(Column {
                name: col_name,
                column_type: col_type,
            });

            if self.current != Token::Comma {
                break;
            }
            self.advance();
        }

        self.expect(Token::RightParen)?;

        Ok(Statement::Create(CreateTable { name, columns }))
    }

    fn parse_where(&mut self) -> Result<WhereClause, ParseError> {
        let column = if let Token::Ident(name) = &self.current {
            let name = name.clone();
            self.advance();
            name
        } else {
            return Err(ParseError::ExpectedToken("column name".to_string()));
        };

        let operator = match &self.current {
            Token::Equal => CompareOp::Equal,
            Token::NotEqual => CompareOp::NotEqual,
            Token::LessThan => CompareOp::LessThan,
            Token::LessThanOrEqual => CompareOp::LessThanOrEqual,
            Token::GreaterThan => CompareOp::GreaterThan,
            Token::GreaterThanOrEqual => CompareOp::GreaterThanOrEqual,
            _ => return Err(ParseError::ExpectedToken("comparison operator".to_string())),
        };
        self.advance();

        let value = match &self.current {
            Token::Number(n) => {
                let v = Expr::Int(*n);
                self.advance();
                v
            }
            Token::String(s) => {
                let v = Expr::Text(s.clone());
                self.advance();
                v
            }
            Token::Null => {
                self.advance();
                Expr::Null
            }
            _ => return Err(ParseError::ExpectedToken("value".to_string())),
        };

        Ok(WhereClause {
            column,
            operator,
            value,
        })
    }

    fn parse_drop(&mut self) -> Result<Statement, ParseError> {
        self.advance();
        self.expect(Token::Table)?;

        let name = match &self.current {
            Token::Ident(s) => s.clone(),
            _ => return Err(ParseError::ExpectedToken("table name".to_string())),
        };
        self.advance();

        Ok(Statement::Drop(DropTable { name }))
    }
}
