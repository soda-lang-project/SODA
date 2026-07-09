use super::{Doc, Item, ItemKind};
use crate::lexer::Kind;

struct Family<'a> {
    keyword: &'a str,
    name: Option<&'a str>,
    subject: Option<&'a str>,
    defined_function: bool,
}

/// A top-level semicolon finishes a declaration. Declaration blocks may omit it.
pub(super) fn module<'a>(items: &'a [Item<'a>]) -> Doc<'a> {
    let mut parts = Vec::new();
    let mut start = 0;
    let mut previous = None;
    let mut index = 0;
    while index < items.len() {
        let item = &items[index];
        let first = items[start..=index].iter().find_map(Item::word);
        let proof_end = matches!(
            first,
            Some("proof" | "induction" | "datatype" | "function" | "map")
        ) && matches!(&item.kind, ItemKind::Group { open: "{", .. })
            && items[index + 1..]
                .iter()
                .find(|i| !matches!(i.kind, ItemKind::Blank) && !i.is_comment())
                .is_none_or(|next| {
                    next.word()
                        .is_some_and(|word| crate::DeclarationKind::from_keyword(word).is_some())
                });
        if item.symbol(';') || proof_end {
            let mut end = index + 1;
            if items
                .get(end)
                .is_some_and(|next| matches!(next.kind, ItemKind::Comment(_, true)))
            {
                end += 1;
            }
            push_statement(&mut parts, &items[start..end], &mut previous);
            start = end;
            index = end;
        } else {
            index += 1;
        }
    }
    if start < items.len() {
        push_statement(&mut parts, &items[start..], &mut previous);
    }
    Doc::concat(parts)
}

fn push_statement<'a>(
    parts: &mut Vec<Doc<'a>>,
    items: &'a [Item<'a>],
    previous: &mut Option<Family<'a>>,
) {
    let cleaned = items
        .iter()
        .enumerate()
        .filter(|(index, item)| {
            !matches!(item.kind, ItemKind::Blank)
                || index
                    .checked_sub(1)
                    .and_then(|i| items.get(i))
                    .is_some_and(Item::is_comment)
                    && items.get(index + 1).is_some_and(Item::is_comment)
        })
        .map(|(_, item)| item)
        .collect::<Vec<_>>();
    let code = cleaned
        .iter()
        .copied()
        .skip_while(|i| i.is_comment() || matches!(i.kind, ItemKind::Blank))
        .collect::<Vec<_>>();
    let current = items.iter().find_map(Item::word);
    let family = current.map(|keyword| Family {
        keyword,
        defined_function: keyword == "function"
            && code.iter().any(|item| {
                matches!(
                    item.kind,
                    ItemKind::Atom(Kind::Arrow, _) | ItemKind::Group { open: "{", .. }
                )
            }),
        name: code.get(1).and_then(|i| i.word()),
        subject: code
            .iter()
            .position(|i| i.symbol(':'))
            .and_then(|index| code.get(index + 1))
            .and_then(|i| i.word()),
    });
    let related = previous
        .as_ref()
        .zip(family.as_ref())
        .is_some_and(|(a, b)| {
            a.keyword == b.keyword
                && matches!(a.keyword, "rule" | "infer" | "axiom")
                && (if a.subject.is_some() && b.subject.is_some() {
                    a.subject == b.subject
                } else {
                    a.name
                        .and_then(|name| name.rsplit_once('_'))
                        .zip(b.name.and_then(|name| name.rsplit_once('_')))
                        .is_some_and(|(a, b)| a.0 == b.0)
                })
        });
    let compact_inputs = previous
        .as_ref()
        .zip(family.as_ref())
        .is_some_and(|(a, b)| a.keyword == b.keyword && matches!(a.keyword, "eval" | "query"));
    let previous_keyword = previous.as_ref().map(|family| family.keyword);
    let compact_declarations = matches!(
        (previous_keyword, current),
        (Some("import"), Some("import")) | (Some("datatype"), Some("constructor"))
    );
    let first_code = items
        .iter()
        .position(|i| !matches!(i.kind, ItemKind::Blank) && !i.is_comment())
        .unwrap_or(items.len());
    let blank = (code.len() < cleaned.len()
        || !(related || compact_inputs || compact_declarations))
        && items[..first_code]
            .iter()
            .any(|i| matches!(i.kind, ItemKind::Blank));
    let substantial = matches!(
        current,
        Some("rule" | "infer" | "axiom" | "proof" | "induction" | "datatype")
    ) && !related
        || family
            .as_ref()
            .is_some_and(|family| family.defined_function)
        || previous
            .as_ref()
            .is_some_and(|family| family.defined_function);
    let category_change = previous.is_some()
        && current != previous_keyword
        && !matches!(
            (previous_keyword, current),
            (Some("import"), Some("import")) | (Some("datatype"), Some("constructor"))
        );
    if !parts.is_empty() {
        parts.push(if blank || substantial || category_change {
            Doc::BlankLine
        } else {
            Doc::HardLine
        });
    }
    parts.push(statement(&cleaned));
    if current.is_some() {
        *previous = family;
    }
}

fn statement<'a>(items: &[&'a Item<'a>]) -> Doc<'a> {
    if let Some(last) = items.last()
        && let ItemKind::Comment(text, true) = last.kind
    {
        return Doc::concat(vec![
            statement(&items[..items.len() - 1]),
            Doc::Text(" "),
            Doc::Text(text),
        ]);
    }

    let leading = items
        .iter()
        .take_while(|item| item.is_comment() || matches!(item.kind, ItemKind::Blank))
        .count();
    if leading > 0 {
        let mut parts = Vec::new();
        for item in &items[..leading] {
            if let ItemKind::Comment(text, _) = item.kind {
                parts.push(Doc::Text(text));
                parts.push(Doc::HardLine);
            }
            if matches!(item.kind, ItemKind::Blank) {
                parts.push(Doc::BlankLine);
            }
        }
        parts.push(statement(&items[leading..]));
        return Doc::concat(parts);
    }
    let keyword = items.first().and_then(|i| i.word());
    let arrow = if matches!(
        keyword,
        Some("rule" | "axiom" | "infer" | "case" | "function")
    ) {
        items.iter().position(|i| {
            matches!(i.kind, ItemKind::Atom(Kind::Arrow, _))
                || keyword == Some("axiom") && i.symbol('=')
        })
    } else {
        None
    };
    if let Some(arrow) = arrow {
        let mut body = vec![Doc::Line(" ")];
        let condition = items[arrow + 1..]
            .iter()
            .position(|i| i.word() == Some("when"))
            .filter(|index| *index > 0)
            .map(|i| i + arrow + 1);
        if let Some(colon) = items[..arrow].iter().position(|i| i.symbol(':'))
            && keyword != Some("function")
            && (items
                .get(1)
                .and_then(|i| i.word())
                .is_some_and(|name| name.chars().count() >= 20)
                || sequence(items).width() > 100)
        {
            let mut rule = vec![sequence(&items[colon + 1..arrow]), Doc::HardLine];
            if let Some(condition) = condition {
                rule.push(sequence(&items[arrow..condition]));
                rule.push(Doc::HardLine);
                rule.push(sequence(&items[condition..]));
            } else {
                rule.push(sequence(&items[arrow..]));
            }
            return Doc::concat(vec![
                sequence(&items[..=colon]),
                Doc::concat(vec![Doc::HardLine, Doc::concat(rule)]).nest(),
            ]);
        }
        if let Some(condition) = condition {
            body.push(sequence(&items[arrow + 1..condition]));
            body.push(Doc::Line(" "));
            body.push(sequence(&items[condition..]));
        } else {
            body.push(sequence(&items[arrow + 1..]));
        }
        return Doc::concat(vec![sequence(&items[..=arrow]), Doc::concat(body).nest()]).group();
    }
    if keyword == Some("infer")
        && let Some(premise) = items
            .iter()
            .position(|i| matches!(i.word(), Some("if" | "let" | "when")))
    {
        return Doc::concat(vec![
            sequence(&items[..premise]),
            Doc::concat(vec![Doc::Line(" "), premises(&items[premise..])]).nest(),
        ])
        .group();
    }
    sequence(items).group()
}

fn sequence<'a>(items: &[&'a Item<'a>]) -> Doc<'a> {
    let mut parts = Vec::new();
    let mut previous = None;
    let lambda = items.first().and_then(|item| item.word()) != Some("import")
        && items.iter().any(|item| {
            item.word() == Some("lambda")
                || matches!(item.kind, ItemKind::Atom(Kind::Symbol('λ'), _))
        });
    let judgment = items
        .iter()
        .any(|item| matches!(item.kind, ItemKind::Atom(Kind::Symbol('⊢' | '⊨'), _)))
        || items
            .windows(2)
            .any(|pair| pair[0].symbol('|') && pair[1].symbol('-'));
    let header_colon = matches!(
        items.first().and_then(|item| item.word()),
        Some("rule" | "infer" | "axiom" | "proof" | "induction")
    )
    .then(|| items.iter().position(|item| item.symbol(':')))
    .flatten();
    for (index, item) in items.iter().enumerate() {
        match &item.kind {
            ItemKind::Blank => continue,
            ItemKind::Comment(text, inline) => {
                if previous.is_some() {
                    parts.push(if *inline {
                        Doc::Text(" ")
                    } else {
                        Doc::HardLine
                    });
                }
                parts.push(Doc::Text(text));
                parts.push(Doc::HardLine);
                previous = None;
                continue;
            }
            _ => {}
        }
        if item.word() == Some("case") && previous.is_some() {
            parts.push(Doc::HardLine);
        } else if let Some(prev) = previous
            && (space(prev, item)
                || lambda && prev.symbol('.')
                || judgment && item.symbol(':') && header_colon != Some(index))
        {
            parts.push(Doc::Text(" "));
        }
        parts.push(match &item.kind {
            ItemKind::Atom(_, text) => Doc::Text(text),
            ItemKind::Conditional(branches) => conditional(branches),
            ItemKind::Group {
                open,
                close,
                children,
            } => group(open, close, children),
            _ => unreachable!(),
        });
        previous = Some(*item);
    }
    Doc::concat(parts)
}

fn premises<'a>(items: &[&'a Item<'a>]) -> Doc<'a> {
    if items.first().and_then(|item| item.word()) != Some("if") {
        return sequence(items);
    }
    let mut parts = Vec::new();
    let mut start = 1;
    for (index, item) in items.iter().enumerate().skip(1) {
        if item.symbol(',') {
            parts.push(sequence(&items[start..=index]));
            parts.push(Doc::Line(" "));
            start = index + 1;
        }
    }
    parts.push(sequence(&items[start..]));
    Doc::concat(vec![Doc::Text("if "), Doc::concat(parts).align().group()])
}

fn contains_conditional(item: &Item<'_>) -> bool {
    match &item.kind {
        ItemKind::Conditional(_) => true,
        ItemKind::Group { children, .. } => children.iter().any(contains_conditional),
        _ => false,
    }
}

fn contains_comment(item: &Item<'_>) -> bool {
    match &item.kind {
        ItemKind::Comment(..) => true,
        ItemKind::Group { children, .. } => children.iter().any(contains_comment),
        ItemKind::Conditional(branches) => branches.iter().flatten().any(contains_comment),
        _ => false,
    }
}

fn conditional<'a>(branches: &'a [Vec<Item<'a>>; 3]) -> Doc<'a> {
    let references = branches
        .iter()
        .map(|branch| branch.iter().collect::<Vec<_>>())
        .collect::<Vec<_>>();
    let compact = Doc::concat(vec![
        Doc::Text("if "),
        sequence(&references[0]),
        Doc::Text(" then "),
        sequence(&references[1]),
        Doc::Text(" else "),
        sequence(&references[2]),
    ]);
    let nested = branches.iter().flatten().any(contains_conditional);
    if compact.width() <= 72 && !nested {
        return compact.group();
    }
    let branch = |index: usize| {
        if branches[index]
            .iter()
            .any(|item| contains_conditional(item) || contains_comment(item))
        {
            Doc::concat(vec![Doc::HardLine, sequence(&references[index])]).nest()
        } else {
            Doc::concat(vec![Doc::Text(" "), sequence(&references[index])])
        }
    };
    Doc::concat(vec![
        Doc::Text("if"),
        if branches[0].iter().any(contains_comment) {
            Doc::concat(vec![Doc::HardLine, sequence(&references[0])]).nest()
        } else {
            Doc::concat(vec![Doc::Text(" "), sequence(&references[0])])
        },
        Doc::HardLine,
        Doc::Text("then"),
        branch(1),
        Doc::HardLine,
        Doc::Text("else"),
        branch(2),
    ])
    .align()
}

fn group<'a>(open: &'a str, close: &'a str, children: &'a [Item<'a>]) -> Doc<'a> {
    let children = children
        .iter()
        .filter(|i| !matches!(i.kind, ItemKind::Blank))
        .collect::<Vec<_>>();
    if children.is_empty() {
        return Doc::concat(vec![Doc::Text(open), Doc::Text(close)]);
    }
    let block = open == "{" && children.iter().any(|i| i.symbol(';'));
    let mut parts = Vec::new();
    let mut start = 0;
    let mut index = 0;
    while index < children.len() {
        let item = children[index];
        if item.symbol(',') || block && item.symbol(';') {
            let inline = children
                .get(index + 1)
                .is_some_and(|item| matches!(item.kind, ItemKind::Comment(_, true)));
            let end = index + 1 + usize::from(inline);
            parts.push(statement(&children[start..end]));
            parts.push(if block || inline {
                Doc::HardLine
            } else {
                Doc::Line(" ")
            });
            start = end;
            index = end;
        } else {
            index += 1;
        }
    }
    parts.push(statement(&children[start..]));
    let space = if open == "{" { " " } else { "" };
    let boundary = || {
        if block {
            Doc::HardLine
        } else {
            Doc::Line(space)
        }
    };
    Doc::concat(vec![
        Doc::Text(open),
        Doc::concat(vec![boundary(), Doc::concat(parts)]).nest(),
        boundary(),
        Doc::Text(close),
    ])
    .group()
}

fn space(previous: &Item<'_>, next: &Item<'_>) -> bool {
    if matches!(
        next.kind,
        ItemKind::Atom(Kind::Symbol(',' | ':' | ';' | '.'), _)
    ) {
        return false;
    }
    if matches!(previous.kind, ItemKind::Atom(Kind::Symbol('?' | '@'), _)) {
        return false;
    }
    if matches!(previous.kind, ItemKind::Atom(Kind::Symbol('.' | 'λ'), _)) {
        return next.space_before;
    }
    if matches!(next.kind, ItemKind::Group { open: "(", .. }) && previous.word().is_some() {
        return false;
    }
    if matches!(next.kind, ItemKind::Group { open: "[", .. }) {
        return next.space_before;
    }
    if let (ItemKind::Atom(Kind::Symbol(a), _), ItemKind::Atom(Kind::Symbol(b), _)) =
        (&previous.kind, &next.kind)
        && "-+>|~!<>/\\=:".contains(*a)
        && "-+>|~!<>/\\=:".contains(*b)
    {
        return next.space_before;
    }
    true
}
