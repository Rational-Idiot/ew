#![allow(unused)]
use std::fmt::format;

use pest::{Parser, iterators::Pair};

use crate::ast::{AssignmentTarget, BinaryOp, Expr, Stmt, UnaryOp};

#[derive(pest_derive::Parser)]
#[grammar = "./grammar.pest"]
struct LangParser;

pub fn parse(source: &str) -> Result<Vec<Stmt>, String> {
    let pairs =
        LangParser::parse(Rule::Program, source).map_err(|e| format!("Parsing error {}", e))?;

    let mut program = Vec::new();
    for pair in pairs {
        match pair.as_rule() {
            Rule::Stmt => program.push(parse_stmt(pair)?),
            Rule::EOI => {}
            _ => {}
        }
    }
    Ok(program)
}

fn parse_stmt(pair: Pair<Rule>) -> Result<Stmt, String> {
    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::Function => parse_func(inner),
        Rule::Return => parse_ret(inner),
        Rule::Assignment => parse_ass(inner),
        Rule::Reassignment => parse_reass(inner),
        Rule::Expr => Ok(Stmt::Expr(parse_expr(inner)?)),
        Rule::Conditional | Rule::WhileLoop | Rule::Comp | Rule::ForLoop => {
            Ok(Stmt::Expr(parse_expr(inner)?))
        }
        r => Err(format!("Unexpected statement rule: {:#?}", r)),
    }
}

fn parse_expr(pair: Pair<Rule>) -> Result<Expr, String> {
    match pair.as_rule() {
        Rule::Expr => {
            let inner = pair.into_inner().next().unwrap();
            parse_expr(inner)
        }
        Rule::Conditional => parse_conditional(pair),
        Rule::Comp => parse_binary(pair),
        Rule::Unary => parse_unary(pair),
        Rule::WhileLoop => parse_while(pair),
        Rule::ForLoop => parse_for(pair),
        Rule::Range => parse_for(pair),
        Rule::Additive => parse_binary(pair),
        Rule::Multiplicative => parse_binary(pair),
        Rule::Call => parse_call(pair),
        Rule::Literal => parse_literal(pair),
        Rule::Ident => Ok(Expr::Var(pair.as_str().to_string())),
        Rule::ArrayAccess => parse_access(pair),
        Rule::Block => {
            let stmts = parse_block(pair)?;
            Ok(Expr::Block(stmts))
        }
        r => Err(format!("Unexpected expression rule: {:#?}", r)),
    }
}

fn parse_conditional(pair: Pair<Rule>) -> Result<Expr, String> {
    let mut inner = pair.into_inner();
    let cond = Box::new(parse_expr(inner.next().unwrap())?);
    let then = parse_block(inner.next().unwrap())?;
    let else_ = if let Some(else_branch) = inner.next() {
        parse_block(else_branch)?
    } else {
        vec![]
    };

    Ok(Expr::If { cond, then, else_ })
}

fn parse_binary(pair: Pair<Rule>) -> Result<Expr, String> {
    let mut inner = pair.into_inner();
    let mut lhs = parse_expr(inner.next().unwrap())?;
    while let Some(op_pair) = inner.next() {
        let op = match op_pair.as_str() {
            "+" => BinaryOp::Add,
            "-" => BinaryOp::Sub,
            "*" => BinaryOp::Mul,
            "/" => BinaryOp::Div,
            "%" => BinaryOp::Mod,
            "==" => BinaryOp::Eq,
            "!=" => BinaryOp::Ne,
            "<" => BinaryOp::Lt,
            "<=" => BinaryOp::Le,
            ">=" => BinaryOp::Ge,
            ">" => BinaryOp::Gt,
            "&&" => BinaryOp::And,
            "||" => BinaryOp::Or,
            e => return Err(format!("Unexprected Operator: {}", e)),
        };

        let rhs = parse_expr(inner.next().unwrap())?;

        lhs = Expr::Binary {
            op: op,
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        };
    }

    Ok(lhs)
}

fn parse_unary(pair: Pair<Rule>) -> Result<Expr, String> {
    let mut inner = pair.into_inner();
    let first = inner.next().unwrap();

    match first.as_rule() {
        Rule::UnaryOp => {
            let op = match first.as_str() {
                "-" => UnaryOp::Neg,
                "!" => UnaryOp::Not,
                o => return Err(format!("Unexpected Unary Operator: {}", o)),
            };

            let expr = parse_expr(inner.next().unwrap())?;
            Ok(Expr::Unary {
                op,
                expr: Box::new(expr),
            })
        }
        _ => parse_expr(first),
    }
}

fn parse_while(pair: Pair<Rule>) -> Result<Expr, String> {
    let mut inner = pair.into_inner();
    let cond = Box::new(parse_expr(inner.next().unwrap())?);
    let body = parse_block(inner.next().unwrap())?;

    Ok(Expr::While { cond, body })
}

fn parse_for(pair: Pair<Rule>) -> Result<Expr, String> {
    let mut inner = pair.into_inner();

    let var = inner.next().unwrap().as_str().to_owned();
    let (start, end) = parse_range(inner.next().unwrap())?;
    let body = parse_block(inner.next().unwrap())?;

    Ok(Expr::For {
        var,
        start,
        end,
        body,
    })
}

fn parse_range(pair: Pair<Rule>) -> Result<(Box<Expr>, Box<Expr>), String> {
    if pair.as_rule() != Rule::Range {
        return Err(format!("Expected Range, got {:?}", pair.as_rule()));
    }

    let mut inner = pair.into_inner();

    let start = Box::new(parse_expr(inner.next().ok_or("Missing range start")?)?);

    let end = Box::new(parse_expr(inner.next().ok_or("Missing range end")?)?);

    Ok((start, end))
}

fn parse_call(pair: Pair<Rule>) -> Result<Expr, String> {
    let mut inner = pair.into_inner();
    let first = inner.next().unwrap();

    let mut expr = parse_expr(first)?;

    for arg in inner {
        if arg.as_rule() == Rule::CallArgs {
            let args: Vec<Expr> = arg
                .into_inner()
                .map(|p| parse_expr(p))
                .collect::<Result<_, _>>()?;

            if let Expr::Var(name) = expr {
                expr = Expr::Call { name, args }
            } else {
                return Err("expected a named function to be called".to_string());
            }
        }
    }

    Ok(expr)
}

fn parse_literal(pair: Pair<Rule>) -> Result<Expr, String> {
    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::Int => Ok(Expr::Int(inner.as_str().parse().unwrap())),
        Rule::Bool => Ok(Expr::Bool(inner.as_str() == "true")),
        Rule::Float => Ok(Expr::Float(inner.as_str().parse().unwrap())),
        Rule::String => {
            let s = inner.as_str().to_string();
            let s = &s[1..s.len() - 1];
            Ok(Expr::Str(s.to_string()))
        }
        Rule::Array => {
            let elements = inner
                .into_inner()
                .map(|p| parse_expr(p))
                .collect::<Result<_, _>>()?;
            Ok(Expr::Array(elements))
        }
        e => Err(format!("Expected a Literal: {:?}", e)),
    }
}

fn parse_access(pair: Pair<Rule>) -> Result<Expr, String> {
    let mut inner = pair.into_inner();
    let name = inner.next().unwrap().as_str().to_string();

    // Trash syntax for idiomacy
    // We first map eval to flesh out all indices
    // And then Box then with the second map
    // ::<> is called the 'turbofish' for some reason
    let indices = inner
        .map(|e| parse_expr(e).map(Box::new))
        .collect::<Result<Vec<Box<Expr>>, String>>()?;

    Ok(Expr::ArrayAccess { name, indices })
}

fn parse_ret(pair: Pair<Rule>) -> Result<Stmt, String> {
    let expr = pair.into_inner().next().unwrap();
    Ok(Stmt::Return(parse_expr(expr)?))
}

fn parse_func(pair: Pair<Rule>) -> Result<Stmt, String> {
    let mut inner = pair.into_inner();
    let name = inner.next().unwrap().as_str().into();

    let mut params = Vec::new();
    let mut body = Vec::new();

    for item in inner {
        match item.as_rule() {
            Rule::Ident => params.push(item.as_str().into()),
            Rule::Block => body = parse_block(item)?,
            _ => {}
        }
    }
    Ok(Stmt::Function { name, params, body })
}

fn parse_block(pair: Pair<Rule>) -> Result<Vec<Stmt>, String> {
    let mut body = Vec::new();
    let inner = pair.into_inner();

    for item in inner {
        if item.as_rule() == Rule::Stmt {
            body.push(parse_stmt(item)?);
        }
    }
    Ok(body)
}

fn parse_ass(pair: Pair<Rule>) -> Result<Stmt, String> {
    let mut inner = pair.into_inner();
    let name = inner.next().unwrap().as_str().to_string();
    let value = parse_expr(inner.next().unwrap())?;

    Ok(Stmt::Assignment { name, value })
}

fn parse_reass(pair: Pair<Rule>) -> Result<Stmt, String> {
    let mut inner = pair.into_inner();

    let target_pair = inner.next().unwrap();
    let value = parse_expr(inner.next().unwrap())?;

    let target = match target_pair.as_rule() {
        Rule::Ident => AssignmentTarget::Ident(target_pair.as_str().to_string()),
        Rule::ArrayAccess => {
            let mut inner = target_pair.into_inner();
            let name = inner.next().unwrap().as_str().to_string();

            let indices = inner
                .map(|e| parse_expr(e).map(Box::new))
                .collect::<Result<_, _>>()?;

            AssignmentTarget::ArrayAccess { name, indices }
        }
        _ => {
            return Err(format!(
                "Unexpected Assignment Target: {:?}",
                target_pair.as_rule()
            ));
        }
    };

    Ok(Stmt::Reassignment { target, value })
}

#[cfg(test)]
mod tests {
    use std::fmt::Binary;

    use super::*;

    #[test]
    fn test_parse_literal() {
        let program = parse("42").unwrap();
        assert_eq!(program, vec![Stmt::Expr(Expr::Int(42))]);
    }

    #[test]
    fn test_parse_bool() {
        let program = parse("true").unwrap();
        assert_eq!(program, vec![Stmt::Expr(Expr::Bool(true))]);
    }

    #[test]
    fn test_parse_binary() {
        let program = parse("1 + 2").unwrap();
        assert_eq!(
            program,
            vec![Stmt::Expr(Expr::Binary {
                op: BinaryOp::Add,
                lhs: Box::new(Expr::Int(1)),
                rhs: Box::new(Expr::Int(2)),
            })]
        );
    }

    #[test]
    fn test_parse_assignment() {
        let program = parse("let x = 42").unwrap();
        assert_eq!(
            program,
            vec![Stmt::Assignment {
                name: "x".to_string(),
                value: Expr::Int(42),
            }]
        );
    }

    #[test]
    fn test_parse_function() {
        let program = parse("fn add(a, b) { return a + b }").unwrap();
        assert_eq!(
            program,
            vec![Stmt::Function {
                name: "add".to_string(),
                params: vec!["a".to_string(), "b".to_string()],
                body: vec![Stmt::Return(Expr::Binary {
                    op: BinaryOp::Add,
                    lhs: Box::new(Expr::Var("a".to_string())),
                    rhs: Box::new(Expr::Var("b".to_string())),
                })],
            }]
        );
    }

    #[test]
    fn test_parse_call() {
        let program = parse("add(1, 2)").unwrap();
        assert_eq!(
            program,
            vec![Stmt::Expr(Expr::Call {
                name: "add".to_string(),
                args: vec![Expr::Int(1), Expr::Int(2),],
            })]
        );
    }

    #[test]
    fn test_parse_conditional() {
        let program = parse("if (x < 10) { 1 } else { 2 }").unwrap();
        assert_eq!(
            program,
            vec![Stmt::Expr(Expr::If {
                cond: Box::new(Expr::Binary {
                    op: BinaryOp::Lt,
                    lhs: Box::new(Expr::Var("x".to_string())),
                    rhs: Box::new(Expr::Int(10)),
                }),
                then: vec![Stmt::Expr(Expr::Int(1))],
                else_: vec![Stmt::Expr(Expr::Int(2))],
            })]
        );
    }

    #[test]
    fn test_parse_while() {
        let program = parse("while (x < 10) { let x = x + 1 }").unwrap();
        assert_eq!(
            program,
            vec![Stmt::Expr(Expr::While {
                cond: Box::new(Expr::Binary {
                    op: BinaryOp::Lt,
                    lhs: Box::new(Expr::Var("x".to_string())),
                    rhs: Box::new(Expr::Int(10)),
                }),
                body: vec![Stmt::Assignment {
                    name: "x".to_string(),
                    value: Expr::Binary {
                        op: BinaryOp::Add,
                        lhs: Box::new(Expr::Var("x".to_string())),
                        rhs: Box::new(Expr::Int(1)),
                    },
                }],
            })]
        );
    }

    #[test]
    fn test_parse_fibonacci() {
        let source = r#"
        fn fib(n) {
            if (n < 2) {
                return n
            } else {
                return fib(n - 1) + fib(n - 2)
            }
        }
        fib(10)
    "#;

        let program = parse(source).unwrap();

        assert_eq!(
            program,
            vec![
                Stmt::Function {
                    name: "fib".to_string(),
                    params: vec!["n".to_string()],
                    body: vec![Stmt::Expr(Expr::If {
                        cond: Box::new(Expr::Binary {
                            op: BinaryOp::Lt,
                            lhs: Box::new(Expr::Var("n".to_string())),
                            rhs: Box::new(Expr::Int(2)),
                        }),
                        then: vec![Stmt::Return(Expr::Var("n".to_string()))],
                        else_: vec![Stmt::Return(Expr::Binary {
                            op: BinaryOp::Add,
                            lhs: Box::new(Expr::Call {
                                name: "fib".to_string(),
                                args: vec![Expr::Binary {
                                    op: BinaryOp::Sub,
                                    lhs: Box::new(Expr::Var("n".to_string())),
                                    rhs: Box::new(Expr::Int(1)),
                                }],
                            }),
                            rhs: Box::new(Expr::Call {
                                name: "fib".to_string(),
                                args: vec![Expr::Binary {
                                    op: BinaryOp::Sub,
                                    lhs: Box::new(Expr::Var("n".to_string())),
                                    rhs: Box::new(Expr::Int(2)),
                                }],
                            }),
                        })],
                    })],
                },
                Stmt::Expr(Expr::Call {
                    name: "fib".to_string(),
                    args: vec![Expr::Int(10)],
                })
            ]
        );
    }
}
