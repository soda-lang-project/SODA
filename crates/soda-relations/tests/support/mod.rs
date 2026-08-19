use soda_relations::{Relations, SearchResult};

pub fn assert_single_verified_answer(relations: &Relations, result: &SearchResult) {
    assert!(result.complete);
    assert_eq!(result.answers.len(), 1);
    relations.verify(&result.answers[0].derivation).unwrap();
}
