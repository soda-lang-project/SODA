pub mod support;
use soda_ast::Term;
use soda_domains::{Domains, Limits};
use soda_parser::{load_module_source, parse_module, parse_term};
use soda_semantics::{Config, Engine, Status};
use std::path::PathBuf;
use std::process::Command;
use support::evaluation::{evaluate, expect};

const SCOTT: &str = include_str!("../../../examples/denotational/scott-strachey.soda");
const PLOTKIN: &str = include_str!("../../../examples/denotational/plotkin-powerdomain.soda");
const MOGGI: &str = include_str!("../../../examples/denotational/moggi-exceptions.soda");
const WADLER: &str = include_str!("../../../examples/denotational/wadler-state.soda");
const PROBABILITY: &str =
    include_str!("../../../examples/denotational/jones-plotkin-probability.soda");
const POINTS: [&str; 4] = ["bottom", "zero", "one", "two"];

fn engine(source: &str) -> Engine {
    let module = load_module_source("<paper>", source, std::path::Path::new("."))
        .unwrap()
        .module;
    soda_sorts::validate_module(&module).unwrap();
    soda_verifier::validate_module(&module).unwrap();

    Engine::new(module.rules).unwrap()
}

fn equal(engine: &Engine, left: &str, right: &str) {
    assert_eq!(
        evaluate(engine, left),
        evaluate(engine, right),
        "{left} != {right}"
    );
}

fn transformer(values: [usize; 3]) -> String {
    format!(
        "trans({}, {}, {})",
        POINTS[values[0]], POINTS[values[1]], POINTS[values[2]]
    )
}

fn tables() -> Vec<[usize; 3]> {
    (0..64).map(|n| [n / 16, n / 4 % 4, n % 4]).collect()
}

#[test]
fn scott_loops_are_least_fixed_points_for_every_strict_finite_body() {
    let engine = engine(SCOTT);

    for body in tables() {
        // Follow the body graph to zero, bottom or a cycle, independently of the equations.

        let result = [1, 2, 3].map(|mut state| {
            let mut seen = [false; 4];

            while state > 1 && !seen[state] {
                seen[state] = true;
                state = body[state - 1];
            }

            if state == 1 { 1 } else { 0 }
        });
        let body_term = transformer(body);
        let result_term = transformer(result);
        expect(
            &engine,
            &format!("iterate({body_term}, trans(bottom, bottom, bottom))"),
            &result_term,
        );
        expect(
            &engine,
            &format!("functional({body_term}, {result_term})"),
            &result_term,
        );
        // Check leastness against all 64 fixed-point candidates.

        for candidate in tables() {
            let apply = |state| if state == 0 { 0 } else { candidate[state - 1] };

            if candidate == [1, apply(body[1]), apply(body[2])] {
                assert!(
                    result
                        .iter()
                        .zip(candidate)
                        .all(|(a, b)| *a == 0 || *a == b)
                );
            }
        }
    }

    expect(
        &engine,
        "meaning(while_positive(skip))",
        "trans(zero, bottom, bottom)",
    );
    expect(
        &engine,
        "meaning(while_positive(decrement))",
        "trans(zero, zero, zero)",
    );
    equal(
        &engine,
        "meaning(seq(decrement, decrement))",
        "compose(meaning(decrement), meaning(decrement))",
    );
    let module = parse_module(SCOTT).unwrap();
    let registry = Domains::new(&module.domains, Limits::default()).unwrap();
    // 64 strict functions, plus the three non-bottom constant functions.
    assert_eq!(
        registry.get("TransformerSpace").unwrap().elements().len(),
        67
    );
    let predecessor = registry.compile(&module.semantic_maps[0]).unwrap();

    for (input, output) in [
        ("bottom", "bottom"),
        ("zero", "zero"),
        ("one", "zero"),
        ("two", "one"),
    ] {
        assert_eq!(
            registry
                .apply(&predecessor, &parse_term(input).unwrap())
                .unwrap(),
            parse_term(output).unwrap()
        );
    }
}

fn interval(a: usize, b: usize) -> String {
    let points = ["low", "mid", "high"];

    format!("interval({}, {})", points[a], points[b])
}

#[test]
fn plotkin_convex_sets_have_egli_milner_order_and_choice_laws() {
    let engine = engine(PLOTKIN);
    let intervals: Vec<_> = (0..3).flat_map(|a| (a..3).map(move |b| (a, b))).collect();

    for &(a, b) in &intervals {
        let x = interval(a, b);
        expect(&engine, &format!("choose({x}, {x})"), &x);
        expect(
            &engine,
            &format!("extend({x}, bind(p: Point, interval(@p, @p)))"),
            &x,
        );

        for &(c, d) in &intervals {
            let y = interval(c, d);
            // The defining quantified set order, evaluated independently.
            let em = (a..=b).all(|x| (c..=d).any(|y| x <= y))
                && (c..=d).all(|y| (a..=b).any(|x| x <= y));

            assert_eq!(
                evaluate(&engine, &format!("power_le({x}, {y})")),
                Term::Bool(em)
            );
            expect(
                &engine,
                &format!("choose({x}, {y})"),
                &interval(a.min(c), b.max(d)),
            );
            equal(
                &engine,
                &format!("choose({x}, {y})"),
                &format!("choose({y}, {x})"),
            );

            for &(e, f) in &intervals {
                let z = interval(e, f);
                equal(
                    &engine,
                    &format!("choose(choose({x}, {y}), {z})"),
                    &format!("choose({x}, choose({y}, {z}))"),
                );
            }
        }
    }
    // Convex closure adds mid; the two-point set is not the canonical denotation.
    equal(
        &engine,
        "hull(more(low, single(high)))",
        "hull(more(low, more(mid, single(high))))",
    );
    expect(
        &engine,
        "power_le(interval(high, high), interval(low, high))",
        "false",
    );
    let kernel = "bind(p: Point, if(eq(@p, low), interval(low, mid), interval(high, high)))";
    expect(
        &engine,
        &format!("extend(interval(low, high), {kernel})"),
        "interval(low, high)",
    );
}

fn fraction(term: &Term) -> (i64, i64) {
    let Term::App(name, args) = term else {
        panic!("expected fraction: {term}")
    };

    assert_eq!(name, "fraction");
    let [Term::Int(n), Term::Int(d)] = args.as_slice() else {
        panic!("expected scalar rational: {term}")
    };

    assert!(*n >= 0 && *d > 0);

    (*n, *d)
}

