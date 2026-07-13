use soda_ast::{JudgmentKind, SignatureKind, Term};
use soda_parser::{parse_module, parse_term, parse_term_in};

#[test]
fn declarations_preserve_sorts_kinds_and_function_sorts() {
    let module = parse_module("module Typed; sort Expr; datatype Nat; constructor zero(): Nat; function f(Nat, Fn(Nat, Nat)): Nat; judgment step(Expr,Expr) kind transition; judgment valid(Nat);").unwrap();

    assert!(!module.sorts[0].inductive);
    assert!(module.sorts[1].inductive);
    assert_eq!(module.signatures[0].kind, SignatureKind::Constructor);
    assert_eq!(module.signatures[1].kind, SignatureKind::Function);
    assert_eq!(module.signatures[1].arguments[1], "Fn(Nat,Nat)");
    assert_eq!(module.judgments[0].kind, JudgmentKind::Transition);
    assert_eq!(module.judgments[1].kind, JudgmentKind::Relation);
}

