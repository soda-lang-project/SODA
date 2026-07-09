use crate::{ParseError, error::error, symbols};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Kind {
    Ident(String),
    Number(String),
    Text(String),
    Symbol(char),
    Arrow,
    End,
}
#[derive(Clone, Debug)]
pub struct Token {
    pub kind: Kind,
    pub offset: usize,
    pub end: usize,
}
pub fn lex(source: &str) -> Result<Vec<Token>, Box<ParseError>> {
    let mut tokens = Vec::new();
    let mut offset = 0;

    while offset < source.len() {
        let rest = &source[offset..];
        let ch = rest.chars().next().unwrap();
        let start = offset;

        if ch.is_whitespace() {
            offset += ch.len_utf8();
            continue;
        }

        if rest.starts_with("//") || ch == '#' {
            offset += rest.find('\n').unwrap_or(rest.len());
            continue;
        }

        let kind = if let Some((symbol, length)) = symbols::tex_symbol(rest) {
            offset += length;

            Kind::Symbol(symbol)
        } else if rest.starts_with("=>") {
            offset += 2;

            Kind::Arrow
        } else if ch == '"' {
            offset += 1;
            let mut value = String::new();
            let mut closed = false;

            while offset < source.len() {
                let next = source[offset..].chars().next().unwrap();
                offset += next.len_utf8();
                if next == '"' {
                    closed = true;
                    break;
                }

                if next == '\\' {
                    let escaped = source[offset..].chars().next().ok_or_else(|| {
                        error(source, start, "unterminated string")
                            .with_kind(crate::ParseErrorKind::UnterminatedString)
                    })?;
                    offset += escaped.len_utf8();
                    value.push(match escaped {
                        'n' => '\n',
                        'r' => '\r',
                        't' => '\t',
                        '"' => '"',
                        '\\' => '\\',
                        _ => {
                            return Err(error(
                                source,
                                offset - escaped.len_utf8(),
                                "unsupported string escape",
                            ));
                        }
                    });
                } else {
                    value.push(next);
                }
            }

            if !closed {
                return Err(error(source, start, "unterminated string")
                    .with_kind(crate::ParseErrorKind::UnterminatedString));
            }

            Kind::Text(value)
        } else if ch.is_alphabetic() && ch != 'λ' || ch == '_' {
            offset += ch.len_utf8();
            while let Some(next) = source[offset..].chars().next() {
                if !next.is_alphanumeric() && next != '_' {
                    break;
                }

                offset += next.len_utf8();
            }

            Kind::Ident(source[start..offset].into())
        } else if ch.is_ascii_digit()
            || ((ch == '-' || ch == '+') && rest.as_bytes().get(1).is_some_and(u8::is_ascii_digit))
        {
            offset += 1;
            while source
                .as_bytes()
                .get(offset)
                .is_some_and(u8::is_ascii_digit)
            {
                offset += 1;
            }

            Kind::Number(source[start..offset].into())
        } else if "?():;=,{}[]@$.-+>|~!<>/\\".contains(ch)
            || (!ch.is_ascii() && !ch.is_alphanumeric())
            || ch == 'λ'
        {
            offset += ch.len_utf8();

            Kind::Symbol(ch)
        } else {
            return Err(error(source, start, format!("unexpected character {ch:?}")));
        };

        tokens.push(Token {
            kind,
            offset: start,
            end: offset,
        });
    }

    tokens.push(Token {
        kind: Kind::End,
        offset: source.len(),
        end: source.len(),
    });

    Ok(tokens)
}
