use std::any::Any;
use std::collections::HashMap;
use std::fmt::Debug;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};

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
type RawParser<T> = Rc<dyn Fn(Pos, &mut Context) -> ParseResult<T>>;

#[derive(Debug, Clone)]
enum Entry<T> {
    Initial,
    InProgress(ParseResult<T>),
    Fixed(ParseResult<T>),
}

pub struct Context {
    cache: HashMap<(ParserId, Pos), Rc<dyn Any>>,
    source: String,
    lr_stack: Vec<ParserId>,
}

impl Context {
    pub fn new(source: impl Into<String>) -> Self {
        Context {
            cache: HashMap::new(),
            source: source.into(),
            lr_stack: Vec::new(),
        }
    }
}

#[derive(Clone)]
pub struct Parser<T> {
    name: String,
    id: ParserId,
    raw_parser: RawParser<T>,
}

impl<T> Parser<T>
where
    T: Clone + Debug + 'static,
{
    fn new(name: String, raw_parser: RawParser<T>) -> Parser<T> {
        Parser {
            name,
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            raw_parser,
        }
    }

    pub fn parse(&self, pos: Pos, ctx: &mut Context) -> ParseResult<T> {
        println!(">>> {} is called at pos {}", self.name, pos);
        let key = (self.id, pos);
        if let Some(cached) = ctx.cache.get(&key) {
            let entry = cached.downcast_ref::<Entry<T>>().cloned().unwrap();
            match entry {
                Entry::Initial => {
                    println!("\tleft recursion detected!");
                    println!("\tname: {}", self.name);
                    if ctx.lr_stack.last().is_none_or(|id| *id != self.id) {
                        ctx.lr_stack.push(self.id);
                    }
                    return Err(ParseError {
                        source: ctx.source.clone(),
                        pos,
                        reason: String::from("Initial placeholder"),
                    });
                }
                Entry::InProgress(res) => {
                    return res;
                }
                Entry::Fixed(res) => {
                    return res;
                }
            }
        }

        // println!("\tcache miss {:?}", key);
        ctx.cache.insert(key, Rc::new(Entry::<T>::Initial));

        let mut best_pos = pos;
        let mut best_res = Err(ParseError {
            source: ctx.source.clone(),
            pos,
            reason: String::from("Initial placeholder"),
        });

        loop {
            println!("trying {} at {}", self.name, pos);
            if let Ok((new_pos, val)) = (self.raw_parser)(pos, ctx)
                && (best_pos < new_pos || (best_pos == new_pos && best_res.is_err()))
            {
                println!("updated {}", self.name);
                best_pos = new_pos;
                best_res = Ok((new_pos, val.clone()));
                match ctx.lr_stack.last() {
                    Some(id) if *id == self.id => {
                        println!("updating cache to {:?}", val);
                        ctx.cache
                            .insert(key, Rc::new(Entry::InProgress(best_res.clone())));
                    }
                    None => {
                        println!("updating cache to {:?}", val);
                        ctx.cache
                            .insert(key, Rc::new(Entry::InProgress(best_res.clone())));
                    }
                    _ => {}
                }
            } else {
                break;
            }
        }

        match ctx.lr_stack.last() {
            Some(id) if *id == self.id => {
                ctx.cache
                    .insert(key, Rc::new(Entry::Fixed(best_res.clone())));
                ctx.lr_stack.pop();
                println!("\tleft recursion resolved!");
                println!("\tname: {}", self.name);
            }
            None => {
                ctx.cache
                    .insert(key, Rc::new(Entry::Fixed(best_res.clone())));
            }
            _ => {
                ctx.cache.remove(&key);
            }
        }
        best_res
    }

    pub fn map<S: Clone + Debug + 'static>(self, f: impl Fn(T) -> S + 'static) -> Parser<S> {
        let Parser {
            name, raw_parser, ..
        } = self;
        let raw_parser = Rc::new(move |pos, ctx: &mut Context| {
            let (pos, val) = raw_parser(pos, ctx)?;
            Ok((pos, f(val)))
        });
        Parser::new(name, raw_parser)
    }

    pub fn rename(self, name: impl Into<String>) -> Parser<T> {
        Parser {
            name: name.into(),
            id: self.id,
            raw_parser: self.raw_parser,
        }
    }

    pub fn try_map<S: Clone + Debug + 'static>(
        self,
        f: impl Fn(T) -> Option<S> + 'static,
    ) -> Parser<S> {
        let Parser {
            name, raw_parser, ..
        } = self;
        let raw_parser = Rc::new(move |pos, ctx: &mut Context| {
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

    pub fn and<S: Clone + Debug + 'static>(self, right: Parser<S>) -> Parser<(T, S)> {
        let name = format!("({}{})", self.name, right.name);
        let raw_parser = Rc::new(move |pos, ctx: &mut Context| {
            let (pos, left_result) = self.parse(pos, ctx)?;
            let (pos, right_result) = right.parse(pos, ctx)?;
            Ok((pos, (left_result, right_result)))
        });
        Parser::new(name, raw_parser)
    }

    pub fn andl<S: Clone + Debug + 'static>(self, right: Parser<S>) -> Parser<T> {
        self.and(right).map(|(left, _)| left)
    }

    pub fn andr<S: Clone + Debug + 'static>(self, right: Parser<S>) -> Parser<S> {
        self.and(right).map(|(_, right)| right)
    }

    pub fn many(self) -> Parser<Vec<T>> {
        let name = format!("({}*)", self.name);
        let raw_parser = Rc::new(move |pos, ctx: &mut Context| {
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

    pub fn opt(self) -> Parser<Option<T>> {
        let name = format!("({}?)", self.name);
        let raw_parser = Rc::new(move |pos, ctx: &mut Context| match self.parse(pos, ctx) {
            Ok((pos, val)) => Ok((pos, Some(val))),
            Err(_) => Ok((pos, None)),
        });
        Parser::new(name, raw_parser)
    }

    pub fn or(self, right: Parser<T>) -> Parser<T> {
        let name = format!("({}/{})", self.name, right.name);
        let raw_parser = Rc::new(move |pos, ctx: &mut Context| {
            let e1 = match self.parse(pos, ctx) {
                ok @ Ok(_) => return ok,
                Err(e) => e,
            };

            let e2 = match right.parse(pos, ctx) {
                ok @ Ok(_) => return ok,
                Err(e) => e,
            };

            if e1.pos >= e2.pos { Err(e1) } else { Err(e2) }
        });

        Parser::new(name, raw_parser)
    }
}

pub fn satisfy(name: impl Into<String>, f: impl Fn(char) -> bool + 'static) -> Parser<char> {
    let name = name.into();
    let raw_parser = {
        let name = name.clone();
        Rc::new(
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
    satisfy("(any char)", |_| true)
}

pub fn char(c: char) -> Parser<char> {
    satisfy(format!("'{}'", c), move |x| x == c)
}

pub fn lazy<T: Clone + Debug + 'static>(
    name: impl Into<String>,
    get_parser: impl Fn(Parser<T>) -> Parser<T> + 'static,
) -> Parser<T> {
    use std::cell::OnceCell;
    use std::rc::Rc;

    let name = name.into();
    let cell = Rc::new(OnceCell::new());
    let cell_for_parse = cell.clone();

    let placeholder = Parser::new(
        name,
        Rc::new(move |pos, ctx: &mut Context| {
            let real: &Parser<T> = cell_for_parse.get().expect("uninitialized lazy parser");
            real.parse(pos, ctx)
        }),
    );

    let real = get_parser(placeholder.clone());

    let _ = cell.set(real);

    placeholder
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

    #[test]
    fn test_opt() {
        let digit = satisfy("digit", |c| c.is_ascii_digit());
        let a = char('a');
        let p = a.opt().andr(digit);

        let ctx = &mut Context::new("a1c");
        assert_eq!(p.parse(0, ctx), Ok((2, '1')));
        let ctx = &mut Context::new("1c");
        assert_eq!(p.parse(0, ctx), Ok((1, '1')));
        let ctx = &mut Context::new("abc");
        assert!(p.parse(0, ctx).is_err());
        let ctx = &mut Context::new("");
        assert!(p.parse(0, ctx).is_err());
    }

    #[test]
    fn test_or() {
        let digit = satisfy("digit", |c| c.is_ascii_digit());
        let a = char('a');
        let p = a.or(digit);

        let ctx = &mut Context::new("abc");
        assert_eq!(p.parse(0, ctx), Ok((1, 'a')));
        let ctx = &mut Context::new("1bc");
        assert_eq!(p.parse(0, ctx), Ok((1, '1')));
        let ctx = &mut Context::new("bcd");
        assert!(p.parse(0, ctx).is_err());
        let ctx = &mut Context::new("");
        assert!(p.parse(0, ctx).is_err());
    }
}
