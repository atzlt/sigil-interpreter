use std::{env, fs, process};

use ariadne::{Label, Report, ReportKind, Source};
use sigil_interpreter::{
    compiler::compile::{CompileError, compile_program},
    value::Value,
    vm::{VM, exec::RuntimeError},
};

fn report_compile_error(source: &str, err: &CompileError) {
    let src = Source::from(source);

    match err {
        CompileError::Unexpected {
            diag: (span, msg), ..
        } => {
            Report::build(ReportKind::Error, span.clone())
                .with_message("unexpected token")
                .with_label(Label::new(span.clone()).with_message(msg))
                .finish()
                .eprint(&src)
                .unwrap();
        }
        CompileError::Unclosed {
            open: (open_span, open_msg),
            close: (close_span, close_msg),
        } => {
            Report::build(ReportKind::Error, close_span.clone())
                .with_message("unclosed delimiter")
                .with_label(Label::new(open_span.clone()).with_message(open_msg))
                .with_label(Label::new(close_span.clone()).with_message(close_msg))
                .finish()
                .eprint(&src)
                .unwrap();
        }
        CompileError::Unrecognized((span, msg)) => {
            Report::build(ReportKind::Error, span.clone())
                .with_message("unrecognized token")
                .with_label(Label::new(span.clone()).with_message(msg))
                .finish()
                .eprint(&src)
                .unwrap();
        }
        CompileError::RegisterOverflow((span, msg)) => {
            Report::build(ReportKind::Error, span.clone())
                .with_message("register overflow")
                .with_label(Label::new(span.clone()).with_message(msg))
                .finish()
                .eprint(&src)
                .unwrap();
        }
        CompileError::UndefinedVariable {
            diag: (span, msg), ..
        } => {
            Report::build(ReportKind::Error, span.clone())
                .with_message("undefined variable")
                .with_label(Label::new(span.clone()).with_message(msg))
                .finish()
                .eprint(&src)
                .unwrap();
        }
        _ => todo!()
    }
}

fn report_runtime_error(source: &str, err: &RuntimeError) {
    let src = Source::from(source);

    match err {
        RuntimeError::StackOverflow => {
            eprintln!("stack overflow");
        }
        RuntimeError::InvalidOpCode { op_byte, span } => {
            Report::build(ReportKind::Error, span.clone())
                .with_message(format!("invalid opcode: 0x{op_byte:02X}"))
                .with_label(Label::new(span.clone()).with_message("invalid instruction"))
                .finish()
                .eprint(&src)
                .unwrap();
        }
        RuntimeError::UndefinedFunction { name, span } => {
            Report::build(ReportKind::Error, span.clone())
                .with_message(format!("undefined function: {name}"))
                .with_label(Label::new(span.clone()).with_message("function not found"))
                .finish()
                .eprint(&src)
                .unwrap();
        }
        RuntimeError::IpOutOfBounds { ip, span } => {
            Report::build(ReportKind::Error, span.clone())
                .with_message(format!("instruction pointer out of bounds: {ip}"))
                .with_label(Label::new(span.clone()).with_message("invalid jump target"))
                .finish()
                .eprint(&src)
                .unwrap();
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("usage: {} <file>", args[0]);
        process::exit(1);
    }

    let filename = &args[1];
    let source = match fs::read_to_string(filename) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error reading {filename}: {e}");
            process::exit(1);
        }
    };

    let mut chunk = match compile_program(&source) {
        Ok(c) => c,
        Err(e) => {
            report_compile_error(&source, &e);
            process::exit(1);
        }
    };

    let mut vm = VM::default();
    match vm.run(&mut chunk) {
        Ok(val) => {
            if val != Value::Nil {
                println!("{val}");
            }
        }
        Err(e) => {
            report_runtime_error(&source, &e);
        }
    }
}
