mod combinators;
mod context;

use std::fmt::Debug;
use std::rc::Rc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

pub use crate::combinators::*;
pub use crate::context::Context;

type ParserId = usize;
type Pos = usize;
type CacheKey = (ParserId, Pos);

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
    LeftRecursion,
    Result(ParseResult<T>),
}

#[derive(Clone)]
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
        static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

        Parser {
            name,
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            raw_parser,
        }
    }

    pub fn parse(&self, pos: Pos, ctx: &mut Context) -> ParseResult<T> {
        let key = (self.id, pos);

        if let Some(cached) = ctx.cache.get(&key) {
            let entry = cached
                .downcast_ref::<Entry<T>>()
                .cloned()
                .expect("failed to cast Any to Entry<T>");
            match entry {
                Entry::LeftRecursion => {
                    ctx.lr_stack.push(key);
                    ctx.schedule_cache_eviction(key);

                    return Err(ParseError {
                        source: ctx.source.iter().collect(),
                        pos,
                        reason: String::from("left recursion not resolved"),
                    });
                }
                Entry::Result(res) => {
                    return res;
                }
            }
        }

        ctx.cache.insert(key, Box::new(Entry::<T>::LeftRecursion));
        ctx.call_path.push(key);

        let mut result = (self.raw_parser)(pos, ctx);
        ctx.cache
            .insert(key, Box::new(Entry::Result(result.clone())));

        if let Some(nearest_lr_key) = ctx.lr_stack.last()
            && *nearest_lr_key == key
        {
            let mut best_res @ Ok((mut best_pos, _)) = result else {
                ctx.lr_stack.pop();
                debug_assert_eq!(ctx.call_path.pop(), Some(key));
                return result;
            };

            loop {
                ctx.execute_cache_eviction(key);

                if let new_res @ Ok((new_pos, _)) = (self.raw_parser)(pos, ctx)
                    && (best_pos < new_pos || (best_pos == new_pos && best_res.is_err()))
                {
                    best_pos = new_pos;
                    best_res = new_res.clone();
                    ctx.cache
                        .insert(key, Box::new(Entry::Result(best_res.clone())));
                } else {
                    break;
                }
            }

            debug_assert_eq!(ctx.lr_stack.pop(), Some(key));
            result = best_res
        }

        debug_assert_eq!(ctx.call_path.pop(), Some(key));
        result
    }

    pub fn run(&self, source: impl Into<String>) -> Result<T, ParseError> {
        let ctx = &mut Context::new(source);
        match self.parse(0, ctx) {
            Ok((_, val)) => Ok(val),
            Err(e) => Err(e),
        }
    }
}
