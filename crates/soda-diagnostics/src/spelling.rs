const MIN_SUGGESTION_LENGTH: usize = 2;
const MAX_SUGGESTION_LENGTH: usize = 128;
const SHORT_NAME_LENGTH: usize = 5;
const SHORT_NAME_DISTANCE: usize = 1;
const LONG_NAME_DISTANCE: usize = 2;

pub fn nearest<'a>(name: &str, candidates: impl IntoIterator<Item = &'a str>) -> Option<&'a str> {
    fn distance(a: &str, b: &str) -> usize {
        let b = b.chars().collect::<Vec<_>>();
        let mut row = (0..=b.len()).collect::<Vec<_>>();

        for (i, a) in a.chars().enumerate() {
            let mut diagonal = row[0];
            row[0] = i + 1;

            for (j, &b) in b.iter().enumerate() {
                let previous = row[j + 1];
                row[j + 1] = (row[j] + 1)
                    .min(previous + 1)
                    .min(diagonal + usize::from(a != b));
                diagonal = previous;
            }
        }

        row[b.len()]
    }
    let length = name.chars().count();

    if !(MIN_SUGGESTION_LENGTH..=MAX_SUGGESTION_LENGTH).contains(&length) {
        return None;
    }

    candidates
        .into_iter()
        .filter(|candidate| {
            candidate.chars().count() <= MAX_SUGGESTION_LENGTH && *candidate != name
        })
        .map(|candidate| (distance(name, candidate), candidate))
        .filter(|(distance, _)| {
            *distance
                <= if length > SHORT_NAME_LENGTH {
                    LONG_NAME_DISTANCE
                } else {
                    SHORT_NAME_DISTANCE
                }
        })
        .min_by(|a, b| a.cmp(b))
        .map(|(_, candidate)| candidate)
}
