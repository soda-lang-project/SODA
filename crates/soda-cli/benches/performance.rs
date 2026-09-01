use soda_ast::{Clause, Rule, Substitution, Term};
use soda_relations::{Relations, SearchConfig};
use soda_semantics::{Config, Engine, Status};
use std::{
    hint::black_box,
    time::{Duration, Instant},
};

fn measure(name: &str, mut work: impl FnMut()) {
    if let Some(filter) = std::env::args().skip(1).find(|arg| !arg.starts_with('-'))
        && !name.contains(&filter)
    {
        return;
    }

    work();
    let start = Instant::now();
    let mut iterations = 0;

    while start.elapsed() < Duration::from_millis(750) {
        work();
        iterations += 1;
    }

    println!(
        "{name}: {:.2} us/iteration ({iterations} iterations)",
        start.elapsed().as_secs_f64() * 1e6 / iterations as f64
    );
}

fn main() {
    let mut rules: Vec<_> = (0..1000)
        .map(|i| Rule {
            name: format!("unused_{i}"),
            lhs: Term::app(format!("unused_{i}"), vec![]),
            rhs: Term::Int(0),
            condition: None,
        })
        .collect();
    rules.push(Rule {
        name: "tick".into(),
        lhs: Term::app("tick", vec![]),
        rhs: Term::Int(1),
        condition: None,
    });
    let engine = Engine::new(rules).unwrap();
    let wide = Term::app("data", vec![Term::app("tick", vec![]); 200]);
    let config = Config::default();
    measure("evaluate/1000-rules-wide-200", || {
        let result = engine.evaluate(black_box(&wide), &config);
        assert_eq!(result.status, Status::NormalForm);
        assert_eq!(result.steps, 200);
        black_box(result);
    });
    measure("denote/1000-rules-wide-200", || {
        let result = engine.denote(black_box(&wide), &config);
        assert_eq!(result.status, Status::NormalForm);
        black_box(result);
    });
    measure("successors/1000-rules-wide-200", || {
        assert_eq!(
            black_box(engine.successors(black_box(&wide), &config).unwrap()).len(),
            200
        );
    });
    let simple = Engine::new(vec![Rule {
        name: "tick".into(),
        lhs: Term::app("tick", vec![]),
        rhs: Term::Int(1),
        condition: None,
    }])
    .unwrap();
    measure("evaluate/1-rule-wide-200", || {
        black_box(simple.evaluate(black_box(&wide), &config));
    });
    let substitution: Substitution = (0..100).map(|i| (format!("v{i}"), Term::Int(i))).collect();
    let binder = Term::Binder {
        parameter: "x".into(),
        sort: "Int".into(),
        body: Box::new(Term::app(
            "data",
            (0..100).map(|i| Term::Var(format!("v{i}"))).collect(),
        )),
    };
    measure("substitute/binder-100-variables", || {
        black_box(binder.substitute(black_box(&substitution)));
    });
    let relations = Relations::new(
        (0..300)
            .map(|i| Clause {
                name: format!("c{i}"),
                conclusion: Term::app(format!("p{i}"), vec![Term::Var("x".into())]),
                premises: vec![],
            })
            .collect(),
    )
    .unwrap();
    let query = Term::app("p299", vec![Term::Int(42)]);
    measure("relations/300-clauses", || {
        let result = relations.search(black_box(&query), &SearchConfig::default());
        assert!(result.complete);
        assert_eq!(result.answers.len(), 1);
        black_box(result);
    });
    let overlap_rules = engine.rules()[..300].to_vec();
    measure("analysis/300-unrelated-rules", || {
        assert!(
            black_box(soda_verifier::critical_pairs(black_box(&overlap_rules)).unwrap())
                .pairs
                .is_empty()
        );
    });
    let source = "module Bench; domain D = flat(a,b); domain F = functions(D,D);";
    let module = soda_parser::parse_module(source).unwrap();
    measure("domains/function-space", || {
        black_box(
            soda_domains::Domains::new(black_box(&module.domains), Default::default()).unwrap(),
        );
    });
    let module = soda_parser::parse_module(
        "module Bench; domain D = flat(a,b,c); domain P = product(D,D); domain Q = product(P,P);",
    )
    .unwrap();
    let domains = soda_domains::Domains::new(&module.domains, Default::default()).unwrap();
    let product = domains.get("Q").unwrap();
    measure("domains/index-product-256", || {
        for (index, term) in product.elements().iter().enumerate() {
            assert_eq!(product.index(black_box(term)), Some(index));
        }
    });
    let mut source = "module Chain; domain D = flat(a,b); domain C0 = flat();".to_owned();

    for index in 1..12 {
        source.push_str(&format!("domain C{index} = lift(C{});", index - 1));
    }

    source.push_str("domain F = functions(C11,D);");
    let module = soda_parser::parse_module(&source).unwrap();
    measure("domains/chain-function-space", || {
        black_box(
            soda_domains::Domains::new(black_box(&module.domains), Default::default()).unwrap(),
        );
    });
}
