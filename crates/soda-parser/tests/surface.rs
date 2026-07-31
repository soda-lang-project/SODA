use soda_ast::Term;
use soda_parser::{parse_module, parse_module_located, parse_term_in};

const PRELUDE: &str = "module Surface;
sort Computation;
function unit(Int): Computation;
function chain(Computation, Int -> Computation): Computation;
monad State { bind = chain; return = unit; };
";

#[test]
fn do_blocks_infer_binding_sorts_and_scope_bare_names() {
    let module = parse_module(&format!(
        "{PRELUDE} eval do State {{ x <- unit(1); y: Int <- unit(x); return add(x,y); }};"
    ))
    .unwrap();
    let expected = parse_term_in(
        &module,
        "chain(unit(1), bind(x:Int, chain(unit(@x), bind(y:Int, unit(add(@x,@y))))))",
    )
    .unwrap();
    assert_eq!(module.evaluations[0], expected);
    let interactive = parse_term_in(&module, "do State { x <- unit(2); return x; }").unwrap();
    assert_eq!(
        interactive,
        parse_term_in(&module, "chain(unit(2), bind(x:Int, unit(@x)))").unwrap()
    );
    assert_eq!(parse_term_in(&module, "x").unwrap(), Term::app("x", vec![]));
}

#[test]
fn nested_blocks_preserve_outer_scope_and_allow_shadowing() {
    let module = parse_module(PRELUDE).unwrap();
    let term = parse_term_in(
        &module,
        "do State { x <- unit(1); y <- do State { x <- unit(x); return x; }; return add(x,y); }",
    )
    .unwrap();
    let expected = parse_term_in(&module, "chain(unit(1), bind(x:Int, chain(chain(unit(@x), bind(x:Int, unit(@x))), bind(y:Int, unit(add(@x,@y))))))").unwrap();
    assert_eq!(term, expected);
}

#[test]
fn ignored_effect_binding_avoids_names_in_source() {
    let module = parse_module(PRELUDE).unwrap();
    let term = parse_term_in(&module, "do State { unit(1); return @__soda_do_3; }").unwrap();
    let Term::App(_, args) = term else { panic!() };
    let Term::Binder {
        parameter, body, ..
    } = &args[1]
    else {
        panic!()
    };
    assert_ne!(parameter, "__soda_do_3");
    assert!(body.free_names().contains("__soda_do_3"));
}

#[test]
fn do_reports_unknown_instances_bad_types_and_incomplete_blocks() {
    let module = parse_module(PRELUDE).unwrap();
    for (source, message) in [
        ("do Missing { return 1; }", "unknown monad instance"),
        ("do State { x: Bool <- unit(1); return 2; }", "binds Int"),
        ("do State { x <- unit(1); }", "final computation"),
        ("do State { }", "final computation"),
    ] {
        assert!(
            parse_term_in(&module, source)
                .unwrap_err()
                .to_string()
                .contains(message)
        );
    }
    let invalid = "module Bad; sort M; function u(Int):M; function b(M, Bool -> M):M; monad X {bind=b;return=u;};";
    assert!(
        parse_module(invalid)
            .unwrap_err()
            .to_string()
            .contains("incompatible")
    );
}

#[test]
fn arrow_types_are_right_associative_and_parentheses_change_grouping() {
    let module = parse_module("module Arrows; function f(Int -> Bool -> Int, (Int -> Bool) -> Int): Int -> (Bool -> Int);").unwrap();
    assert_eq!(
        module.signatures[0].arguments,
        ["Fn(Int,Fn(Bool,Int))", "Fn(Fn(Int,Bool),Int)"]
    );
    assert_eq!(module.signatures[0].result, "Fn(Int,Fn(Bool,Int))");
}

#[test]
fn lists_support_empty_nested_and_pattern_tails() {
    let module = parse_module(
        "module Lists; eval []; eval [1,[2],3]; rule head: first([?x | ?tail]) => ?x;",
    )
    .unwrap();
    assert_eq!(module.evaluations[0], Term::app("end", vec![]));
    assert_eq!(
        module.evaluations[1],
        parse_term_in(&module, "cons(1,cons(cons(2,end),cons(3,end)))").unwrap()
    );
    assert_eq!(
        module.rules[0].lhs,
        parse_term_in(&module, "first(cons(?x,?tail))").unwrap()
    );
}

#[test]
fn records_reorder_fields_in_values_patterns_and_interactive_terms() {
    let source = "module Records; record Store = store { value: Int, valid: Bool }; eval store {valid: true, value: 7}; rule read: read(store {valid: true, value: ?x}) => ?x;";
    let parsed = parse_module_located(source).unwrap();
    let module = &parsed.module;
    assert_eq!(module.signatures[0].arguments, ["Int", "Bool"]);
    assert!(module.sorts[0].inductive);
    assert_eq!(
        module.evaluations[0],
        Term::app("store", vec![Term::Int(7), Term::Bool(true)])
    );
    assert_eq!(
        parse_term_in(module, "store {value: 7, valid: true}").unwrap(),
        module.evaluations[0]
    );
    assert!(parsed.locations.declaration("Store").is_some());
    assert_eq!(
        module.rules[0].lhs,
        parse_term_in(module, "read(store(?x,true))").unwrap()
    );
}

#[test]
fn record_diagnostics_identify_missing_duplicate_and_unknown_fields() {
    let module =
        parse_module("module Records; record Store = store { value: Int, valid: Bool };").unwrap();
    for (source, message) in [
        ("store {value:1}", "missing field 'valid'"),
        (
            "store {value:1,value:2,valid:true}",
            "duplicate field 'value'",
        ),
        ("store {other:1}", "unknown field 'other'"),
    ] {
        assert!(
            parse_term_in(&module, source)
                .unwrap_err()
                .to_string()
                .contains(message)
        );
    }
    assert!(
        parse_module("module X; record S = s {x:Int,x:Int};")
            .unwrap_err()
            .to_string()
            .contains("duplicate record field")
    );
}
