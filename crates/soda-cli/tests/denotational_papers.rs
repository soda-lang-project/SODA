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

#[test]
fn moggi_exception_model_obeys_the_three_monad_laws_and_short_circuits() {
    let engine = engine(MOGGI);
    let kernels = [
        "bind(x: Int, exception_unit(add(@x, 1)))",
        "bind(x: Int, exception_raised(exception_error))",
        "bind(x: Int, exception_div(12, @x))",
    ];

    for n in -2..=2 {
        for f in kernels {
            equal(
                &engine,
                &format!("exception_bind(exception_unit({n}), {f})"),
                &format!("apply({f}, {n})"),
            );
        }
    }

    for m in [
        "exception_returned(0)",
        "exception_returned(2)",
        "exception_raised(exception_error)",
        "exception_raised(exception_zero_divisor)",
    ] {
        equal(
            &engine,
            &format!("exception_bind({m}, bind(x: Int, exception_unit(@x)))"),
            m,
        );

        for f in kernels {
            for g in kernels {
                equal(
                    &engine,
                    &format!("exception_bind(exception_bind({m}, {f}), {g})"),
                    &format!(
                        "exception_bind({m}, bind(a: Int, exception_bind(apply({f}, @a), {g})))"
                    ),
                );
            }
        }
    }

    expect(
        &engine,
        "meaning(plus(fail(exception_error), quotient(literal(1), literal(0))))",
        "exception_raised(exception_error)",
    );
    expect(
        &engine,
        "meaning(quotient(literal(1), literal(0)))",
        "exception_raised(exception_zero_divisor)",
    );
}

#[test]
fn wadler_denotations_are_state_functions_with_monad_laws_and_correct_counts() {
    let engine = engine(WADLER);
    expect(
        &engine,
        "apply(meaning(quotient(quotient(literal(1972), literal(2)), literal(23))), 0)",
        "state_answer(42, 2)",
    );
    for a in 0..=4 {
        for b in 1..=3 {
            for initial in [-1, 0, 7] {
                expect(
                    &engine,
                    &format!("apply(meaning(quotient(literal({a}), literal({b}))), {initial})"),
                    &format!("state_answer({}, {})", a / b, initial + 1),
                );
            }
        }
    }

    let f = "bind(x: Int, state_bind(state_tick, bind(u: Int, state_unit(add(@x, 1)))))";
    let g = "bind(x: Int, state_bind(state_tick, bind(u: Int, state_unit(mul(@x, 2)))))";

    for state in [0, 5] {
        equal(
            &engine,
            &format!("apply(state_bind(state_unit(3), {f}), {state})"),
            &format!("apply(apply({f}, 3), {state})"),
        );

        for m in [
            "state_unit(2)",
            "state_tick",
            "meaning(quotient(literal(8), literal(2)))",
        ] {
            equal(
                &engine,
                &format!("apply(state_bind({m}, bind(x: Int, state_unit(@x))), {state})"),
                &format!("apply({m}, {state})"),
            );
            equal(
                &engine,
                &format!("apply(state_bind(state_bind({m}, {f}), {g}), {state})"),
                &format!(
                    "apply(state_bind({m}, bind(a: Int, state_bind(apply({f}, @a), {g}))), {state})"
                ),
            );
        }
    }

    let undefined = engine.evaluate(
        &parse_term("apply(meaning(quotient(literal(1), literal(0))), 0)").unwrap(),
        &Config::default(),
    );

    assert!(matches!(undefined.status, Status::Error(_)));
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

#[test]
fn probability_has_exact_weights_missing_mass_and_kleisli_laws() {
    let engine = engine(PROBABILITY);

    for denominator in 1..=4 {
        for numerator in 0..=denominator {
            let query = format!(
                "meaning(coin(fraction({numerator}, {denominator}), certain(true), certain(false)))"
            );
            let result = evaluate(&engine, &query);
            let Term::App(name, args) = result else {
                panic!("expected valuation")
            };

            assert_eq!(name, "prob_weights");
            for (prob_mass, expected) in args.iter().zip([denominator - numerator, numerator]) {
                let (n, d) = fraction(prob_mass);

                assert_eq!(n * denominator, expected * d);
            }

            expect(
                &engine,
                &format!("rle(prob_mass({query}), fraction(1, 1))"),
                "true",
            );
        }
    }

    expect(
        &engine,
        "prob_equal(meaning(coin(fraction(1, 2), certain(true), diverge)), prob_weights(fraction(0, 1), fraction(1, 2)))",
        "true",
    );
    expect(
        &engine,
        "rle(prob_mass(meaning(coin(fraction(1, 2), certain(true), diverge))), fraction(1, 2))",
        "true",
    );
    expect(
        &engine,
        "prob_le(prob_unit(true), meaning(diverge))",
        "false",
    );
    let f = "bind(x: Bool, if(@x, prob_unit(false), meaning(coin(fraction(1, 2), certain(true), diverge))))";
    let g = "bind(x: Bool, if(@x, meaning(diverge), prob_unit(true)))";

    for b in ["false", "true"] {
        expect(
            &engine,
            &format!("prob_equal(prob_bind(prob_unit({b}), {f}), apply({f}, {b}))"),
            "true",
        );
    }

    for m in [
        "prob_unit(false)",
        "prob_unit(true)",
        "meaning(diverge)",
        "meaning(coin(fraction(1, 2), certain(true), certain(false)))",
    ] {
        expect(
            &engine,
            &format!("prob_equal(prob_bind({m}, bind(x: Bool, prob_unit(@x))), {m})"),
            "true",
        );
        // Denotational evaluation avoids duplicating intermediate rational expressions.

        let query = format!(
            "prob_equal(prob_bind(prob_bind({m}, {f}), {g}), prob_bind({m}, bind(a: Bool, prob_bind(apply({f}, @a), {g}))))"
        );
        let result = engine.denote(&parse_term(&query).unwrap(), &Config::default());

        assert_eq!(result.status, Status::NormalForm, "{query}: {result:?}");
        assert_eq!(result.term, Term::Bool(true));
    }

    let invalid = engine.evaluate(
        &parse_term("meaning(coin(fraction(2, 1), certain(true), certain(false)))").unwrap(),
        &Config::default(),
    );

    assert!(matches!(invalid.status, Status::Stuck(_)));
}

#[test]
fn all_five_paper_models_check_and_the_shipped_denotations_agree() {
    for source in [SCOTT, PLOTKIN, MOGGI, WADLER, PROBABILITY] {
        let module = load_module_source("<paper>", source, std::path::Path::new("."))
            .unwrap()
            .module;
        let engine = engine(source);

        for term in &module.evaluations {
            let a = engine.evaluate(term, &Config::default());
            let b = engine.denote(term, &Config::default());

            assert_eq!(
                a.status,
                Status::NormalForm,
                "{}: {term}: {a:?}",
                module.name
            );
            assert_eq!(
                b.status,
                Status::NormalForm,
                "{}: {term}: {b:?}",
                module.name
            );
            assert_eq!(a.term, b.term, "{}: {term}", module.name);
        }
    }
}

#[test]
fn five_papers_execute_via_the_cli_denote_command() {
    for name in [
        "scott-strachey",
        "plotkin-powerdomain",
        "moggi-exceptions",
        "wadler-state",
        "jones-plotkin-probability",
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_soda"))
            .current_dir(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."))
            .args(["denote", &format!("examples/denotational/{name}.soda")])
            .output()
            .unwrap();

        assert!(
            output.status.success(),
            "{name}: {}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
