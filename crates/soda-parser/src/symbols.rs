pub(super) fn tex_symbol(source: &str) -> Option<(char, usize)> {
    let rest = source.strip_prefix('\\')?;
    let length = rest.bytes().take_while(u8::is_ascii_alphabetic).count();
    let symbol = match &rest[..length] {
        "llbracket" => '⟦',
        "rrbracket" => '⟧',
        "vdash" => '⊢',
        "models" | "vDash" => '⊨',
        "rightarrow" | "to" => '→',
        "Rightarrow" => '⇒',
        "Downarrow" => '⇓',
        "uparrow" => '↑',
        "Uparrow" => '⇑',
        "mapsto" => '↦',
        "equiv" => '≡',
        "lambda" => 'λ',
        "bot" => '⊥',
        _ => return None,
    };

    Some((symbol, length + 1))
}
