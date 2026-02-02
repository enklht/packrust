# 🐀 packrust

A packrat parser combinator in Rust handles direct and indirect left recursion correctly.

## Features

- 🧠 Memoized parsing
- 🔄 Correctly handles left recursion (both direct and indirect)
- 🔧 Combinator-based API: `and`, `or`, `map`, `many`, `opt`, `lazy`, and more
- 📦 Small dependencies
  - `rustc_hash` for faster hashmap
  - `log`, `env_logger` for logging

## Example

```rust
use packrust::*;

let expr: Parser<i32> = lazy("expr", |expr| {
    let digit = satisfy("digit", |c| c.is_ascii_digit());
    let int = digit.many().try_map(|c| c.iter().collect::<String>().parse().ok());

    expr.andl(char('+')).and(int)
        .map(|(a, b)| a + b)
        .or(int)
});

let result = expr.run("1+2+3");
```

## Educational Purpose

This is a learning project exploring packrat parsing with left recursion support.

## References

Heavily inspired by:

- <https://zenn.dev/fj68/articles/b789c67f6b6e38> (Japanese)

Other references

- Warth et al., Packrat parsers can support left recursion (PEPM 2008)
- Umeda & Maeda, Packrat Parsers Can Support Multiple Left-recursive Calls (2021)
- Ford, Packrat Parsing series (2006-2007)
- Ford, Parsing Expression Grammars
