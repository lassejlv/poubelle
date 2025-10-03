use crate::ast::{Column, CreateTable, Expr, InsertStatement, SelectQuery, Statement};
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

        let mut columns = Vec::new();
        if self.current == Token::Asterisk {
            columns.push("*".to_string());
            self.advance();
        } else {
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
        }

        self.expect(Token::From)?;

        let table = if let Token::Ident(name) = &self.current {
            let name = name.clone();
            self.advance();
            name
        } else {
            return Err(ParseError::ExpectedToken("table name".to_string()));
        };

        Ok(Statement::Select(SelectQuery { columns, table }))
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
}
