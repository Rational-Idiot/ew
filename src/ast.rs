use crate::interpreter::Val;

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Function {
        name: String,
        params: Vec<String>,
        body: Vec<Stmt>,
    },

    Return(Expr),
    Assignment {
        name: String,
        value: Expr,
    },
    Reassignment {
        target: AssignmentTarget,
        value: Expr,
    },
    Expr(Expr),
}

#[derive(Debug, PartialEq, Clone)]
pub enum AssignmentTarget {
    Ident(String),
    ArrayAccess {
        name: String,
        indices: Vec<Box<Expr>>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(String),
    Array(Vec<Expr>),
    ArrayAccess {
        name: String,
        indices: Vec<Box<Expr>>,
    },
    Var(String),

    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
    },
    Binary {
        op: BinaryOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    Call {
        name: String,
        args: Vec<Expr>,
    },
    If {
        cond: Box<Expr>,
        then: Vec<Stmt>,
        else_: Vec<Stmt>,
    },
    While {
        cond: Box<Expr>,
        body: Vec<Stmt>,
    },
    For {
        var: String,
        start: Box<Expr>,
        end: Box<Expr>,
        body: Vec<Stmt>,
    },
    Block(Vec<Stmt>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOp {
    Neg,
    Not,
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,

    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,

    And,
    Or,
}
