use super::{Item, ItemKind};
use crate::{Locations, lexer::Kind};
use soda_ast::Term;
use soda_diagnostics::Span;
use std::{cmp::Reverse, collections::BTreeMap};

pub(super) fn rewrite<'a>(items: &[Item<'a>], locations: &Locations) -> Vec<Item<'a>> {
    let mut sites = BTreeMap::new();
    for location in &locations.terms {
        if let Term::App(name, args) = &location.term
            && name == "if"
            && args.len() == 3
        {
            let mut cursor = location.span.start;
            let mut spans = Vec::new();
            for arg in args {
                if let Some(site) = locations
                    .terms
                    .iter()
                    .filter(|s| {
                        s.term == *arg && s.span.start >= cursor && location.span.contains(s.span)
                    })
                    .min_by_key(|s| (s.span.start, Reverse(s.span.end)))
                {
                    spans.push(site.span);
                    cursor = site.span.end;
                }
            }
            if let Ok(spans) = <Vec<Span> as TryInto<[Span; 3]>>::try_into(spans) {
                sites.insert(location.span.start, (location.span.end, spans));
            }
        }
    }
    transform(items, &sites)
}

fn transform<'a>(items: &[Item<'a>], sites: &BTreeMap<usize, (usize, [Span; 3])>) -> Vec<Item<'a>> {
    let mut result = Vec::new();
    let mut index = 0;
    while index < items.len() {
        let item = &items[index];
        if matches!(item.kind, ItemKind::Atom(Kind::Ident(word), _) if word == "if")
            && let Some((end, args)) = sites.get(&item.start)
        {
            let count = items[index..].iter().take_while(|i| i.end <= *end).count();
            let consumed = &items[index..index + count];
            if count > 1 && consumed.last().unwrap().end == *end {
                let (arguments, separators) = if let Some(Item {
                    kind:
                        ItemKind::Group {
                            open: "(",
                            children,
                            ..
                        },
                    ..
                }) = consumed.get(1)
                    && count == 2
                {
                    (
                        children.as_slice(),
                        children
                            .iter()
                            .enumerate()
                            .filter(|(_, item)| item.symbol(','))
                            .map(|(index, _)| index)
                            .collect::<Vec<_>>(),
                    )
                } else {
                    let arguments = &consumed[1..];
                    let separators = ["then", "else"]
                        .into_iter()
                        .enumerate()
                        .filter_map(|(branch, keyword)| {
                            arguments.iter().position(|item| {
                                item.word() == Some(keyword)
                                    && item.start >= args[branch].end
                                    && item.end <= args[branch + 1].start
                            })
                        })
                        .collect::<Vec<_>>();
                    (arguments, separators)
                };
                if let [first, second] = separators.as_slice() {
                    let branches = [
                        &arguments[..*first],
                        &arguments[first + 1..*second],
                        &arguments[second + 1..],
                    ]
                    .map(|branch| transform(branch, sites));
                    if branches.iter().all(|branch| !branch.is_empty()) {
                        let mut conditional = item.clone();
                        conditional.end = *end;
                        conditional.kind = ItemKind::Conditional(branches);
                        result.push(conditional);
                        index += count;
                        continue;
                    }
                }
            }
        }
        let mut copy = item.clone();
        if let ItemKind::Group { children, .. } = &mut copy.kind {
            *children = transform(children, sites);
        }
        result.push(copy);
        index += 1;
    }
    result
}
