use std::io::{self, BufRead};

use crate::compiler::error::LexError;
use crate::compiler::span::Span;
use crate::compiler::token::{Token, TokenKind};

pub struct Lexer<R: BufRead> {
    reader: R,
    pending_byte: Option<(u8, usize)>,
    pending_char: Option<(char, Span)>,
    line: usize,
    column: usize,
    offset: usize,
}

impl<R: BufRead> Lexer<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            pending_byte: None,
            pending_char: None,
            line: 1,
            column: 1,
            offset: 0,
        }
    }

    pub fn next_token(&mut self) -> Result<Token, LexError> {
        self.skip_whitespace_and_comments()
            .map_err(|err| self.io_error(err))?;
        let next = self.next_char().map_err(|err| self.io_error(err))?;
        let (ch, span) = match next {
            Some(value) => value,
            None => return Ok(Token::new(TokenKind::Eof, self.current_span())),
        };

        let token = match ch {
            '@' => {
                let name = self.collect_identifier()?;
                if name.is_empty() {
                    return Err(LexError::new("import name cannot be empty", span));
                }
                Token::new(TokenKind::Import(name), span)
            }
            'a'..='z' | 'A'..='Z' | '_' => {
                let mut ident = String::new();
                ident.push(ch);
                ident.push_str(&self.collect_identifier()?);
                Token::new(TokenKind::Ident(ident), span)
            }
            '0'..='9' => {
                let mut literal = ch.to_string();
                literal.push_str(&self.collect_digits()?);
                let value = literal
                    .parse::<i64>()
                    .map_err(|_| LexError::new("invalid integer literal", span))?;
                Token::new(TokenKind::IntLiteral(value), span)
            }
            '(' => Token::new(TokenKind::LParen, span),
            ')' => Token::new(TokenKind::RParen, span),
            '{' => Token::new(TokenKind::LBrace, span),
            '}' => Token::new(TokenKind::RBrace, span),
            '[' => Token::new(TokenKind::LBracket, span),
            ']' => Token::new(TokenKind::RBracket, span),
            ',' => Token::new(TokenKind::Comma, span),
            ':' => Token::new(TokenKind::Colon, span),
            ';' => Token::new(TokenKind::Semicolon, span),
            '.' => {
                let dot_count = self.collect_dots(span)?;
                match dot_count {
                    1 => Token::new(TokenKind::Dot, span),
                    2 => return Err(LexError::new("invalid token: expected '.' or '...'", span)),
                    3 => Token::new(TokenKind::Ellipsis, span),
                    _ => return Err(LexError::new("invalid token: expected '.' or '...'", span)),
                }
            }
            '+' => Token::new(TokenKind::Plus, span),
            '*' => Token::new(TokenKind::Star, span),
            '!' => Token::new(TokenKind::Bang, span),
            '?' => Token::new(TokenKind::Question, span),
            '/' => Token::new(TokenKind::Slash, span),
            '-' => Token::new(TokenKind::Minus, span),
            '=' => Token::new(TokenKind::Equals, span),
            '"' => self.string_token(span, '"')?,
            '\'' => self.string_token(span, '\'')?,
            _ => {
                return Err(LexError::new(
                    format!("unexpected character '{}'", ch),
                    span,
                ))
            }
        };

        Ok(token)
    }

    fn collect_digits(&mut self) -> Result<String, LexError> {
        let mut buf = String::new();
        while let Some((ch, _)) = self.peek_char().map_err(|err| self.io_error(err))? {
            if ch.is_ascii_digit() {
                buf.push(ch);
                self.next_char().map_err(|err| self.io_error(err))?;
            } else {
                break;
            }
        }
        Ok(buf)
    }

    fn collect_identifier(&mut self) -> Result<String, LexError> {
        let mut buf = String::new();
        while let Some((ch, _)) = self.peek_char().map_err(|err| self.io_error(err))? {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                buf.push(ch);
                self.next_char().map_err(|err| self.io_error(err))?;
            } else {
                break;
            }
        }
        Ok(buf)
    }

    fn collect_dots(&mut self, start_span: Span) -> Result<usize, LexError> {
        let mut count = 1;

        while let Some((ch, _)) = self.peek_char().map_err(|err| self.io_error(err))? {
            if ch == '.' {
                count += 1;

                if count > 3 {
                    return Err(LexError::new("too many dots (maximum is 3)", start_span));
                }

                // consume the dot
                self.next_char().map_err(|err| self.io_error(err))?;
            } else {
                break;
            }
        }

        Ok(count)
    }

    fn skip_whitespace_and_comments(&mut self) -> io::Result<()> {
        loop {
            let Some((ch, _span)) = self.peek_char()? else {
                break;
            };
            match ch {
                c if c.is_whitespace() => {
                    self.next_char()?;
                }
                '/' => {
                    let slash = self.next_char()?.expect("peeked char must exist");
                    if let Some((next, _)) = self.peek_char()? {
                        if next == '/' {
                            self.next_char()?; // consume second slash
                            self.consume_until_newline()?;
                            continue;
                        }
                    }
                    self.pending_char = Some(slash);
                    break;
                }
                _ => break,
            }
        }
        Ok(())
    }

    fn consume_until_newline(&mut self) -> io::Result<()> {
        while let Some((ch, _)) = self.next_char()? {
            if ch == '\n' {
                break;
            }
        }
        Ok(())
    }

    fn string_token(&mut self, start_span: Span, delimiter: char) -> Result<Token, LexError> {
        let mut value = String::new();
        loop {
            let (ch, _) = self
                .next_char()
                .map_err(|err| self.io_error(err))?
                .ok_or_else(|| LexError::new("unterminated string literal", start_span))?;
            if ch == delimiter {
                break;
            }
            if ch == '\n' {
                return Err(LexError::new("newline inside string literal", start_span));
            }

            if ch == '\\' && delimiter == '"' {
                let (escaped, _) = self
                    .next_char()
                    .map_err(|err| self.io_error(err))?
                    .ok_or_else(|| LexError::new("unterminated escape sequence", start_span))?;
                if escaped == '\n' {
                    return Err(LexError::new("newline inside string literal", start_span));
                }
                let esc_char = match escaped {
                    '"' => '"',
                    '\'' => '\'',
                    '\\' => '\\',
                    '0' => '\0',
                    'n' => '\n',
                    'r' => '\r',
                    't' => '\t',
                    'u' => self.parse_unicode_escape(start_span)?,
                    other => {
                        return Err(LexError::new(
                            format!("invalid escape '\\\\{}'", other),
                            start_span,
                        ))
                    }
                };
                value.push(esc_char);
            } else {
                value.push(ch);
            }
        }
        Ok(Token::new(TokenKind::StringLiteral(value), start_span))
    }

    fn parse_unicode_escape(&mut self, start_span: Span) -> Result<char, LexError> {
        let (opening, _) = self
            .next_char()
            .map_err(|err| self.io_error(err))?
            .ok_or_else(|| LexError::new("unterminated escape sequence", start_span))?;
        if opening != '{' {
            return Err(LexError::new("invalid unicode escape", start_span));
        }
        let mut digits = String::new();
        loop {
            let (ch, _) = self
                .next_char()
                .map_err(|err| self.io_error(err))?
                .ok_or_else(|| LexError::new("unterminated unicode escape", start_span))?;
            if ch == '}' {
                break;
            }
            if ch == '\n' {
                return Err(LexError::new("newline inside string literal", start_span));
            }
            if ch.is_ascii_hexdigit() {
                digits.push(ch);
            } else {
                return Err(LexError::new("invalid unicode escape", start_span));
            }
        }
        if digits.is_empty() {
            return Err(LexError::new("invalid unicode escape", start_span));
        }
        let codepoint = u32::from_str_radix(&digits, 16)
            .map_err(|_| LexError::new("invalid unicode escape", start_span))?;
        char::from_u32(codepoint)
            .ok_or_else(|| LexError::new("invalid unicode codepoint", start_span))
    }

    fn peek_char(&mut self) -> io::Result<Option<(char, Span)>> {
        if self.pending_char.is_none() {
            self.pending_char = self.read_char_raw()?;
        }
        Ok(self.pending_char)
    }

    fn next_char(&mut self) -> io::Result<Option<(char, Span)>> {
        if let Some(ch) = self.pending_char.take() {
            return Ok(Some(ch));
        }
        self.read_char_raw()
    }

    fn read_char_raw(&mut self) -> io::Result<Option<(char, Span)>> {
        let (byte, offset) = match self.read_byte()? {
            Some(pair) => pair,
            None => return Ok(None),
        };
        let span = Span::new(self.line, self.column, offset);
        match byte {
            b'\n' => {
                self.line += 1;
                self.column = 1;
                Ok(Some(('\n', span)))
            }
            b'\r' => {
                if let Some((next, _)) = self.peek_byte()? {
                    if next == b'\n' {
                        self.read_byte()?; // consume '\n'
                    }
                }
                self.line += 1;
                self.column = 1;
                Ok(Some(('\n', span)))
            }
            _ => {
                self.column += 1;
                Ok(Some((byte as char, span)))
            }
        }
    }

    fn peek_byte(&mut self) -> io::Result<Option<(u8, usize)>> {
        if self.pending_byte.is_none() {
            let buf = self.reader.fill_buf()?;
            if buf.is_empty() {
                return Ok(None);
            }
            let offset = self.offset;
            self.pending_byte = Some((buf[0], offset));
        }
        Ok(self.pending_byte)
    }

    fn read_byte(&mut self) -> io::Result<Option<(u8, usize)>> {
        if let Some((byte, offset)) = self.pending_byte.take() {
            self.reader.consume(1);
            self.offset = offset + 1;
            return Ok(Some((byte, offset)));
        }
        let (byte, offset) = {
            let buf = self.reader.fill_buf()?;
            if buf.is_empty() {
                return Ok(None);
            }
            (buf[0], self.offset)
        };
        self.reader.consume(1);
        self.offset += 1;
        Ok(Some((byte, offset)))
    }

    fn current_span(&self) -> Span {
        Span::new(self.line, self.column, self.offset)
    }

    fn io_error(&self, err: io::Error) -> LexError {
        LexError::new(err.to_string(), self.current_span())
    }
}
