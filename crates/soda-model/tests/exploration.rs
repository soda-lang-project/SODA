use soda_ast::{Rule, Term};
use soda_model::{Config, Incomplete, InvariantResult, Reachability, StateStatus, explore};
use soda_semantics::{Engine, Limit, Status};

fn atom(name: &str) -> Term {
    Term::app(name, vec![])
}

fn engine(edges: &[(&str, &str, &str)]) -> Engine {
    Engine::new(
        edges
            .iter()
            .map(|(name, from, to)| Rule {
                name: (*name).into(),
                lhs: atom(from),
                rhs: atom(to),
                condition: None,
            })
            .collect(),
    )
    .unwrap()
}

#[test]
fn explores_both_choices_in_declaration_order() {
    let graph = explore(
        &engine(&[("left", "a", "b"), ("right", "a", "c")]),
        &atom("a"),
        &Config::default(),
    );

    assert!(graph.is_complete());
    assert_eq!(
        graph
            .states
            .iter()
            .map(|state| state.term.clone())
            .collect::<Vec<_>>(),
        vec![atom("a"), atom("b"), atom("c")]
    );
    assert_eq!(graph.transitions.len(), 2);
    assert_eq!(graph.transitions[0].rule, "left");
    assert_eq!(graph.transitions[1].rule, "right");
    assert_eq!(graph.states[0].status, StateStatus::Expanded);
    assert_eq!(graph.states[1].status, StateStatus::Terminal);
    assert_eq!(graph.states[2].status, StateStatus::Terminal);
    assert_eq!(
        graph.reachability(&atom("absent")),
        Reachability::Unreachable
    );
}

