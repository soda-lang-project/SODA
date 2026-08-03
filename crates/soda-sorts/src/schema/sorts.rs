use super::TypeChecker;
use crate::{Sort, SortError};

impl TypeChecker {
    pub fn parse_sort(&self, source: &str) -> Result<Sort, SortError> {
        enum Task<'a> {
            Parse(&'a str, usize),
            Function,
        }

        let mut pending = vec![Task::Parse(source, 0)];
        let mut results = Vec::new();

        while let Some(task) = pending.pop() {
            let Task::Parse(source, depth) = task else {
                let output = results.pop().expect("function output was parsed");
                let input = results.pop().expect("function input was parsed");
                results.push(Sort::Function(Box::new(input), Box::new(output)));
                continue;
            };

            if depth >= crate::MAX_SORT_DEPTH {
                return Err(SortError::new(format!(
                    "sort nesting exceeds the limit of {}",
                    crate::MAX_SORT_DEPTH
                )));
            }

            let source = source.trim();

            if let Some(inner) = source.strip_prefix("Fn(").and_then(|s| s.strip_suffix(')')) {
                let mut parentheses = 0usize;
                let mut separator = None;

                for (index, ch) in inner.char_indices() {
                    match ch {
                        '(' => parentheses += 1,
                        ')' => {
                            parentheses = parentheses.checked_sub(1).ok_or_else(|| {
                                SortError::new(format!("invalid sort `{source}`"))
                            })?;
                        }
                        ',' if parentheses == 0 => {
                            separator = Some(index);
                            break;
                        }
                        _ => {}
                    }
                }

                if let Some(index) = separator {
                    pending.push(Task::Function);
                    pending.push(Task::Parse(&inner[index + 1..], depth + 1));
                    pending.push(Task::Parse(&inner[..index], depth + 1));
                    continue;
                }
            }

            if self.sorts.contains(source) {
                results.push(Sort::Named(source.into()));
            } else {
                return Err(
                    SortError::new(format!("unknown or malformed sort `{source}`")).symbol(source),
                );
            }
        }

        Ok(results.pop().expect("the root sort was parsed"))
    }
}
