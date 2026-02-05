// example that is claimed fail with Warth et al.’s method
// source: Packrat Parsers Can Support Multiple Left-recursive Calls at the Same Position
// doi: 10.2197/ipsjjip.29.174

use packrust::*;

fn main() {
    env_logger::init();

    // S -> A '-' A
    // A -> B 'b' / 'b'
    // B -> B 'a' / A 'a'
    let s = {
        let a = lazy("A", |a| {
            let b = {
                let a = a.clone();
                lazy("B", move |b| {
                    b.andr(char('a')).or(a.clone().andr(char('a')))
                })
            };
            b.andr(char('b')).or(char('b'))
        });

        a.clone().andr(char('-')).andr(a)
    }
    .end()
    .map(|_| "parse success");

    let res = s.run("baab-baab");
    println!("{:?}", res);
}
