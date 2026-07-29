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

