use crate::{stream::Stream, util};

use super::Selector;

pub struct Parser<'a> {
    pub stream: Stream<'a, u8>,
}

impl<'a> Parser<'a> {
    pub fn new(input: &'a [u8]) -> Self {
        Self {
            stream: Stream::new(input),
        }
    }

    pub fn skip_whitespaces(&mut self) -> bool {
        let has_whitespace = self.stream.expect_and_skip(b' ').is_some();
        while !self.stream.is_eof() {
            if self.stream.expect_and_skip(b' ').is_none() {
                break;
            }
        }
        has_whitespace
    }

    pub fn read_identifier(&mut self) -> &'a [u8] {
        let start = self.stream.idx;

        while !self.stream.is_eof() {
            let is_ident = self.stream.current().copied().map_or(false, util::is_ident);
            if !is_ident {
                break;
            } else {
                self.stream.advance();
            }
        }

        self.stream.slice(start, self.stream.idx)
    }

    pub fn parse_combinator(&mut self, left: Selector<'a>) -> Option<Selector<'a>> {
        let has_whitespaces = self.skip_whitespaces();

        let tok = if let Some(tok) = self.stream.current_cpy() {
            tok
        } else {
            return Some(left);
        };

        let combinator = match tok {
            b',' => {
                self.stream.advance();
                let right = self.selector()?;
                Selector::Or(Box::new(left), Box::new(right))
            }
            b'>' => {
                self.stream.advance();
                let right = self.selector()?;
                Selector::Parent(Box::new(left), Box::new(right))
            }
            _ if has_whitespaces => {
                let right = self.selector()?;
                Selector::Descendant(Box::new(left), Box::new(right))
            }
            _ if !has_whitespaces => {
                let right = self.selector()?;
                Selector::And(Box::new(left), Box::new(right))
            }
            _ => unreachable!(),
        };

        Some(combinator)
    }

    pub fn parse_attribute(&mut self) -> Option<Selector<'a>> {
        let attribute = self.read_identifier();
        let ty = match self.stream.current_cpy() {
            Some(b']') => {
                self.stream.advance();
                Selector::Attribute(attribute)
            }
            Some(b'=') => {
                self.stream.advance();
                let value = self.read_identifier();
                self.stream.expect_and_skip(b']')?;
                Selector::AttributeValue(attribute, value)
            }
            Some(c @ b'~' | c @ b'^' | c @ b'$' | c @ b'*') => {
                self.stream.advance();
                self.stream.expect_and_skip(b'=')?;
                let value = self.read_identifier();
                self.stream.expect_and_skip(b']')?;
                match c {
                    b'~' => Selector::AttributeValueWhitespacedContains(attribute, value),
                    b'^' => Selector::AttributeValueStartsWith(attribute, value),
                    b'$' => Selector::AttributeValueEndsWith(attribute, value),
                    b'*' => Selector::AttributeValueSubstring(attribute, value),
                    _ => unreachable!(),
                }
            }
            _ => return None,
        };
        Some(ty)
    }

    pub fn selector(&mut self) -> Option<Selector<'a>> {
        self.skip_whitespaces();
        let tok = self.stream.current_cpy()?;

        let left = match tok {
            b'#' => {
                self.stream.advance();
                let id = self.read_identifier();
                Selector::Id(id)
            }
            b'.' => {
                self.stream.advance();
                let class = self.read_identifier();
                Selector::Class(class)
            }
            b'*' => {
                self.stream.advance();
                Selector::All
            }
            b'[' => {
                self.stream.advance();
                self.parse_attribute()?
            }
            _ if util::is_ident(tok) => {
                let tag = self.read_identifier();
                Selector::Tag(tag)
            }
            _ => return None,
        };

        self.parse_combinator(left)
    }
}