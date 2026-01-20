use std::{env::args, fs};

use ew::{
    interpreter::{Interpreter, Val},
    parser::parse,
};
use rustyline::{Editor, error::ReadlineError, history::DefaultHistory};

fn main() {
    let args: Vec<String> = args().collect();

    if args.len() > 1 {
        let file = &args[1];
        run(file)
    } else {
        repl();
    }
}

fn run(file: &str) {
    let source = match fs::read_to_string(file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error reading file '{}': {}", file, e);
            std::process::exit(1);
        }
    };

    match ew::run(&source) {
        Ok(_) => println!(),
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

fn repl() {
    println!("Lmao v0.0.1");
    println!("Type 'quit' to exit\n");

    let mut interpret = Interpreter::new();
    let mut rl = Editor::<(), DefaultHistory>::new().unwrap();
    loop {
        let mut inp = String::new();

        let line = match rl.readline("> ") {
            Ok(line) => line,
            Err(ReadlineError::Interrupted) => {
                println!("^C");
                continue;
            }
            Err(ReadlineError::Eof) => {
                println!("Goodbye!");
                break;
            }
            Err(err) => {
                eprintln!("Error: {:?}", err);
                break;
            }
        };

        let trim = line.trim();
        if trim.is_empty() {
            continue;
        }

        if trim == "quit" || trim == "exit" {
            println!("Goodbye!");
            break;
        }

        rl.add_history_entry(line.as_str()).unwrap();
        inp.push_str(&line);
        inp.push('\n');

        while bracket_depth(&inp) > 0 {
            let line = match rl.readline("... ") {
                Ok(line) => line,
                Err(_) => break,
            };

            rl.add_history_entry(line.as_str()).unwrap();
            inp.push_str(&line);
            inp.push('\n');
        }

        let inp = inp.trim();
        match parse(inp) {
            Ok(program) => match interpret.run(&program) {
                Ok(value) => {
                    if value != Val::Unit {
                        println!("{}", value);
                    }
                }
                Err(e) => eprintln!("Runtime error: {}", e),
            },
            Err(e) => eprintln!("Parse error: {}", e),
        }
    }
}

fn bracket_depth(s: &str) -> i32 {
    let mut depth = 0;
    let mut in_string = false;
    let mut prev_char = ' ';

    for c in s.chars() {
        if c == '"' && prev_char != '\\' {
            in_string = !in_string;
        }

        if !in_string {
            match c {
                '{' | '(' | '[' => depth += 1,
                '}' | ')' | ']' => depth -= 1,
                _ => {}
            }
        }
        prev_char = c;
    }

    depth
}
