mod common;

use common::run_program;
use sigil_interpreter::value::Value;

// ── basic function declaration & call ──

#[test]
fn test_fn_call_two_params() {
    assert_eq!(
        run_program(r"fn add(x, y) { return x + y; } return add(1, 2);"),
        Value::Number(3.0)
    );
}

#[test]
fn test_fn_call_three_params() {
    assert_eq!(
        run_program(r"fn add(x, y, z) { return x + y + z; } return add(1, 2, 3);"),
        Value::Number(6.0)
    );
}

#[test]
fn test_fn_call_one_param() {
    assert_eq!(
        run_program(r"fn double(x) { return x * 2; } return double(21);"),
        Value::Number(42.0)
    );
}

#[test]
fn test_fn_call_no_params() {
    assert_eq!(
        run_program(r"fn answer() { return 42; } return answer();"),
        Value::Number(42.0)
    );
}

#[test]
fn test_fn_no_return() {
    assert_eq!(run_program(r"fn nop() { } return nop();"), Value::Nil);
}

// ── nested / chained calls ──

#[test]
fn test_fn_call_nested() {
    assert_eq!(
        run_program(r"fn double(x) { return x * 2; } return double(double(10));"),
        Value::Number(40.0)
    );
}

#[test]
fn test_fn_call_as_arg() {
    assert_eq!(
        run_program(
            r"fn add(x, y) { return x + y; } fn triple(x) { return x * 3; } return add(triple(1), triple(2));"
        ),
        Value::Number(9.0)
    );
}

// ── recursion ──

#[test]
fn test_fn_recursive_factorial() {
    assert_eq!(
        run_program(
            r"fn fact(n) {
                if n < 2 { return 1; }
                return n * fact(n - 1);
            }
            return fact(5);"
        ),
        Value::Number(120.0)
    );
}

#[test]
fn test_fn_recursive_fibonacci() {
    assert_eq!(
        run_program(
            r"fn fib(n) {
                if n < 2 { return n; }
                return fib(n - 1) + fib(n - 2);
            }
            return fib(10);"
        ),
        Value::Number(55.0)
    );
}

// ── variable shadowing ──

#[test]
fn test_fn_param_shadows_global() {
    assert_eq!(
        run_program(r"let x = 1; fn f(x) { return x + 1; } return f(41);"),
        Value::Number(42.0)
    );
}

#[test]
fn test_fn_local_does_not_leak() {
    assert_eq!(
        run_program(r"fn f() { let x = 42; return x; } return f();"),
        Value::Number(42.0)
    );
}

// ── multiple functions ──

#[test]
fn test_fn_multiple() {
    assert_eq!(
        run_program(
            r"fn a() { return 1; } fn b() { return 2; } fn c() { return a() + b(); } return c();"
        ),
        Value::Number(3.0)
    );
}

// #[test]
// fn test_fn_mutual_call() {
//     assert_eq!(
//         run_program(
//             r"fn is_even(n) {
//                 if n == 0 { return 1; }
//                 return is_odd(n - 1);
//             }
//             fn is_odd(n) {
//                 if n == 0 { return 0; }
//                 return is_even(n - 1);
//             }
//             return is_even(8);"
//         ),
//         Value::Number(1.0)
//     );
// }

#[test]
fn test_fn_early_return() {
    assert_eq!(
        run_program(r"fn test(x) { if x { return 1; } return 2; } return test(1);"),
        Value::Number(1.0)
    );
}

#[test]
fn test_fn_early_return_false() {
    assert_eq!(
        run_program(r"fn test(x) { if x { return 1; } return 2; } return test(0);"),
        Value::Number(2.0)
    );
}

// ── while inside function ──

#[test]
fn test_fn_while_sum() {
    assert_eq!(
        run_program(
            r"fn sum_to(n) {
                let i = 0;
                let s = 0;
                while i < n {
                    i = i + 1;
                    s = s + i;
                }
                return s;
            }
            return sum_to(10);"
        ),
        Value::Number(55.0)
    );
}

// ── void return (empty return) ──

#[test]
fn test_fn_empty_return() {
    assert_eq!(run_program(r"fn f() { return; } return f();"), Value::Nil);
}
