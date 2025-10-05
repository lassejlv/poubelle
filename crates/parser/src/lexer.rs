#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Select,
    From,
    Insert,
    Into,
    Values,
    Create,
    Drop,
    Table,
    Int,
    Text,
    Null,
    Where,
    Limit,
    Format,
    Json,
    Ident(String),
    Number(i64),
    String(String),
    Asterisk,
    Comma,
    LeftParen,
    RightParen,
    Equal,
    NotEqual,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
    Eof,
}

pub struct Lexer {
    input: Vec<char>,
    pos: usize,
}

impl Lexer {
    pub fn new(input: &str) -> Self {
        Self {
            input: input.chars().collect(),
            pos: 0,
        }
    }

    pub fn next_token(&mut self) -> Token {
        self.skip_whitespace();

        if self.pos >= self.input.len() {
            return Token::Eof;
        }

        let ch = self.input[self.pos];

        match ch {
            '*' => {
                self.pos += 1;
                Token::Asterisk
            }
            ',' => {
                self.pos += 1;
                Token::Comma
            }
            '(' => {
                self.pos += 1;
                Token::LeftParen
            }
            ')' => {
                self.pos += 1;
                Token::RightParen
            }
            '=' => {
                self.pos += 1;
                Token::Equal
            }
            '!' => {
                self.pos += 1;
                if self.pos < self.input.len() && self.input[self.pos] == '=' {
                    self.pos += 1;
                    Token::NotEqual
                } else {
                    self.next_token()
                }
            }
            '<' => {
                self.pos += 1;
                if self.pos < self.input.len() && self.input[self.pos] == '=' {
                    self.pos += 1;
                    Token::LessThanOrEqual
                } else {
                    Token::LessThan
                }
            }
            '>' => {
                self.pos += 1;
                if self.pos < self.input.len() && self.input[self.pos] == '=' {
                    self.pos += 1;
                    Token::GreaterThanOrEqual
                } else {
                    Token::GreaterThan
                }
            }
            '\'' => self.read_string(),
            '0'..='9' | '-' => self.read_number(),
            _ if ch.is_alphabetic() => self.read_identifier(),
            _ => {
                self.pos += 1;
                self.next_token()
            }
        }
    }

    fn skip_whitespace(&mut self) {
        while self.pos < self.input.len() && self.input[self.pos].is_whitespace() {
            self.pos += 1;
        }
    }

    fn read_identifier(&mut self) -> Token {
        let start = self.pos;
        while self.pos < self.input.len()
            && (self.input[self.pos].is_alphanumeric() || self.input[self.pos] == '_')
        {
            self.pos += 1;
        }

        let ident: String = self.input[start..self.pos].iter().collect();
        match ident.to_uppercase().as_str() {
            "SELECT" => Token::Select,
            "FROM" => Token::From,
            "INSERT" => Token::Insert,
            "INTO" => Token::Into,
            "VALUES" => Token::Values,
            "CREATE" => Token::Create,
            "TABLE" => Token::Table,
            "INT" => Token::Int,
            "TEXT" => Token::Text,
            "NULL" => Token::Null,
            "WHERE" => Token::Where,
            "LIMIT" => Token::Limit,
            "FORMAT" => Token::Format,
            "JSON" => Token::Json,
            "DROP" => Token::Drop,
            _ => Token::Ident(ident),
        }
    }

    fn read_number(&mut self) -> Token {
        let start = self.pos;
        if self.input[self.pos] == '-' {
            self.pos += 1;
        }
        while self.pos < self.input.len() && self.input[self.pos].is_numeric() {
            self.pos += 1;
        }

        let num_str: String = self.input[start..self.pos].iter().collect();
        Token::Number(num_str.parse().unwrap_or(0))
    }

    fn read_string(&mut self) -> Token {
        self.pos += 1;
        let start = self.pos;
        while self.pos < self.input.len() && self.input[self.pos] != '\'' {
            self.pos += 1;
        }

        let s: String = self.input[start..self.pos].iter().collect();
        self.pos += 1;
        Token::String(s)
    }
}
