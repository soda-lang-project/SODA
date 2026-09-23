use soda_model::{Config, InvariantResult, Reachability, explore};
use soda_parser::{parse_module, parse_term};
use soda_semantics::Engine;

#[test]
fn mutex_excludes_simultaneous_entry_and_both_workers_can_finish() {
    let module = parse_module(include_str!("../../../examples/concurrency.soda")).unwrap();
    let engine = Engine::new(module.rules).unwrap();
    let graph = explore(&engine, &module.evaluations[0], &Config::default());
    let inside = parse_term("inside").unwrap();

    assert!(graph.is_complete());
    assert_eq!(
        graph.check_invariant(
            |term| !(term.at(&[1]) == Some(&inside) && term.at(&[2]) == Some(&inside))
        ),
        InvariantResult::Holds
    );
    let finished = parse_term("state(free, done, done)").unwrap();
    let Reachability::Reachable { witness, .. } = graph.reachability(&finished) else {
        panic!("both workers should be able to finish");
    };

    assert_eq!(witness.len(), 4);

    for edge in witness {
        let source = &graph.states[edge.from].term;
        let target = &graph.states[edge.to].term;

        assert!(
            engine
                .successors(source, &Default::default())
                .unwrap()
                .iter()
                .any(|step| step.rule == edge.rule
                    && source.replace(&step.path, step.after.clone()).as_ref() == Some(target))
        );
    }
}
