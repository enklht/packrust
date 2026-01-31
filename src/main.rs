use packrat::*;

fn main() {
    let a = satisfy("char a", |c| c == 'a');

    let mut ctx = Context::new("abc");
    println!("{:?}", a.parse(0, &mut ctx))
}

