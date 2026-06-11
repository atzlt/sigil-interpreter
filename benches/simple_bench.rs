use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use sigil_interpreter::{compiler::compile::compile_program, vm::VM};

fn compile_and_run(source: &str) {
    let compiled = compile_program(source).unwrap();
    let mut vm = VM::default();
    vm.run(&compiled.0, &compiled.1).unwrap();
}

fn bench_expr_long_chain(c: &mut Criterion) {
    let source: String = (0..1000)
        .map(|i| i.to_string())
        .collect::<Vec<_>>()
        .join(" + ");
    let source = format!("return {source};");
    c.bench_function("expr/1000_add_chain", |b| {
        b.iter(|| compile_and_run(black_box(&source)))
    });
}

fn bench_while_counting(c: &mut Criterion) {
    let source = "
        { let i = 0;
        while i < 10000 {
            i = i + 1;
        }
        return i; }
    ";
    c.bench_function("while/10000_counting", |b| {
        b.iter(|| compile_and_run(black_box(source)))
    });
}

fn bench_fibonacci_iter(c: &mut Criterion) {
    let source = "
        { let n = 500;
        let a = 0;
        let b = 1;
        let i = 0;
        while i < n {
            let t = a + b;
            a = b;
            b = t;
            i = i + 1;
        }
        return a; }
    ";
    c.bench_function("fibonacci/iter_500", |b| {
        b.iter(|| compile_and_run(black_box(source)))
    });
}

fn bench_if_else_chain(c: &mut Criterion) {
    let mut parts = Vec::new();
    for i in 0..100 {
        if i < 99 {
            parts.push(format!("if 0 {{ return 0; }} else "));
        } else {
            parts.push(format!("{{ return {i}; }}"));
        }
    }
    let source = parts.join("");
    c.bench_function("if_else/100_chain", |b| {
        b.iter(|| compile_and_run(black_box(&source)))
    });
}

criterion_group!(
    benches,
    bench_expr_long_chain,
    bench_while_counting,
    bench_fibonacci_iter,
    bench_if_else_chain,
);
criterion_main!(benches);
