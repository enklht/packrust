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
type RawParser<T> = Box<dyn Fn(Pos, &mut Context) -> ParseResult<T>>;

pub struct Context {
    cache: HashMap<(ParserId, Pos), Box<dyn Any>>,
    source: String,
}

impl Context {
    pub fn new(source: impl Into<String>) -> Self {
        Context {
            cache: HashMap::new(),
            source: source.into(),
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

        let r = (self.raw_parser)(pos, ctx);
        ctx.cache.insert(key, Box::new(r.clone()));
        r
    }

    pub fn map<S: Clone + 'static>(self, f: impl Fn(T) -> S + 'static) -> Parser<S> {
        let Parser {
            name, raw_parser, ..
        } = self;
        let raw_parser = Box::new(move |pos, ctx: &mut Context| {
            let (pos, val) = raw_parser(pos, ctx)?;
            Ok((pos, f(val)))
        });
        Parser::new(name, raw_parser)
    }

    pub fn try_map<S: Clone + 'static>(self, f: impl Fn(T) -> Option<S> + 'static) -> Parser<S> {
        let Parser {
            name, raw_parser, ..
        } = self;
        let raw_parser = Box::new(move |pos, ctx: &mut Context| {
            let (pos, val) = raw_parser(pos, ctx)?;
            let Some(val) = f(val) else {
                return Err(ParseError {
                    source: ctx.source.clone(),
                    pos,
                    reason: String::from("try map failed: got None"),
                });
            };
            Ok((pos, val))
        });
        Parser::new(name, raw_parser)
    }

    pub fn and<S: Clone + 'static>(self, right: Parser<S>) -> Parser<(T, S)> {
        let name = format!("{} and {}", self.name, right.name);
        let raw_parser = Box::new(move |pos, ctx: &mut Context| {
            let (pos, left_result) = self.parse(pos, ctx)?;
            let (pos, right_result) = right.parse(pos, ctx)?;
            Ok((pos, (left_result, right_result)))
        });
        Parser::new(name, raw_parser)
    }

    pub fn andl<S: Clone + 'static>(self, right: Parser<S>) -> Parser<T> {
        self.and(right).map(|(left, _)| left)
    }

    pub fn andr<S: Clone + 'static>(self, right: Parser<S>) -> Parser<S> {
        self.and(right).map(|(_, right)| right)
    }

    pub fn many(self) -> Parser<Vec<T>> {
        let name = format!("many {}", self.name);
        let raw_parser = Box::new(move |pos, ctx: &mut Context| {
            let mut acc = Vec::new();
            let mut pos = pos;

            while let Ok((new_pos, val)) = self.parse(pos, ctx) {
                pos = new_pos;
                acc.push(val);
            }

            Ok((pos, acc))
        });

        Parser::new(name, raw_parser)
    }
}

pub fn satisfy(name: impl Into<String>, f: impl Fn(char) -> bool + 'static) -> Parser<char> {
    let name = name.into();
    let raw_parser = {
        let name = name.clone();
        Box::new(
            move |pos, ctx: &mut Context| match ctx.source.chars().nth(pos) {
                Some(c) if f(c) => Ok((pos + 1, c)),
                Some(c) => Err(ParseError {
                    source: ctx.source.to_string(),
                    pos,
                    reason: format!("expected {} got {}", name, c),
                }),
                None => Err(ParseError {
                    source: ctx.source.to_string(),
                    pos,
                    reason: format!("expected {} got EOF", name),
                }),
            },
        )
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
        assert_eq!(any_char.parse(0, ctx), Ok((1, 'a')));
        let ctx = &mut Context::new("");
        assert!(any_char.parse(0, ctx).is_err());
    }

    #[test]
    fn test_char() {
        let a = char('a');

        let ctx = &mut Context::new("abc");
        assert_eq!(a.parse(0, ctx), Ok((1, 'a')));
        let ctx = &mut Context::new("bcd");
        assert!(a.parse(0, ctx).is_err());
        let ctx = &mut Context::new("");
        assert!(a.parse(0, ctx).is_err());
    }

    #[test]
    fn test_satisfy() {
        let digit = satisfy("digit", |c| c.is_ascii_digit());

        let ctx = &mut Context::new("123");
        assert_eq!(digit.parse(0, ctx), Ok((1, '1')));
        let ctx = &mut Context::new("abc");
        assert!(digit.parse(0, ctx).is_err());
        let ctx = &mut Context::new("");
        assert!(digit.parse(0, ctx).is_err());
    }

    #[test]
    fn test_and() {
        let abc = char('a').and(char('b')).and(char('c'));

        let ctx = &mut Context::new("abc");
        assert_eq!(abc.parse(0, ctx), Ok((3, (('a', 'b'), 'c'))));
        let ctx = &mut Context::new("abcd");
        assert_eq!(abc.parse(0, ctx), Ok((3, (('a', 'b'), 'c'))));
        let ctx = &mut Context::new("123");
        assert!(abc.parse(0, ctx).is_err());
        let ctx = &mut Context::new("a123");
        assert!(abc.parse(0, ctx).is_err());
        let ctx = &mut Context::new("");
        assert!(abc.parse(0, ctx).is_err());
    }

    #[test]
    fn test_many() {
        let digit = satisfy("digit", |c| c.is_ascii_digit());
        let int = digit
            .many()
            .try_map(|c| c.iter().collect::<String>().parse::<i32>().ok());

        let ctx = &mut Context::new("1234");
        assert_eq!(int.parse(0, ctx), Ok((4, 1234)));
        let ctx = &mut Context::new("12a4");
        assert_eq!(int.parse(0, ctx), Ok((2, 12)));
        let ctx = &mut Context::new("abc");
        assert!(int.parse(0, ctx).is_err());
        let ctx = &mut Context::new("");
        assert!(int.parse(0, ctx).is_err());
    }
}
