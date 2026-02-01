// example that is claimed fail with Warth et al.’s method
// source: Packrat Parsers Can Support Multiple Left-recursive Calls at the Same Position
// doi: 10.2197/ipsjjip.29.174

use packrust::*;

fn main() {
    // S -> A '-' A
    // A -> B 'b' / 'b'
    // B -> B 'a' / A 'a'

    let parser = {
        let a = lazy("A", |a| {
            let b = {
                let a = a.clone();
                lazy("B", move |b| {
                    b.and(char('a'))
                        .map(|_| ())
                        .or(a.clone().and(char('a')).map(|_| ()))
                })
            };
            b.andr(char('b')).or(char('b')).map(|_| ())
        });

        a.clone().and(char('-')).and(a)
    }
    .end()
    .map(|_| "parse success");

    let mut ctx = Context::new("baab-baab");
    let res = parser.parse(0, &mut ctx);
    println!("{:?}", res);
}
