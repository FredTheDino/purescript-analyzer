use drop_bomb::DropBomb;
use syntax::SyntaxKind;

use crate::{
    input::Input,
    layout::{Layout, LayoutKind},
    position::Position,
};

#[derive(Debug)]
pub enum Event {
    Start { kind: SyntaxKind },
    Token { kind: SyntaxKind },
    Error { message: String },
    Finish,
}

pub struct Parser {
    input: Input,
    index: usize,

    layouts: Vec<Layout>,
    events: Vec<Event>,
}

impl Parser {
    pub fn new(input: Input) -> Parser {
        let index = 0;
        let layout = vec![Layout {
            kind: LayoutKind::Root,
            position: Position { offset: 0, line: 1, column: 1 },
        }];
        let events = vec![];
        Parser { input, index, layouts: layout, events }
    }

    pub fn is_eof(&self) -> bool {
        self.index == self.input.len()
    }
}

impl Parser {
    /// Starts a new layout context.
    pub fn layout_start(&mut self, kind: LayoutKind) {
        assert!(!self.is_eof());
        let position = self.input.position(self.index);
        self.layouts.push(Layout::new(kind, position));
    }

    /// Finishes the current layout context.
    pub fn layout_end(&mut self) {
        self.layouts.pop();
    }

    /// Determines if the current token belongs to the next layout context.
    pub fn layout_done(&self) -> bool {
        if self.is_eof() {
            return true;
        }

        let position = self.input.position(self.index);
        let layout = self.layouts.last().unwrap();

        assert!(position.line >= layout.position.line);

        match layout.kind {
            LayoutKind::Root => panic!("Invalid call."),
            // NOTE: handled by is_eof
            LayoutKind::Module => false,
            LayoutKind::Instance => position.column <= layout.position.column,
            // NOTE: handled by is_eof
            LayoutKind::Parenthesis => false,
        }
    }

    /// Determines if the current token belongs to the next token group.
    pub fn group_done(&self) -> bool {
        if self.is_eof() {
            return true;
        }

        let position = self.input.position(self.index);
        let layout = self.layouts.last().unwrap();

        assert!(position.line >= layout.position.line);

        match layout.kind {
            LayoutKind::Root => panic!("Invalid call."),
            // NOTE: handled by is_eof
            LayoutKind::Module => position.column == layout.position.column,
            LayoutKind::Instance => position.column <= layout.position.column,
            // NOTE: handled by is_eof
            LayoutKind::Parenthesis => false,
        }
    }
}

impl Parser {
    pub fn start(&mut self) -> NodeMarker {
        let index = self.events.len();
        self.events.push(Event::Start { kind: SyntaxKind::Sentinel });
        NodeMarker::new(index)
    }
}

pub struct NodeMarker {
    index: usize,
    bomb: DropBomb,
}

impl NodeMarker {
    pub fn new(index: usize) -> NodeMarker {
        let bomb = DropBomb::new("failed to call end or cancel");
        NodeMarker { index, bomb }
    }

    pub fn end(&mut self, parser: &mut Parser, kind: SyntaxKind) {
        self.bomb.defuse();
        match &mut parser.events[self.index] {
            Event::Start { kind: sentinel } => {
                *sentinel = kind;
            }
            _ => unreachable!(),
        }
        parser.events.push(Event::Finish);
    }

    pub fn cancel(&mut self, parser: &mut Parser) {
        self.bomb.defuse();
        if self.index == parser.events.len() - 1 {
            match parser.events.pop() {
                Some(Event::Start { kind: SyntaxKind::Sentinel }) => (),
                _ => unreachable!(),
            }
        }
    }
}

impl Parser {
    /// Returns the nth token given an `offset`.
    pub fn nth(&self, offset: usize) -> SyntaxKind {
        self.input.kind(self.index + offset)
    }

    /// Determines if an nth token matches a `kind`.
    pub fn nth_at(&self, offset: usize, kind: SyntaxKind) -> bool {
        self.nth(offset) == kind
    }

    /// Returns the current token.
    pub fn current(&self) -> SyntaxKind {
        self.nth(0)
    }

    /// Determines if the current token matches a `kind`.
    pub fn at(&self, kind: SyntaxKind) -> bool {
        self.nth_at(0, kind)
    }

    /// Consumes a token, advancing the parser.
    pub fn consume(&mut self) {
        let kind = self.current();
        self.index += 1;
        self.events.push(Event::Token { kind })
    }

    /// Consumes a token if it matches the `kind`.
    pub fn eat(&mut self, kind: SyntaxKind) -> bool {
        if !self.at(kind) {
            return false;
        }
        self.consume();
        true
    }
}

#[cfg(test)]
mod tests {
    use syntax::SyntaxKind::{self, *};

    use crate::{layout::LayoutKind, lexer::lex};

    use super::Parser;

    fn parse_module(parser: &mut Parser) {
        parser.eat(ModuleKw);
        parse_module_name(parser);
        parser.eat(WhereKw);

        parser.layout_start(LayoutKind::Module);
        loop {
            parse_value_declaration(parser);
            if parser.layout_done() {
                break;
            }
        }
        parser.layout_end();
    }

    fn parse_module_name(parser: &mut Parser) {
        parser.eat(SyntaxKind::Upper);
    }

    fn parse_value_declaration(parser: &mut Parser) {
        let mut marker = parser.start();
        loop {
            if parser.at(SyntaxKind::LeftParenthesis) {
                parser.layout_start(LayoutKind::Parenthesis);
            }
            if parser.at(SyntaxKind::RightParenthesis) {
                parser.layout_end();
            }
            parser.consume();
            if parser.group_done() {
                break;
            }
        }
        marker.end(parser, SyntaxKind::ValueDeclaration);
    }

    #[test]
    fn grammar_api_test() {
        let lexed = lex(r"module Hello where
hello = world
  0 'a' 1.2

hello = (
world
0 
'a' 
1.2
)
  ");
        let input = lexed.as_input();
        let mut parser = Parser::new(input);
        parse_module(&mut parser);
        dbg!(parser.layouts);
        dbg!(parser.events);
    }
}
