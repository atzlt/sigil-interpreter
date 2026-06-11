mod common;

use common::run_program;
use sigil_interpreter::value::Value;

// ── let + return ──

#[test]
fn test_program_let_return() {
    assert_eq!(run_program("let x = 42; return x;"), Value::Number(42.0));
}

#[test]
fn test_program_let_expr_chain() {
    assert_eq!(
        run_program("let a = 1; let b = 2; let c = a + b; c;"),
        Value::Nil
    );
}

#[test]
fn test_program_let_expr_return() {
    assert_eq!(run_program("let x = 1 + 2 * 3; x + 4; return;"), Value::Nil);
}

#[test]
fn test_program_return_nil_after_lets() {
    assert_eq!(run_program("let a = 1; let b = 2; return;"), Value::Nil);
}

#[test]
fn test_program_return_expr_after_lets() {
    assert_eq!(
        run_program("let a = 10; let b = 20; return a + b; let b = 30; return a;"),
        Value::Number(30.0)
    );
}

// ── blocks ──

#[test]
fn test_block_basic() {
    assert_eq!(
        run_program(r"{ let x = 42; return x; }"),
        Value::Number(42.0)
    );
}

#[test]
fn test_block_var_shadowing() {
    assert_eq!(
        run_program(r"let x = 1; { let x = 2; } return x;"),
        Value::Number(1.0)
    );
}

#[test]
fn test_block_nested() {
    assert_eq!(
        run_program(r"{ let a = 1; { let b = 2; return a + b; } }"),
        Value::Number(3.0)
    );
}

#[test]
fn test_block_register_reuse() {
    assert_eq!(
        run_program(r"let x = 1; { let y = x + 3; } let z = 3; return x + z;"),
        Value::Number(4.0)
    );
}

// ── if statements ──

#[test]
fn test_if_true() {
    assert_eq!(run_program(r"if 1 { return 42; }"), Value::Number(42.0));
}

#[test]
fn test_if_false_falls_through() {
    assert_eq!(
        run_program(r"if 0 { return 1; } return 2;"),
        Value::Number(2.0)
    );
}

#[test]
fn test_if_else_true() {
    assert_eq!(
        run_program(r"if 1 { return 1; } else { return 2; }"),
        Value::Number(1.0)
    );
}

#[test]
fn test_if_else_false() {
    assert_eq!(
        run_program(r"if 0 { return 1; } else { return 2; }"),
        Value::Number(2.0)
    );
}

#[test]
fn test_if_else_if_chain_first_true() {
    assert_eq!(
        run_program(r"if 1 { return 1; } else if 1 { return 2; } else { return 3; }"),
        Value::Number(1.0)
    );
}

#[test]
fn test_if_else_if_chain_second_true() {
    assert_eq!(
        run_program(r"if 0 { return 1; } else if 1 { return 2; } else { return 3; }"),
        Value::Number(2.0)
    );
}

#[test]
fn test_if_else_if_chain_all_false() {
    assert_eq!(
        run_program(r"if 0 { return 1; } else if 0 { return 2; } else { return 3; }"),
        Value::Number(3.0)
    );
}

#[test]
fn test_if_nested_true() {
    assert_eq!(
        run_program(r"if 1 { if 1 { return 42; } }"),
        Value::Number(42.0)
    );
}

#[test]
fn test_if_nested_outer_false() {
    assert_eq!(
        run_program(r"if 0 { if 1 { return 1; } } return 2;"),
        Value::Number(2.0)
    );
}

#[test]
fn test_if_with_variable_true() {
    assert_eq!(
        run_program(r"let x = 1; if x { return 10; } else { return 20; }"),
        Value::Number(10.0)
    );
}

#[test]
fn test_if_with_variable_false() {
    assert_eq!(
        run_program(r"let x = 0; if x { return 10; } else { return 20; }"),
        Value::Number(20.0)
    );
}

#[test]
fn test_if_with_expression_condition() {
    assert_eq!(
        run_program(r"if 2 - 2 { return 1; } else { return 2; }"),
        Value::Number(2.0)
    );
}

#[test]
fn test_if_with_let_in_body() {
    assert_eq!(
        run_program(r"if 1 { let x = 42; return x; }"),
        Value::Number(42.0)
    );
}

#[test]
fn test_if_long_else_if_chain() {
    assert_eq!(
        run_program(
            r"if 0 { return 0; } else if 0 { return 1; } else if 0 { return 2; }
              else if 0 { return 3; } else if 0 { return 4; } else if 0 { return 5; }
              
              else if 0 { return 6; } else if 0 { return 7; } else if 0 { return 8; }
              else if 1 { return 9; } else { return 10; }"
        ),
        Value::Number(9.0)
    );
}

// ── while loops ──

#[test]
fn test_while_true_returns_from_body() {
    assert_eq!(run_program(r"while 1 { return 42; }"), Value::Number(42.0));
}

#[test]
fn test_while_false_skips_body() {
    assert_eq!(
        run_program(r"while 0 { return 1; } return 2;"),
        Value::Number(2.0)
    );
}

#[test]
fn test_while_condition_truthy_then_returns() {
    assert_eq!(
        run_program(r"while 1 < 2 { return 10; }"),
        Value::Number(10.0)
    );
}

#[test]
fn test_while_condition_falsey_skips() {
    assert_eq!(
        run_program(r"while 2 < 1 { return 10; } return 20;"),
        Value::Number(20.0)
    );
}

#[test]
fn test_while_nested() {
    assert_eq!(
        run_program(r"while 1 { while 1 { return 42; } }"),
        Value::Number(42.0)
    );
}

#[test]
fn test_while_outer_false_inner_true() {
    assert_eq!(
        run_program(r"while 0 { while 1 { return 1; } } return 2;"),
        Value::Number(2.0)
    );
}

#[test]
fn test_while_with_let_in_body() {
    assert_eq!(
        run_program(r"while 1 { let x = 42; return x; }"),
        Value::Number(42.0)
    );
}

#[test]
fn test_while_with_if_in_body() {
    assert_eq!(
        run_program(r"while 1 { if 1 { return 42; } else { return 0; } }"),
        Value::Number(42.0)
    );
}

#[test]
fn test_while_with_expr_condition_false() {
    assert_eq!(
        run_program(r"while 1 - 1 { return 1; } return 2;"),
        Value::Number(2.0)
    );
}

// ── assignment ──

#[test]
fn test_assignment_basic() {
    assert_eq!(
        run_program(r"let x = 1; x = 2; return x;"),
        Value::Number(2.0)
    );
}

#[test]
fn test_while_counting_with_assignment() {
    assert_eq!(
        run_program(r"{ let i = 0; while i < 1000 { i = i + 1; } return i; }"),
        Value::Number(1000.0)
    );
}

#[test]
fn test_while_sum_with_assignment() {
    assert_eq!(
        run_program(
            r"let i = 0; let sum = 0;
              while i < 10 {
                sum = sum + i;
                i = i + 1;
              }
              return sum;"
        ),
        Value::Number(45.0)
    );
}

#[test]
fn test_assignment_to_outer_var_from_block() {
    assert_eq!(
        run_program(r"let x = 1; { x = 2; } return x;"),
        Value::Number(2.0)
    );
}

// ── break ──

#[test]
fn test_break_immediate() {
    assert_eq!(run_program(r"while 1 { break; }"), Value::Nil);
}

#[test]
fn test_break_before_return() {
    assert_eq!(run_program(r"while 1 { break; return 42; }"), Value::Nil);
}

#[test]
fn test_break_after_statement() {
    assert_eq!(
        run_program(r"let x = 1; while 1 { x = 2; break; } return x;"),
        Value::Number(2.0)
    );
}

#[test]
fn test_break_conditional() {
    assert_eq!(
        run_program(r"let i = 0; while i < 10 { i = i + 1; if i == 3 { break; } } return i;"),
        Value::Number(3.0)
    );
}

#[test]
fn test_break_nested_inner() {
    assert_eq!(
        run_program(r"while 1 { while 1 { break; } return 1; } return 2;"),
        Value::Number(1.0)
    );
}

#[test]
fn test_break_nested_outer_via_flag() {
    assert_eq!(
        run_program(
            r"let flag = 0;
              while 1 {
                while 1 { flag = 1; break; }
                if flag { break; }
              }
              return flag;"
        ),
        Value::Number(1.0)
    );
}

// ── continue ──

#[test]
fn test_continue_skips_rest_of_body() {
    assert_eq!(
        run_program(r"let x = 1; while x < 3 { x = x + 1; continue; x = 3; } return x;"),
        Value::Number(3.0)
    );
}

#[test]
fn test_continue_conditional() {
    assert_eq!(
        run_program(
            r"let x = 0; let i = 0; while i < 10 { i = i + 1; if i == 3 { continue; } x = x + 1; } return x;"
        ),
        Value::Number(9.0)
    );
}

#[test]
fn test_continue_nested_inner() {
    assert_eq!(
        run_program(
            r"let x = 0; while x < 2 { x = x + 1; while x < 80 { x = 100; continue; x = 70; } } return x;"
        ),
        Value::Number(100.0)
    );
}

#[test]
fn test_fn_decl() {
    assert_eq!(
        run_program(r"fn add(x, y, z) { return x + y + z; } return 0;"),
        Value::Number(0.0)
    );
}

#[test]
fn test_fn_call() {
    assert_eq!(
        run_program(r"fn add(x, y) { return x + y; } return add(1, 2);"),
        Value::Number(3.0)
    );
}

#[test]
fn test_empty_fn_call() {
    assert_eq!(
        run_program(r"fn add(x, y) { } return add(1, 2);"),
        Value::Nil
    );
}
