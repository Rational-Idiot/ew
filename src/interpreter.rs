use std::{
    collections::HashMap,
    io::{self, Write},
};

use crate::ast::{AssignmentTarget, BinaryOp, Expr, Stmt, UnaryOp};

#[derive(Debug, Clone, PartialEq)]
pub enum Val {
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(String),
    Array(Vec<Val>),

    Function {
        params: Vec<String>,
        body: Vec<Stmt>,
    },

    Unit,
}

impl std::fmt::Display for Val {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Val::Int(n) => write!(f, "{}", n),
            Val::Float(n) => write!(f, "{}", n),
            Val::Bool(b) => write!(f, "{}", b),
            Val::Str(s) => write!(f, "{}", s),

            Val::Array(arr) => {
                write!(f, "[")?;
                let mut first = true;
                for elem in arr {
                    if !first {
                        write!(f, ", ")?;
                    }
                    first = false;
                    write!(f, "{}", elem)?;
                }
                write!(f, "]")
            }

            Val::Function { params, .. } => write!(f, "<function({})>", params.join(", ")),
            Val::Unit => write!(f, "()"),
        }
    }
}

struct Frame {
    local: HashMap<String, Val>,
    parent: Option<usize>,
}

pub struct Interpreter {
    global: HashMap<String, Val>,
    stack: Vec<Frame>,
}

enum Flow {
    Continue(Val),
    Return(Val),
}

impl Interpreter {
    pub fn new() -> Interpreter {
        Interpreter {
            global: HashMap::new(),
            stack: vec![Frame::new()],
        }
    }

    pub fn run(&mut self, source: &Vec<Stmt>) -> Result<Val, String> {
        let mut res = Val::Unit;
        for stmt in source {
            match self.exec_stmt(stmt)? {
                Flow::Continue(v) => res = v,
                Flow::Return(v) => return Ok(v),
            }
        }
        Ok(res)
    }

    fn exec_stmt(&mut self, stmt: &Stmt) -> Result<Flow, String> {
        match stmt {
            Stmt::Function { name, params, body } => {
                self.global.insert(
                    name.clone(),
                    Val::Function {
                        params: params.clone(),
                        body: body.clone(),
                    },
                );
                Ok(Flow::Continue(Val::Unit))
            }

            Stmt::Return(expr) => {
                let retval = self.eval_expr(expr)?;
                Ok(Flow::Return(retval))
            }

            Stmt::Assignment { name, value } => {
                let val = self.eval_expr(value)?;

                if let Some(exists) = self.lookup_mut(name) {
                    return Err(format!(
                        "The variable already exists: [{} = {}]",
                        name, exists
                    ));
                } else {
                    self.stack
                        .last_mut()
                        .expect("Call Stack Should Not Be Empty")
                        .local
                        .insert(name.clone(), val);
                }

                Ok(Flow::Continue(Val::Unit))
            }

            Stmt::Reassignment { target, value } => {
                let val = self.eval_expr(value)?;
                match target {
                    AssignmentTarget::Ident(name) => {
                        if let Some(exists) = self.lookup_mut(name) {
                            *exists = val;
                        } else {
                            return Err(format!("The variable [{}] does not exist", name));
                        }
                    }
                    AssignmentTarget::ArrayAccess { name, indices } => {
                        let evaluated_indices: Result<Vec<usize>, String> = indices
                            .iter()
                            .map(|expr| {
                                let idx_val = self.eval_expr(expr)?;
                                match idx_val {
                                    Val::Int(n) => Ok(n as usize),
                                    _ => Err(format!(
                                        "Array index must be an integer, got {:?}",
                                        idx_val
                                    )),
                                }
                            })
                            .collect();
                        let evaluated_indices = evaluated_indices?;
                        let var = self
                            .lookup_mut(name)
                            .ok_or_else(|| format!("The variable [{}] does not exist", name))?;
                        let mut cur = var;

                        for &idx in &evaluated_indices[..evaluated_indices.len() - 1] {
                            match cur {
                                Val::Array(arr) => {
                                    if idx >= arr.len() {
                                        return Err(format!("Array index out of bounds: {}", idx));
                                    }
                                    cur = &mut arr[idx];
                                }
                                _ => return Err(format!("Cannot index into {:?}", cur)),
                            }
                        }

                        let final_idx = evaluated_indices[evaluated_indices.len() - 1];
                        match cur {
                            Val::Array(arr) => {
                                if final_idx >= arr.len() {
                                    return Err(format!(
                                        "Array index out of bounds: {}",
                                        final_idx
                                    ));
                                }
                                arr[final_idx] = val;
                            }
                            Val::Str(s) => {
                                let mut chars: Vec<char> = s.chars().collect();
                                if final_idx >= chars.len() {
                                    return Err(format!(
                                        "String index out of bounds: {}",
                                        final_idx
                                    ));
                                }
                                match &val {
                                    Val::Str(new_char) => {
                                        let new_chars: Vec<char> = new_char.chars().collect();
                                        if new_chars.len() != 1 {
                                            return Err(format!(
                                                "Can only assign single character to string index, got string of length {}",
                                                new_chars.len()
                                            ));
                                        }
                                        chars[final_idx] = new_chars[0];
                                        *s = chars.into_iter().collect();
                                    }
                                    _ => {
                                        return Err(format!(
                                            "Can only assign string to string index, got {:?}",
                                            val
                                        ));
                                    }
                                }
                            }
                            _ => return Err(format!("Cannot index into {:?}", cur)),
                        }
                    }
                }
                Ok(Flow::Continue(Val::Unit))
            }

            Stmt::Expr(expr) => {
                let value = self.eval_expr(expr)?;
                Ok(Flow::Continue(value))
            }
        }
    }

    fn eval_expr(&mut self, expr: &Expr) -> Result<Val, String> {
        match expr {
            Expr::Int(i) => Ok(Val::Int(*i)),
            Expr::Bool(b) => Ok(Val::Bool(*b)),
            Expr::Float(f) => Ok(Val::Float(*f)),
            Expr::Str(s) => Ok(Val::Str(s.clone())),
            Expr::Array(arr) => {
                let res: Vec<Val> = arr
                    .iter()
                    .map(|e| self.eval_expr(e))
                    .collect::<Result<Vec<Val>, String>>()?;
                Ok(Val::Array(res))
            }

            Expr::Var(name) => self.lookup(name),

            Expr::Unary { op, expr } => {
                let val = self.eval_expr(expr)?;

                match (op, val) {
                    (UnaryOp::Neg, Val::Int(i)) => Ok(Val::Int(-i)),
                    (UnaryOp::Neg, Val::Float(f)) => Ok(Val::Float(-f)),
                    (UnaryOp::Not, Val::Bool(b)) => Ok(Val::Bool(!b)),
                    (op, val) => Err(format!("Cannot apply {:?} to {:?}", op, val)),
                }
            }

            Expr::Binary { op, lhs, rhs } => {
                let l = self.eval_expr(lhs)?;
                let r = self.eval_expr(rhs)?;

                self.eval_bin_op(*op, l, r)
            }

            Expr::Call { name, args } => {
                if let Some(builtin) = Self::builtins().get(name.as_str()) {
                    let arg_vals: Vec<Val> = args
                        .iter()
                        .map(|a| self.eval_expr(a))
                        .collect::<Result<_, _>>()?;
                    return builtin(arg_vals);
                }

                let func = self.lookup(name)?;

                if let Val::Function { params, body } = func {
                    let arg_vals: Vec<Val> = args
                        .iter()
                        .map(|a| self.eval_expr(a))
                        .collect::<Result<_, _>>()?;

                    if params.len() != arg_vals.len() {
                        return Err(format!(
                            "Function {} expects {} arguments, got {}",
                            name,
                            params.len(),
                            arg_vals.len()
                        ));
                    }

                    let mut frame = Frame::new();

                    for (param, arg) in params.iter().zip(arg_vals) {
                        frame.local.insert(param.clone(), arg);
                    }

                    self.stack.push(frame);

                    let mut res = Val::Unit;
                    for stmt in body {
                        match self.exec_stmt(&stmt)? {
                            Flow::Continue(v) => res = v,
                            Flow::Return(v) => {
                                self.stack.pop();
                                return Ok(v);
                            }
                        }
                    }

                    self.stack.pop();
                    Ok(res)
                } else {
                    Err(format!("'{}' is not a function", func))
                }
            }

            Expr::If { cond, then, else_ } => {
                let cond = self.eval_expr(cond)?;
                if let Val::Bool(b) = cond {
                    let branch = if b { then } else { else_ };

                    let mut res = Val::Unit;
                    for stmt in branch {
                        match self.exec_stmt(stmt)? {
                            Flow::Continue(v) => res = v,
                            Flow::Return(v) => return Ok(v),
                        }
                    }
                    Ok(res)
                } else {
                    Err(format!("Condition Must be a Boolean, got {:?}", cond))
                }
            }

            Expr::While { cond, body } => {
                loop {
                    let cond = self.eval_expr(cond)?;
                    if let Val::Bool(b) = cond {
                        if !b {
                            break;
                        }

                        for stmt in body {
                            match self.exec_stmt(stmt)? {
                                Flow::Continue(_) => {}
                                Flow::Return(v) => return Ok(v),
                            }
                        }
                    } else {
                        return Err(format!("While condition Must be a Boolean, got {:?}", cond));
                    }
                }
                Ok(Val::Unit)
            }

            Expr::For {
                var,
                start,
                end,
                body,
            } => {
                let st = self.eval_expr(start)?;
                let en = self.eval_expr(end)?;

                let (sti, eni) = match (st, en) {
                    (Val::Int(i), Val::Int(j)) => (i, j),
                    (a, b) => {
                        return Err(format!(
                            "The range must evaluate to ineteger bounds, got {}..{}",
                            a, b
                        ));
                    }
                };

                let parent_idx = Some(self.stack.len() - 1);
                let frame = Frame {
                    local: HashMap::new(),
                    parent: parent_idx,
                };
                self.stack.push(frame);
                for i in sti..eni {
                    if let Some(frame) = self.stack.last_mut() {
                        frame.local.insert(var.clone(), Val::Int(i));
                    }
                    for stmt in body {
                        match self.exec_stmt(stmt)? {
                            Flow::Continue(_) => {}
                            Flow::Return(v) => {
                                self.stack.pop();
                                return Ok(v);
                            }
                        }
                    }
                }

                self.stack.pop();
                return Ok(Val::Unit);
            }

            Expr::Block(stmts) => {
                let mut res = Val::Unit;
                for stmt in stmts {
                    match self.exec_stmt(stmt)? {
                        Flow::Continue(v) => res = v,
                        Flow::Return(v) => return Ok(v),
                    }
                }
                Ok(res)
            }

            Expr::ArrayAccess { name, indices } => {
                let val = self.lookup(name)?;
                let mut cur = &val;

                for expr in indices {
                    let idx_val = self.eval_expr(expr)?;
                    let idx = match idx_val {
                        Val::Int(i) => i as usize,
                        _ => {
                            return Err(format!(
                                "Array index must be an integer, got {:?}",
                                idx_val
                            ));
                        }
                    };

                    match cur {
                        Val::Array(arr) => {
                            if idx >= arr.len() {
                                return Err(format!("Array index out of bounds: {}", idx));
                            }
                            cur = &arr[idx];
                        }
                        Val::Str(s) => {
                            let chars: Vec<char> = s.chars().collect();
                            if idx >= chars.len() {
                                return Err(format!("String index out of bounds: {}", idx));
                            }
                            return Ok(Val::Str(chars[idx].to_string()));
                        }
                        _ => return Err(format!("Cannot index into {:?}", cur)),
                    }
                }

                Ok(cur.clone())
            }
        }
    }

    fn eval_bin_op(&self, op: BinaryOp, left: Val, right: Val) -> Result<Val, String> {
        match (op, &left, &right) {
            (BinaryOp::Add, Val::Int(a), Val::Int(b)) => Ok(Val::Int(a + b)),
            (BinaryOp::Sub, Val::Int(a), Val::Int(b)) => Ok(Val::Int(a - b)),
            (BinaryOp::Mul, Val::Int(a), Val::Int(b)) => Ok(Val::Int(a * b)),
            (BinaryOp::Div, Val::Int(a), Val::Int(b)) => {
                if *b == 0 {
                    Err("Division by zero".to_string())
                } else {
                    Ok(Val::Int(a / b))
                }
            }

            (BinaryOp::Mod, Val::Int(a), Val::Int(b)) => {
                if *b == 0 {
                    Err("Modulo by zero".to_string())
                } else {
                    Ok(Val::Int(a % b))
                }
            }

            (BinaryOp::Add, Val::Float(a), Val::Float(b)) => Ok(Val::Float(a + b)),
            (BinaryOp::Sub, Val::Float(a), Val::Float(b)) => Ok(Val::Float(a - b)),
            (BinaryOp::Mul, Val::Float(a), Val::Float(b)) => Ok(Val::Float(a * b)),
            (BinaryOp::Div, Val::Float(a), Val::Float(b)) => {
                if *b == 0f64 {
                    Err("Division by zero".to_string())
                } else {
                    Ok(Val::Float(a / b))
                }
            }

            (BinaryOp::Mod, Val::Float(a), Val::Float(b)) => {
                if *b == 0f64 {
                    Err("Modulo by zero".to_string())
                } else {
                    Ok(Val::Float(a % b))
                }
            }

            (BinaryOp::Add, Val::Str(a), Val::Str(b)) => Ok(Val::Str(a.clone() + b)),
            (BinaryOp::Mul, Val::Str(a), Val::Int(i)) => Ok(Val::Str(a.repeat(*i as usize))),

            (BinaryOp::Add, Val::Array(a), Val::Array(i)) => {
                let mut res = a.clone();
                res.extend(i.clone());
                Ok(Val::Array(res))
            }

            (BinaryOp::Eq, Val::Int(a), Val::Int(b)) => Ok(Val::Bool(a == b)),
            (BinaryOp::Ne, Val::Int(a), Val::Int(b)) => Ok(Val::Bool(a != b)),
            (BinaryOp::Gt, Val::Int(a), Val::Int(b)) => Ok(Val::Bool(a > b)),
            (BinaryOp::Ge, Val::Int(a), Val::Int(b)) => Ok(Val::Bool(a >= b)),
            (BinaryOp::Lt, Val::Int(a), Val::Int(b)) => Ok(Val::Bool(a < b)),
            (BinaryOp::Le, Val::Int(a), Val::Int(b)) => Ok(Val::Bool(a <= b)),

            (BinaryOp::Eq, Val::Float(a), Val::Float(b)) => Ok(Val::Bool(a == b)),
            (BinaryOp::Ne, Val::Float(a), Val::Float(b)) => Ok(Val::Bool(a != b)),
            (BinaryOp::Gt, Val::Float(a), Val::Float(b)) => Ok(Val::Bool(a > b)),
            (BinaryOp::Ge, Val::Float(a), Val::Float(b)) => Ok(Val::Bool(a >= b)),
            (BinaryOp::Lt, Val::Float(a), Val::Float(b)) => Ok(Val::Bool(a < b)),
            (BinaryOp::Le, Val::Float(a), Val::Float(b)) => Ok(Val::Bool(a <= b)),

            (BinaryOp::Eq, Val::Bool(a), Val::Bool(b)) => Ok(Val::Bool(a == b)),
            (BinaryOp::Ne, Val::Bool(a), Val::Bool(b)) => Ok(Val::Bool(a != b)),
            (BinaryOp::And, Val::Bool(a), Val::Bool(b)) => Ok(Val::Bool(*a && *b)),
            (BinaryOp::Or, Val::Bool(a), Val::Bool(b)) => Ok(Val::Bool(*a || *b)),

            _ => Err(format!(
                "Cannot apply {:?} to {:?} and {:?}",
                op, left, right
            )),
        }
    }

    fn lookup(&mut self, name: &str) -> Result<Val, String> {
        let mut current_idx = self.stack.len() - 1;

        loop {
            let frame = &self.stack[current_idx];

            if let Some(val) = frame.local.get(name) {
                return Ok(val.clone());
            }

            match frame.parent {
                Some(parent_idx) => current_idx = parent_idx,
                None => break,
            }
        }

        if let Some(val) = self.global.get(name) {
            return Ok(val.clone());
        }

        Err(format!("Undefined Variable: {}", name))
    }

    fn lookup_mut(&mut self, name: &str) -> Option<&mut Val> {
        let mut search_idx = self.stack.len() - 1;

        let mut found_idx = None;
        loop {
            let frame = &self.stack[search_idx];

            if frame.local.contains_key(name) {
                found_idx = Some(search_idx);
                break;
            }

            match frame.parent {
                Some(parent_idx) => search_idx = parent_idx,
                None => break,
            }
        }

        if let Some(idx) = found_idx {
            return self.stack[idx].local.get_mut(name);
        }

        self.global.get_mut(name)
    }

    fn builtins() -> HashMap<&'static str, fn(Vec<Val>) -> Result<Val, String>> {
        let mut map: HashMap<&'static str, fn(Vec<Val>) -> Result<Val, String>> = HashMap::new();

        map.insert("print", |args| {
            for v in &args {
                print!("{}", v);
            }
            io::stdout().flush().unwrap();
            Ok(Val::Unit)
        });

        map.insert("println", |args| {
            for (i, v) in args.iter().enumerate() {
                if i > 0 {
                    print!(" ");
                }
                print!("{}", v);
            }
            println!();
            Ok(Val::Unit)
        });
        map.insert("sin", |args: Vec<Val>| -> Result<Val, String> {
            if args.len() != 1 {
                return Err(format!("sin() takes 1 argument, got {}", args.len()));
            }
            match &args[0] {
                Val::Int(n) => Ok(Val::Float((*n as f64).sin())),
                Val::Float(f) => Ok(Val::Float(f.sin())),
                _ => Err(format!("sin() requires a number, got {:?}", args[0])),
            }
        });

        map.insert("cos", |args: Vec<Val>| -> Result<Val, String> {
            if args.len() != 1 {
                return Err(format!("cos() takes 1 argument, got {}", args.len()));
            }
            match &args[0] {
                Val::Int(n) => Ok(Val::Float((*n as f64).cos())),
                Val::Float(f) => Ok(Val::Float(f.cos())),
                _ => Err(format!("cos() requires a number, got {:?}", args[0])),
            }
        });

        map.insert("floor", |args: Vec<Val>| -> Result<Val, String> {
            if args.len() != 1 {
                return Err(format!("floor() takes 1 argument, got {}", args.len()));
            }
            match &args[0] {
                Val::Int(n) => Ok(Val::Int(*n)),
                Val::Float(f) => Ok(Val::Int(f.floor() as i64)),
                _ => Err(format!("floor() requires a number, got {:?}", args[0])),
            }
        });

        map.insert("abs", |args: Vec<Val>| -> Result<Val, String> {
            if args.len() != 1 {
                return Err(format!("abs() takes 1 argument, got {}", args.len()));
            }
            match &args[0] {
                Val::Int(n) => Ok(Val::Int(n.abs())),
                Val::Float(f) => Ok(Val::Float(f.abs())),
                _ => Err(format!("abs() requires a number, got {:?}", args[0])),
            }
        });

        map.insert("sqrt", |args: Vec<Val>| -> Result<Val, String> {
            if args.len() != 1 {
                return Err(format!("sqrt() takes 1 argument, got {}", args.len()));
            }
            match &args[0] {
                Val::Int(n) => Ok(Val::Float((*n as f64).sqrt())),
                Val::Float(f) => Ok(Val::Float(f.sqrt())),
                _ => Err(format!("sqrt() requires a number, got {:?}", args[0])),
            }
        });

        map.insert("len", |args: Vec<Val>| -> Result<Val, String> {
            if args.len() != 1 {
                return Err(format!("len() takes 1 argument, got {}", args.len()));
            }
            match &args[0] {
                Val::Array(arr) => Ok(Val::Int(arr.len() as i64)),
                Val::Str(s) => Ok(Val::Int(s.chars().count() as i64)),
                _ => Err(format!(
                    "len() requires an array or string, got {:?}",
                    args[0]
                )),
            }
        });

        map.insert("clear", |args: Vec<Val>| -> Result<Val, String> {
            if !args.is_empty() {
                return Err(format!("clear() takes no arguments, got {}", args.len()));
            }
            print!("\x1B[2J\x1B[1;1H");
            Ok(Val::Unit)
        });

        map.insert("sleep", |args: Vec<Val>| -> Result<Val, String> {
            if args.len() != 1 {
                return Err(format!("sleep() takes 1 argument, got {}", args.len()));
            }
            match &args[0] {
                Val::Int(ms) => {
                    std::thread::sleep(std::time::Duration::from_millis(*ms as u64));
                    Ok(Val::Unit)
                }
                _ => Err(format!(
                    "sleep() requires an integer (milliseconds), got {:?}",
                    args[0]
                )),
            }
        });

        map
    }
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}

impl Frame {
    pub fn new() -> Frame {
        Frame {
            local: HashMap::new(),
            parent: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    fn run(source: &str) -> Result<Val, String> {
        let program = parse(source)?;
        let mut interpreter = Interpreter::new();
        interpreter.run(&program)
    }

    #[test]
    fn test_literal() {
        assert_eq!(run("42").unwrap(), Val::Int(42));
        assert_eq!(run("true").unwrap(), Val::Bool(true));
        assert_eq!(run("false").unwrap(), Val::Bool(false));
    }

    #[test]
    fn test_arithmetic() {
        assert_eq!(run("1 + 2").unwrap(), Val::Int(3));
        assert_eq!(run("10 - 3").unwrap(), Val::Int(7));
        assert_eq!(run("4 * 5").unwrap(), Val::Int(20));
        assert_eq!(run("15 / 3").unwrap(), Val::Int(5));
        assert_eq!(run("17 % 5").unwrap(), Val::Int(2));
    }

    #[test]
    fn test_comparison() {
        assert_eq!(run("1 < 2").unwrap(), Val::Bool(true));
        assert_eq!(run("2 > 1").unwrap(), Val::Bool(true));
        assert_eq!(run("1 == 1").unwrap(), Val::Bool(true));
        assert_eq!(run("1 != 2").unwrap(), Val::Bool(true));
    }

    #[test]
    fn test_variables() {
        assert_eq!(run("let x = 42\nx").unwrap(), Val::Int(42));
    }

    #[test]
    fn test_function() {
        let source = r#"
            fn add(a, b) {
                return a + b
            }
            add(3, 4)
        "#;
        assert_eq!(run(source).unwrap(), Val::Int(7));
    }

    #[test]
    fn test_conditional() {
        let source = r#"
            if (true) {
                42
            } else {
                0
            }
        "#;
        assert_eq!(run(source).unwrap(), Val::Int(42));

        let source = r#"
            if (false) {
                42
            } else {
                0
            }
        "#;
        assert_eq!(run(source).unwrap(), Val::Int(0));
    }

    #[test]
    fn test_while_loop() {
        let source = r#"
            let x = 0
            while (x < 5) {
                x = x + 1
            }
            x
        "#;
        assert_eq!(run(source).unwrap(), Val::Int(5));
    }

    #[test]
    fn test_factorial_iterative() {
        let source = r#"
            fn factorial(n) {
                let result = 1
                while (n > 1) {
                    result = result * n
                    n = n - 1
                }
                return result
            }
            factorial(5)
        "#;
        assert_eq!(run(source).unwrap(), Val::Int(120));
    }

    #[test]
    fn test_factorial_recursive() {
        let source = r#"
            fn factorial(n) {
                if (n <= 1) {
                    return 1
                } else {
                    return n * factorial(n - 1)
                }
            }
            factorial(5)
        "#;
        assert_eq!(run(source).unwrap(), Val::Int(120));
    }

    #[test]
    fn test_fibonacci_recursive() {
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
        assert_eq!(run(source).unwrap(), Val::Int(55));
    }

    #[test]
    fn test_fibonacci_iterative() {
        let source = r#"
            fn fib(n) {
                if (n < 2) {
                    return n
                } else {
                    let a = 0
                    let b = 1
                    let i = 2
                    let temp = 0
                    while (i <= n) {
                        temp = a + b
                        a = b
                        b = temp
                        i = i + 1
                    }
                    return b
                }
            }
            fib(10)
        "#;
        assert_eq!(run(source).unwrap(), Val::Int(55));
    }

    #[test]
    fn test_nested_calls() {
        let source = r#"
            fn double(x) {
                return x * 2
            }
            fn quadruple(x) {
                return double(double(x))
            }
            quadruple(5)
        "#;
        assert_eq!(run(source).unwrap(), Val::Int(20));
    }
}
