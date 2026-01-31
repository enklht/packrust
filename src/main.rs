use packrat::*;

#[derive(Debug, Clone)]
enum Operator {
    Add,
    Sub,
    Mul,
    Div,
}

#[derive(Debug, Clone)]
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
    let a = char('a').map(|_| Expr::Literal(1));

    let expr = lazy("expr", move |expr| {
        a.clone()
            .andl(char('+'))
            .and(expr)
            .map(|(left, right)| Expr::Binary {
                op: Operator::Add,
                left: Box::new(left),
                right: Box::new(right),
            })
            .or(a.clone())
        // expr.andl(char('+'))
        //     .and(a.clone())
        //     .map(|(left, right)| Expr::Binary {
        //         op: Operator::Add,
        //         left: Box::new(left),
        //         right: Box::new(right),
        //     })
        //     .or(a.clone())
    });

    let mut ctx = Context::new("a+a");
    println!("{:?}", expr.parse(0, &mut ctx))
}
