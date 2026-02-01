use packrat::*;

#[derive(Debug, Clone)]
enum Operator {
    Add,
    Mul,
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
                // Operator::Sub => left.eval() - right.eval(),
                Operator::Mul => left.eval() * right.eval(),
                // Operator::Div => left.eval() / right.eval(),
            },
            Expr::Literal(n) => *n,
        }
    }
}

fn main() {
    let expr: Parser<Expr> = lazy("expr", move |expr| {
        let term: Parser<Expr> = {
            let expr = expr.clone();
            lazy("term", move |term| {
                let digit = satisfy("digit", |c| c.is_ascii_digit());
                let int = digit
                    .many()
                    .try_map(|c| c.iter().collect::<String>().parse::<i32>().ok())
                    .map(Expr::Literal)
                    .rename("int");
                let factor = int
                    .or(char('(').andr(expr.clone()).andl(char(')')))
                    .rename("factor");
                term.andl(char('*'))
                    .and(factor.clone())
                    .map(|(left, right)| Expr::Binary {
                        op: Operator::Mul,
                        left: Box::new(left),
                        right: Box::new(right),
                    })
                    .or(factor)
            })
        };

        expr.andl(char('+'))
            .and(term.clone())
            .map(|(left, right)| Expr::Binary {
                op: Operator::Add,
                left: Box::new(left),
                right: Box::new(right),
            })
            .or(term)
    });

    let mut ctx = Context::new("5*6*7+1*2+3*4");
    let ast = expr.parse(0, &mut ctx);
    println!("{:?}", ast);
    println!("{}", ast.unwrap().1.eval());
}
