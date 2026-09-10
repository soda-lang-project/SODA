use soda_ast::Term;
use soda_parser::parse_term;
use soda_semantics::{Config, Engine, Status};

pub fn evaluate(engine: &Engine, source: &str) -> Term {
    let result = engine.evaluate(&parse_term(source).unwrap(), &Config::default());
    assert_eq!(result.status, Status::NormalForm, "{source}: {result:?}");

    result.term
}

pub fn expect(engine: &Engine, source: &str, expected: &str) {
    assert_eq!(
        evaluate(engine, source),
        parse_term(expected).unwrap(),
        "{source}"
    );
}
