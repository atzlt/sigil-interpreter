mod common;

use common::run_program;
use sigil_interpreter::{value::Value, vm::exec::RuntimeError};

use crate::common::run_program_err;

// ── basic function declaration & call ──

#[test]
fn test_fn_call_two_params() {
    assert_eq!(
        run_program(r"fn sub(x, y) { return x - y; } return sub(1, 2);"),
        Value::Number(-1.0)
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

#[test]
fn test_fn_mutual_call() {
    assert_eq!(
        run_program(
            r"
            fn is_odd() {}
            fn is_even(n) {
                if n == 0 { return 1; }
                return is_odd(n - 1);
            }
            fn is_odd(n) {
                if n == 0 { return 0; }
                return is_even(n - 1);
            }
            return is_even(8);"
        ),
        Value::Number(1.0)
    );
}

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

#[test]
fn test_fn_overflow() {
    assert!(matches!(
        run_program_err(r"fn f() { return f(); } return f();"),
        RuntimeError::StackOverflow { .. }
    ));
}

// ── nested functions & closures ──

#[test]
fn test_nested_fn_captures_outer_local() {
    assert_eq!(
        run_program(
            r"fn outer() {
                let x = 42;
                fn inner() {
                    return x;
                }
                return inner();
            }
            return outer();"
        ),
        Value::Number(42.0)
    );
}

#[test]
fn test_nested_fn_captures_param() {
    assert_eq!(
        run_program(
            r"fn make_adder(n) {
                fn adder(x) {
                    return n + x;
                }
                return adder(3);
            }
            return make_adder(5);"
        ),
        Value::Number(8.0)
    );
}

#[test]
fn test_nested_fn_mutates_outer() {
    assert_eq!(
        run_program(
            r"fn counter() {
                let n = 0;
                fn bump() {
                    n = n + 1;
                    return n;
                }
                bump();
                bump();
                return bump();
            }
            return counter();"
        ),
        Value::Number(3.0)
    );
}

#[test]
fn test_nested_fn_no_capture() {
    assert_eq!(
        run_program(
            r"fn outer() {
                fn inner(x) {
                    return x * 2;
                }
                return inner(21);
            }
            return outer();"
        ),
        Value::Number(42.0)
    );
}

#[test]
fn test_nested_fn_deep_capture() {
    // Three levels: innermost captures from outermost
    assert_eq!(
        run_program(
            r"fn a() {
                let x = 10;
                fn b() {
                    let y = 20;
                    fn c() {
                        fn d() { return x + y; }
                        return d();
                    }
                    return c();
                }
                return b();
            }
            return a();"
        ),
        Value::Number(30.0)
    );
}

// ── closure expression (anonymous fn) tests ──

#[test]
fn test_closure_expr_block_body() {
    assert_eq!(
        run_program(r"let f = fn(x) { return x + 1; }; return f(5);"),
        Value::Number(6.0)
    );
}

#[test]
fn test_closure_expr_expr_body() {
    assert_eq!(
        run_program(r"let f = fn(x) x + 1; return f(5);"),
        Value::Number(6.0)
    );
}

#[test]
fn test_closure_expr_captures_outer() {
    assert_eq!(
        run_program(r"let n = 10; let f = fn(x) x + n; return f(5);"),
        Value::Number(15.0)
    );
}

#[test]
fn test_closure_expr_passed_as_arg() {
    assert_eq!(
        run_program(
            r"fn apply(f, x) { return f(x); } return apply(fn(n) n + 1, 5);"
        ),
        Value::Number(6.0)
    );
}

#[test]
fn test_closure_expr_returned() {
    assert_eq!(
        run_program(
            r"fn make_adder(n) { return fn(x) x + n; } let add5 = make_adder(5); return add5(3);"
        ),
        Value::Number(8.0)
    );
}

#[test]
fn test_closure_expr_no_params() {
    assert_eq!(run_program(r"let f = fn() 42; return f();"), Value::Number(42.0));
}

#[test]
fn test_closure_expr_mutates_outer() {
    assert_eq!(
        run_program(
            r"let n = 0; let bump = fn() { n = n + 1; return n; }; bump(); bump(); return bump();"
        ),
        Value::Number(3.0)
    );
}

// ── first-class function tests ──

#[test]
fn test_fn_stored_in_variable_and_called() {
    assert_eq!(
        run_program(r"fn double(x) { return x * 2; } let f = double; return f(21);"),
        Value::Number(42.0)
    );
}

#[test]
fn test_fn_passed_as_argument() {
    assert_eq!(
        run_program(
            r"fn apply_twice(f, x) { return f(f(x)); } fn double(x) { return x * 2; } return apply_twice(double, 5);"
        ),
        Value::Number(20.0)
    );
}

#[test]
fn test_fn_returned_from_function() {
    assert_eq!(
        run_program(
            r"fn make_doubler() { fn double(x) { return x * 2; } return double; } let d = make_doubler(); return d(21);"
        ),
        Value::Number(42.0)
    );
}

#[test]
fn test_fn_returned_from_function_captures_param() {
    assert_eq!(
        run_program(
            r"fn make_adder(n) { fn add(x) { return x + n; } return add; } let a5 = make_adder(5); let a10 = make_adder(10); return a5(3) + a10(3);"
        ),
        Value::Number(21.0)
    );
}

#[test]
fn test_closure_expr_called_directly() {
    assert_eq!(
        run_program(r"return (fn(x) x + 1)(5);"),
        Value::Number(6.0)
    );
}

#[test]
fn test_closure_expr_as_arg_to_closure() {
    assert_eq!(
        run_program(
            r"fn twice(f, x) { return f(f(x)); } return twice(fn(n) n * 2, 5);"
        ),
        Value::Number(20.0)
    );
}
