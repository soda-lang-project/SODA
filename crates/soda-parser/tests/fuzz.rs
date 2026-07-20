use soda_parser::{parse_module, parse_term, parse_term_in};

#[test]
fn bounded_mixed_unicode_inputs_never_panic_and_have_valid_diagnostic_offsets() {
    let module =
        parse_module("module M; judgment j(A,A); notation j \"$0 → $1\" ascii \"$0 -> $1\";")
            .unwrap();
    let alphabet = [
        'a', 'Γ', 'λ', 'ρ', '0', '9', '?', '@', '(', ')', '[', ']', '{', '}', ':', ';', '=', ',',
        '-', '+', '>', '|', '$', '"', '\\', '\n', ' ', '⊢', '→', '⟦', '⟧', '⊥', '🦀', '\0',
        '\u{301}',
    ];
    let mut state = 0x534f44415f46555au64;

    for sample in 0..12_000 {
        let length = sample % 81;
        let mut source = String::new();

        for _ in 0..length {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            source.push(alphabet[(state >> 32) as usize % alphabet.len()]);
        }

        for result in [
            parse_term(&source).map(|_| ()),
            parse_term_in(&module, &source).map(|_| ()),
            parse_module(&source).map(|_| ()),
        ] {
            if let Err(error) = result {
                assert!(source.is_char_boundary(error.offset), "{source:?}");
                assert!(error.line > 0 && error.column > 0);
            }
        }

        let template_source =
            format!("module M; judgment j(A); notation j \"{source}\" ascii \"$0 ->\";");
        let _ = parse_module(&template_source);
    }
}
