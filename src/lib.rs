use std::sync::atomic::{AtomicUsize, Ordering};
use std::{any::Any, collections::HashMap};

static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

type ParserId = usize;
type Pos = usize;

#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    source: String,
    pos: usize,
    reason: String,
}

type ParseResult<T> = Result<(Pos, T), ParseError>;
type RawParser<T> = Box<dyn Fn(&str, Pos) -> ParseResult<T>>;

pub struct Context {
    cache: HashMap<(ParserId, Pos), Box<dyn Any>>,
    source: String,
    next_id: usize,
}

impl Context {
    pub fn new(source: impl Into<String>) -> Self {
        Context {
            cache: HashMap::new(),
            source: source.into(),
            next_id: 0,
        }
    }
}

pub struct Parser<T> {
    name: String,
    id: ParserId,
    raw_parser: RawParser<T>,
}

impl<T> Parser<T>
where
    T: Clone + 'static,
{
    fn new(name: String, raw_parser: RawParser<T>) -> Parser<T> {
        Parser {
            name,
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            raw_parser,
        }
    }

    pub fn parse(&self, pos: Pos, ctx: &mut Context) -> ParseResult<T> {
        let key = (self.id, pos);
        if let Some(v) = ctx.cache.get(&key) {
            return v.downcast_ref::<ParseResult<T>>().cloned().unwrap();
        }

        let r = (self.raw_parser)(&ctx.source, pos);
        ctx.cache.insert(key, Box::new(r.clone()));
        r
    }
}

pub fn satisfy(name: impl Into<String>, f: impl Fn(char) -> bool + 'static) -> Parser<char> {
    let name = name.into();
    let raw_parser = {
        let name = name.clone();
        Box::new(move |source: &str, pos| match source.chars().nth(pos) {
            Some(c) if f(c) => Ok((pos + 1, c)),
            Some(c) => Err(ParseError {
                source: source.to_string(),
                pos,
                reason: format!("expected {} got {}", name, c),
            }),
            None => Err(ParseError {
                source: source.to_string(),
                pos,
                reason: format!("expected {} got EOF", name),
            }),
        })
    };

    Parser::new(name, raw_parser)
}

pub fn any_char() -> Parser<char> {
    satisfy("any char", |_| true)
}

pub fn char(c: char) -> Parser<char> {
    satisfy(format!("char {}", c), move |x| x == c)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_any_char() {
        let any_char = any_char();

        let ctx = &mut Context::new("abc");
        debug_assert_eq!(any_char.parse(0, ctx), Ok((1, 'a')));
        let ctx = &mut Context::new("");
        debug_assert!(any_char.parse(0, ctx).is_err());
    }

    #[test]
    fn test_char() {
        let a = char('a');

        let ctx = &mut Context::new("abc");
        debug_assert_eq!(a.parse(0, ctx), Ok((1, 'a')));
        let ctx = &mut Context::new("");
        debug_assert!(a.parse(0, ctx).is_err());
        let ctx = &mut Context::new("bcd");
        debug_assert!(a.parse(0, ctx).is_err());
    }

    #[test]
    fn test_satisfy() {
        let a = satisfy("digit", |c| c.is_ascii_digit());

        let ctx = &mut Context::new("123");
        debug_assert_eq!(a.parse(0, ctx), Ok((1, '1')));
        let ctx = &mut Context::new("");
        debug_assert!(a.parse(0, ctx).is_err());
        let ctx = &mut Context::new("abc");
        debug_assert!(a.parse(0, ctx).is_err());
    }
}
