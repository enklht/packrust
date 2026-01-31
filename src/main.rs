use packrat::*;

enum Operator {
    Add,
    Sub,
    Mul,
    Div,
}

enum Expr {
    Binary {
        op: Operator,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Literal(i32),
}

impl Expr {
    fn eval(&self) -> i32 {
        match self {
            Expr::Binary { op, left, right } => match op {
                Operator::Add => left.eval() + right.eval(),
                Operator::Sub => left.eval() - right.eval(),
                Operator::Mul => left.eval() * right.eval(),
                Operator::Div => left.eval() / right.eval(),
            },
            Expr::Literal(n) => *n,
        }
    }
}

fn main() {
    let a = satisfy("char a", |c| c == 'a');

    let mut ctx = Context::new("abc");
    println!("{:?}", a.parse(0, &mut ctx))
}
