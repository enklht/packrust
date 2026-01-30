#[derive(Debug, PartialEq, Clone)]
struct ParseInput {
    source: String,
    pos: usize,
}

impl ParseInput {
    fn new(text: impl Into<String>) -> Self {
        ParseInput {
            source: text.into(),
            pos: 0,
        }
    }

    fn peek(&self) -> Option<char> {
        self.source.chars().nth(self.pos)
    }

    fn next(self) -> Self {
        ParseInput {
            source: self.source,
            pos: self.pos + 1,
        }
    }
}

#[derive(Debug, PartialEq)]
struct ParseError {
    input: ParseInput,
    reason: String,
}

type ParseResult<T> = Result<(ParseInput, T), ParseError>;

trait Parser<T>: Fn(ParseInput) -> ParseResult<T> + Sized + Clone {
    fn map<S>(self, f: impl Fn(T) -> S + Clone) -> impl Parser<S> {
        move |input| {
            let (input, result) = self(input)?;
            Ok((input, f(result)))
        }
    }

    fn and<S>(self, right: impl Parser<S>) -> impl Parser<(T, S)> {
        move |input| {
            let (input, left_result) = self(input)?;
            let (input, right_result) = right(input)?;
            Ok((input, (left_result, right_result)))
        }
    }

    fn andl<S>(self, right: impl Parser<S>) -> impl Parser<T> {
        let combined = self.and(right);
        move |input| match combined(input) {
            Ok((input, (l, _))) => Ok((input, l)),
            Err(e) => Err(e),
        }
    }

    fn andr<S>(self, right: impl Parser<S>) -> impl Parser<S> {
        let combined = self.and(right);
        move |input| match combined(input) {
            Ok((input, (_, r))) => Ok((input, r)),
            Err(e) => Err(e),
        }
    }

    fn many(self) -> impl Parser<Vec<T>> {
        move |mut input| {
            let mut acc = Vec::new();
            loop {
                let Ok((new_input, val)) = self(input.clone()) else {
                    break;
                };
                input = new_input;
                acc.push(val);
            }
            Ok((input, acc))
        }
    }

    fn many1(self) -> impl Parser<Vec<T>> {
        self.clone().and(self.many()).map(|(first, mut rest)| {
            let mut v = Vec::with_capacity(1 + rest.len());
            v.push(first);
            v.append(&mut rest);
            v
        })
    }

    fn opt(self) -> impl Parser<Option<T>> {
        move |input| match self(input.clone()) {
            Ok((input, val)) => Ok((input, Some(val))),
            Err(_) => Ok((input, None)),
        }
    }
}

impl<P, T> Parser<T> for P where P: Fn(ParseInput) -> ParseResult<T> + Sized + Clone {}

fn satisfy<F>(name: String, f: F) -> impl Parser<char>
where
    F: Fn(char) -> bool + Clone,
{
    move |input| match input.peek() {
        Some(c) if f(c) => Ok((input.next(), c)),
        Some(c) => Err(ParseError {
            input,
            reason: format!("expected {}, got {}", name, c),
        }),
        None => Err(ParseError {
            input,
            reason: format!("expected {}, got EOF", name),
        }),
    }
}

fn any_char(input: ParseInput) -> ParseResult<char> {
    satisfy(String::from("any_char"), |_| true)(input)
}

fn char(c: char) -> impl Parser<char> {
    satisfy(format!("char {}", c), move |x| x == c)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_any_char() {
        assert_eq!(
            any_char(ParseInput::new("abc")),
            Ok((
                ParseInput {
                    source: String::from("abc"),
                    pos: 1
                },
                'a'
            ))
        );
        assert!(matches!(
            any_char(ParseInput::new("")),
            Err(ParseError { .. })
        ));
    }

    #[test]
    fn test_char() {
        assert_eq!(
            char('a')(ParseInput::new("abc")),
            Ok((
                ParseInput {
                    source: String::from("abc"),
                    pos: 1
                },
                'a'
            ))
        );
        assert!(matches!(
            char('a')(ParseInput::new("bcd")),
            Err(ParseError { .. })
        ));
        assert!(matches!(
            char('a')(ParseInput::new("")),
            Err(ParseError { .. })
        ));
    }

    #[test]
    fn test_satisfy() {
        let digit = satisfy(String::from("digit"), |c| c.is_ascii_digit());
        assert_eq!(
            digit(ParseInput::new("123")),
            Ok((
                ParseInput {
                    source: String::from("123"),
                    pos: 1
                },
                '1'
            ))
        );
        assert!(matches!(
            digit(ParseInput::new("abc")),
            Err(ParseError { .. })
        ));
    }

    #[test]
    fn test_and() {
        let ab = char('a').and(char('b'));
        assert_eq!(
            ab(ParseInput::new("abc")),
            Ok((
                ParseInput {
                    source: String::from("abc"),
                    pos: 2
                },
                ('a', 'b')
            ))
        );
        assert!(matches!(ab(ParseInput::new("bcd")), Err(ParseError { .. })));
        assert!(matches!(ab(ParseInput::new("acd")), Err(ParseError { .. })));
    }

    #[test]
    fn test_abcx() {
        let digit =
            satisfy(String::from("digit"), |c| c.is_ascii_digit()).map(|c| c.to_digit(10).unwrap());
        let abc = char('a').and(char('b')).and(char('c'));
        let abcx = abc.andr(digit);

        assert!(matches!(abcx(ParseInput::new("abc8")), Ok((_, 8))));
    }

    #[test]
    fn test_many() {
        let digit =
            satisfy(String::from("digit"), |c| c.is_ascii_digit()).map(|c| c.to_digit(10).unwrap());
        let int = digit.many();

        assert!(matches!(
            int(ParseInput::new("12y")),
            Ok((_, vec)) if vec == vec![1, 2]
        ));
        assert!(matches!(
            int(ParseInput::new("abc")),
            Ok((_, vec)) if vec == vec![]
        ));
    }

    #[test]
    fn test_many1() {
        let digit =
            satisfy(String::from("digit"), |c| c.is_ascii_digit()).map(|c| c.to_digit(10).unwrap());
        let int = digit.many1();

        assert!(matches!(
            int(ParseInput::new("12y")),
            Ok((_, vec)) if vec == vec![1, 2]
        ));
        assert!(int(ParseInput::new("abc")).is_err());
    }

    #[test]
    fn test_opt() {
        let digit =
            satisfy(String::from("digit"), |c| c.is_ascii_digit()).map(|c| c.to_digit(10).unwrap());
        let a = char('a');
        let p = a.opt().andr(digit);

        assert!(matches!(p(ParseInput::new("a1c")), Ok((_, 1))));
        assert!(matches!(p(ParseInput::new("1c")), Ok((_, 1))));
        assert!(p(ParseInput::new("abc")).is_err());
    }
}
