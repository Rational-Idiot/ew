use crate::{
    interpreter::{Interpreter, Val},
    parser::parse,
};

pub mod ast;
pub mod interpreter;
pub mod jit;
pub mod parser;

extern crate pest;

#[macro_use]
extern crate pest_derive;

// pub use ast::{Expr, Program, Stmt};
// pub use interpreter::{Interpreter, Val};
// pub use parser::parse;

pub fn run(source: &str) -> Result<Val, String> {
    let program = parse(source)?;
    let mut interpreter = Interpreter::new();
    interpreter.run(&program)
}
