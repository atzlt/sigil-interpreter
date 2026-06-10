use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use sigil_interpreter::{compiler::compile::compile_program, functions::FunctionRegistry, vm::VM};

fn compile_and_run(source: &str) {
    let mut chunk = compile_program(source).unwrap();
    let registry = FunctionRegistry::with_std();
    let mut vm = VM::new();
    vm.run(&mut chunk, &registry).unwrap();
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

criterion_group!(
    benches,
    bench_while_counting,
);
criterion_main!(benches);
