use crate::ast::{BinaryOp, Expr, StructuredRef, UnaryOp};
use crate::error::CellError;
use crate::value::{CellCoord, CellRange, Value};
use rust_decimal::Decimal;
use std::iter::Peekable;
use std::str::Chars;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Number(Decimal),
    StringLiteral(String),
    Boolean(bool),
    Ident(String),
    CellRef(CellCoord),
    RangeRef(CellRange),
    SpillRef(CellCoord),
    StructuredRef(StructuredRef),
    Plus,
    Minus,
    Star,
    Slash,
    Caret,
    Ampersand,
    Percent,
    Equal,
    NotEqual,
    LessThan,
    LessEqual,
    GreaterThan,
    GreaterEqual,
    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace,
    Comma,
    Semicolon,
    Colon,
    Eof,
}

pub struct Lexer<'a> {
    chars: Peekable<Chars<'a>>,
    #[allow(dead_code)]
    input: &'a str,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        let trimmed = input.trim();
        let stripped = if trimmed.starts_with('=') {
            &trimmed[1..]
        } else {
            trimmed
        };
        Self {
            chars: stripped.chars().peekable(),
            input: stripped,
        }
    }

    fn skip_whitespace(&mut self) {
        while let Some(&ch) = self.chars.peek() {
            if ch.is_whitespace() {
                self.chars.next();
            } else {
                break;
            }
        }
    }

    pub fn next_token(&mut self) -> Result<Token, CellError> {
        self.skip_whitespace();
        let ch = match self.chars.next() {
            Some(c) => c,
            None => return Ok(Token::Eof),
        };

        match ch {
            '+' => Ok(Token::Plus),
            '-' => Ok(Token::Minus),
            '*' => Ok(Token::Star),
            '/' => Ok(Token::Slash),
            '^' => Ok(Token::Caret),
            '&' => Ok(Token::Ampersand),
            '%' => Ok(Token::Percent),
            '(' => Ok(Token::LParen),
            ')' => Ok(Token::RParen),
            '{' => Ok(Token::LBrace),
            '}' => Ok(Token::RBrace),
            ',' => Ok(Token::Comma),
            ';' => Ok(Token::Semicolon),
            '=' => Ok(Token::Equal),
            '<' => {
                if let Some(&'>') = self.chars.peek() {
                    self.chars.next();
                    Ok(Token::NotEqual)
                } else if let Some(&'=') = self.chars.peek() {
                    self.chars.next();
                    Ok(Token::LessEqual)
                } else {
                    Ok(Token::LessThan)
                }
            }
            '>' => {
                if let Some(&'=') = self.chars.peek() {
                    self.chars.next();
                    Ok(Token::GreaterEqual)
                } else {
                    Ok(Token::GreaterThan)
                }
            }
            ':' => Ok(Token::Colon),
            '"' => self.lex_string(),
            '0'..='9' => self.lex_number(ch),
            'a'..='z' | 'A'..='Z' | '_' | '$' => self.lex_ident_or_ref(ch),
            '[' => {
                // Table header or bracketed identifier
                let mut buf = String::new();
                while let Some(&c) = self.chars.peek() {
                    if c == ']' {
                        self.chars.next();
                        break;
                    }
                    buf.push(c);
                    self.chars.next();
                }
                Ok(Token::Ident(buf))
            }
            _ => Err(CellError::Name),
        }
    }

    fn lex_string(&mut self) -> Result<Token, CellError> {
        let mut s = String::new();
        while let Some(ch) = self.chars.next() {
            if ch == '"' {
                // Check escaped quote ""
                if let Some(&'"') = self.chars.peek() {
                    self.chars.next();
                    s.push('"');
                } else {
                    return Ok(Token::StringLiteral(s));
                }
            } else {
                s.push(ch);
            }
        }
        Err(CellError::Value)
    }

    fn lex_number(&mut self, first: char) -> Result<Token, CellError> {
        let mut s = String::new();
        s.push(first);
        let mut has_dot = false;

        while let Some(&ch) = self.chars.peek() {
            if ch.is_ascii_digit() {
                s.push(ch);
                self.chars.next();
            } else if ch == '.' && !has_dot {
                has_dot = true;
                s.push(ch);
                self.chars.next();
            } else {
                break;
            }
        }

        let dec = Decimal::from_str_exact(&s).map_err(|_| CellError::Num)?;
        Ok(Token::Number(dec))
    }

    fn lex_ident_or_ref(&mut self, first: char) -> Result<Token, CellError> {
        let mut s = String::new();
        s.push(first);

        while let Some(&ch) = self.chars.peek() {
            if ch.is_ascii_alphanumeric() || ch == '_' || ch == '.' || ch == '$' {
                s.push(ch);
                self.chars.next();
            } else {
                break;
            }
        }

        // Check for structured reference: TableName[Column]
        if let Some(&'[') = self.chars.peek() {
            self.chars.next(); // consume '['
            let mut bracket_content = String::new();
            let mut depth = 1;
            while let Some(ch) = self.chars.next() {
                if ch == '[' {
                    depth += 1;
                    bracket_content.push(ch);
                } else if ch == ']' {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                    bracket_content.push(ch);
                } else {
                    bracket_content.push(ch);
                }
            }

            let sref = parse_structured_ref(&s, &bracket_content)?;
            return Ok(Token::StructuredRef(sref));
        }

        // Check for spill reference: A1#
        if let Some(&'#') = self.chars.peek() {
            if let Ok(coord) = CellCoord::from_a1(&s) {
                self.chars.next(); // consume '#'
                return Ok(Token::SpillRef(coord));
            }
        }

        // Check if Range Ref: A1:B10
        if let Some(&':') = self.chars.peek() {
            if let Ok(start) = CellCoord::from_a1(&s) {
                self.chars.next(); // consume ':'
                self.skip_whitespace();
                let mut end_s = String::new();
                while let Some(&ch) = self.chars.peek() {
                    if ch.is_ascii_alphanumeric() || ch == '$' {
                        end_s.push(ch);
                        self.chars.next();
                    } else {
                        break;
                    }
                }
                if let Ok(end) = CellCoord::from_a1(&end_s) {
                    return Ok(Token::RangeRef(CellRange::new(
                        start.row, start.col, end.row, end.col,
                    )));
                } else {
                    return Err(CellError::Ref);
                }
            }
        }

        // Check single CellRef
        if let Ok(coord) = CellCoord::from_a1(&s) {
            return Ok(Token::CellRef(coord));
        }

        let upper = s.to_uppercase();
        if upper == "TRUE" {
            return Ok(Token::Boolean(true));
        }
        if upper == "FALSE" {
            return Ok(Token::Boolean(false));
        }

        Ok(Token::Ident(upper))
    }
}

fn parse_structured_ref(table_name: &str, content: &str) -> Result<StructuredRef, CellError> {
    let trimmed = content.trim();
    let mut is_row_relative = false;
    let mut include_headers = false;
    let mut include_totals = false;

    // Handle nested syntax e.g. [[#All],[Amount]] or [@Amount] or [Amount]
    let col = if trimmed.starts_with('@') {
        is_row_relative = true;
        trimmed[1..].trim()
    } else if trimmed.starts_with("[#All]") {
        include_headers = true;
        let rest = trimmed["[#All]".len()..].trim().trim_start_matches(',').trim();
        strip_brackets(rest)
    } else if trimmed.starts_with("[#Totals]") {
        include_totals = true;
        let rest = trimmed["[#Totals]".len()..].trim().trim_start_matches(',').trim();
        strip_brackets(rest)
    } else {
        strip_brackets(trimmed)
    };

    Ok(StructuredRef {
        table_name: table_name.to_string(),
        column_name: col.to_string(),
        is_row_relative,
        include_headers,
        include_totals,
    })
}

fn strip_brackets(s: &str) -> &str {
    let trimmed = s.trim();
    if trimmed.starts_with('[') && trimmed.ends_with(']') {
        &trimmed[1..trimmed.len() - 1]
    } else {
        trimmed
    }
}

pub struct Parser<'a> {
    tokens: Vec<Token>,
    pos: usize,
    _marker: std::marker::PhantomData<&'a ()>,
}

impl<'a> Parser<'a> {
    pub fn new(input: &'a str) -> Result<Self, CellError> {
        let mut lexer = Lexer::new(input);
        let mut tokens = Vec::new();
        loop {
            let tok = lexer.next_token()?;
            if tok == Token::Eof {
                tokens.push(tok);
                break;
            }
            tokens.push(tok);
        }
        Ok(Self {
            tokens,
            pos: 0,
            _marker: std::marker::PhantomData,
        })
    }

    fn peek(&self) -> &Token {
        self.tokens.get(self.pos).unwrap_or(&Token::Eof)
    }

    fn next(&mut self) -> Token {
        if self.pos < self.tokens.len() {
            let tok = self.tokens[self.pos].clone();
            self.pos += 1;
            tok
        } else {
            Token::Eof
        }
    }

    fn expect(&mut self, expected: Token) -> Result<(), CellError> {
        let actual = self.next();
        if actual == expected {
            Ok(())
        } else {
            Err(CellError::Value)
        }
    }

    pub fn parse(&mut self) -> Result<Expr, CellError> {
        if self.tokens.is_empty() || self.peek() == &Token::Eof {
            return Ok(Expr::Literal(Value::Blank));
        }
        let expr = self.parse_comparison()?;
        if self.peek() != &Token::Eof {
            return Err(CellError::Value);
        }
        Ok(expr)
    }

    fn parse_comparison(&mut self) -> Result<Expr, CellError> {
        let mut left = self.parse_concat()?;

        while let Token::Equal
        | Token::NotEqual
        | Token::LessThan
        | Token::LessEqual
        | Token::GreaterThan
        | Token::GreaterEqual = self.peek()
        {
            let op_token = self.next();
            let op = match op_token {
                Token::Equal => BinaryOp::Eq,
                Token::NotEqual => BinaryOp::NotEq,
                Token::LessThan => BinaryOp::Lt,
                Token::LessEqual => BinaryOp::LtEq,
                Token::GreaterThan => BinaryOp::Gt,
                Token::GreaterEqual => BinaryOp::GtEq,
                _ => unreachable!(),
            };
            let right = self.parse_concat()?;
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_concat(&mut self) -> Result<Expr, CellError> {
        let mut left = self.parse_additive()?;

        while let Token::Ampersand = self.peek() {
            self.next();
            let right = self.parse_additive()?;
            left = Expr::Binary {
                op: BinaryOp::Concat,
                left: Box::new(left),
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_additive(&mut self) -> Result<Expr, CellError> {
        let mut left = self.parse_multiplicative()?;

        while let Token::Plus | Token::Minus = self.peek() {
            let op_tok = self.next();
            let op = match op_tok {
                Token::Plus => BinaryOp::Add,
                Token::Minus => BinaryOp::Sub,
                _ => unreachable!(),
            };
            let right = self.parse_multiplicative()?;
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> Result<Expr, CellError> {
        let mut left = self.parse_power()?;

        while let Token::Star | Token::Slash = self.peek() {
            let op_tok = self.next();
            let op = match op_tok {
                Token::Star => BinaryOp::Mul,
                Token::Slash => BinaryOp::Div,
                _ => unreachable!(),
            };
            let right = self.parse_power()?;
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_power(&mut self) -> Result<Expr, CellError> {
        let mut left = self.parse_unary()?;

        if let Token::Caret = self.peek() {
            self.next();
            let right = self.parse_power()?;
            left = Expr::Binary {
                op: BinaryOp::Power,
                left: Box::new(left),
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, CellError> {
        match self.peek() {
            Token::Plus => {
                self.next();
                let expr = self.parse_unary()?;
                Ok(Expr::Unary {
                    op: UnaryOp::Plus,
                    expr: Box::new(expr),
                })
            }
            Token::Minus => {
                self.next();
                let expr = self.parse_unary()?;
                Ok(Expr::Unary {
                    op: UnaryOp::Minus,
                    expr: Box::new(expr),
                })
            }
            _ => {
                let mut primary = self.parse_primary()?;
                if let Token::Percent = self.peek() {
                    self.next();
                    primary = Expr::Unary {
                        op: UnaryOp::Percent,
                        expr: Box::new(primary),
                    };
                }
                Ok(primary)
            }
        }
    }

    fn parse_primary(&mut self) -> Result<Expr, CellError> {
        match self.peek() {
            Token::Number(_) => {
                if let Token::Number(d) = self.next() {
                    Ok(Expr::Literal(Value::Decimal(d)))
                } else {
                    unreachable!()
                }
            }
            Token::StringLiteral(_) => {
                if let Token::StringLiteral(s) = self.next() {
                    Ok(Expr::Literal(Value::Text(s)))
                } else {
                    unreachable!()
                }
            }
            Token::Boolean(_) => {
                if let Token::Boolean(b) = self.next() {
                    Ok(Expr::Literal(Value::Boolean(b)))
                } else {
                    unreachable!()
                }
            }
            Token::CellRef(_) => {
                if let Token::CellRef(coord) = self.next() {
                    if let Token::Colon = self.peek() {
                        self.next();
                        if let Token::CellRef(end) = self.next() {
                            Ok(Expr::RangeRef(CellRange::new(
                                coord.row, coord.col, end.row, end.col,
                            )))
                        } else {
                            Err(CellError::Ref)
                        }
                    } else {
                        Ok(Expr::CellRef(coord))
                    }
                } else {
                    unreachable!()
                }
            }
            Token::RangeRef(_) => {
                if let Token::RangeRef(range) = self.next() {
                    Ok(Expr::RangeRef(range))
                } else {
                    unreachable!()
                }
            }
            Token::SpillRef(_) => {
                if let Token::SpillRef(coord) = self.next() {
                    Ok(Expr::SpillRef(coord))
                } else {
                    unreachable!()
                }
            }
            Token::StructuredRef(_) => {
                if let Token::StructuredRef(sref) = self.next() {
                    Ok(Expr::StructuredRef(sref))
                } else {
                    unreachable!()
                }
            }
            Token::Ident(_) => {
                if let Token::Ident(name) = self.next() {
                    if let Token::LParen = self.peek() {
                        self.next(); // consume '('
                        let mut args = Vec::new();
                        if self.peek() != &Token::RParen {
                            loop {
                                args.push(self.parse_comparison()?);
                                if self.peek() == &Token::Comma {
                                    self.next();
                                } else {
                                    break;
                                }
                            }
                        }
                        self.expect(Token::RParen)?;
                        Ok(Expr::FunctionCall { name, args })
                    } else if name == "NULL" {
                        Ok(Expr::Literal(Value::Null))
                    } else {
                        Err(CellError::Name)
                    }
                } else {
                    unreachable!()
                }
            }
            Token::LParen => {
                self.next();
                let expr = self.parse_comparison()?;
                self.expect(Token::RParen)?;
                Ok(expr)
            }
            Token::LBrace => self.parse_array_literal(),
            _ => Err(CellError::Value),
        }
    }

    fn parse_array_literal(&mut self) -> Result<Expr, CellError> {
        self.next(); // consume '{'
        let mut rows = Vec::new();
        let mut current_row = Vec::new();

        while self.peek() != &Token::RBrace && self.peek() != &Token::Eof {
            current_row.push(self.parse_comparison()?);
            match self.peek() {
                Token::Comma => {
                    self.next();
                }
                Token::Semicolon => {
                    self.next();
                    rows.push(std::mem::take(&mut current_row));
                }
                Token::RBrace => break,
                _ => return Err(CellError::Value),
            }
        }
        if !current_row.is_empty() {
            rows.push(current_row);
        }
        self.expect(Token::RBrace)?;
        Ok(Expr::ArrayLiteral(rows))
    }
}

pub fn parse_formula(input: &str) -> Result<Expr, CellError> {
    Parser::new(input)?.parse()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_arithmetic() {
        let expr = parse_formula("=1 + 2 * 3").unwrap();
        match expr {
            Expr::Binary { op: BinaryOp::Add, left, right } => {
                assert_eq!(*left, Expr::Literal(Value::Decimal(Decimal::from(1))));
                match *right {
                    Expr::Binary { op: BinaryOp::Mul, .. } => {}
                    _ => panic!("Expected multiplication on right"),
                }
            }
            _ => panic!("Expected addition"),
        }
    }

    #[test]
    fn test_parse_function_call() {
        let expr = parse_formula("=SUM(A1:B10, 42)").unwrap();
        match expr {
            Expr::FunctionCall { name, args } => {
                assert_eq!(name, "SUM");
                assert_eq!(args.len(), 2);
                assert_eq!(args[0], Expr::RangeRef(CellRange::new(0, 0, 9, 1)));
                assert_eq!(args[1], Expr::Literal(Value::Decimal(Decimal::from(42))));
            }
            _ => panic!("Expected function call"),
        }
    }

    #[test]
    fn test_parse_structured_ref() {
        let expr = parse_formula("=Invoices[Amount] * 1.05").unwrap();
        match expr {
            Expr::Binary { op: BinaryOp::Mul, left, .. } => {
                match *left {
                    Expr::StructuredRef(sref) => {
                        assert_eq!(sref.table_name, "Invoices");
                        assert_eq!(sref.column_name, "Amount");
                        assert!(!sref.is_row_relative);
                    }
                    _ => panic!("Expected structured ref"),
                }
            }
            _ => panic!("Expected binary op"),
        }
    }

    #[test]
    fn test_parse_spill_ref() {
        let expr = parse_formula("=A1#").unwrap();
        assert_eq!(expr, Expr::SpillRef(CellCoord::new(0, 0)));
    }
}
