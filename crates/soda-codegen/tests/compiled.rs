use soda_ast::{Clause, Equation, Module, Proof, ProofStep, Rule, Term};
use soda_codegen::emit_rust;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "soda-codegen-{}-{}",
            std::process::id(),
            NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).expect("create unique codegen fixture");

        Self(path)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn succeeded(output: Output) -> Output {
    assert!(
        output.status.success(),
        "command failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    output
}

#[test]
fn emitted_rust_compiles_without_warnings_and_preserves_every_field() {
    let fixture = Fixture::new();
    let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    let ast_source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../soda-ast/src/lib.rs");
    let ast_library = fixture.0.join("libsoda_ast.rlib");
    succeeded(
        Command::new(&rustc)
            .args([
                "--edition=2024",
                "--crate-name=soda_ast",
                "--crate-type=rlib",
                "-Dwarnings",
            ])
            .arg(ast_source)
            .arg("-o")
            .arg(&ast_library)
            .output()
            .expect("run rustc for soda-ast"),
    );

    let unusual = "quote\" slash\\ newline\n tab\t nul\0 unicode λ 🦀\r\n\u{202e}";
    let variable = Term::Var(unusual.into());
    let application = Term::app(
        unusual,
        vec![variable.clone(), Term::Int(i64::MIN), Term::Bool(false)],
    );
    let module = Module {
        name: unusual.into(),
        imports: vec![unusual.into()],
        records: vec![soda_ast::RecordDecl {
            name: unusual.into(),
            constructor: unusual.into(),
            fields: vec![(unusual.into(), unusual.into())],
        }],
        monads: vec![soda_ast::MonadDecl {
            name: unusual.into(),
            bind: unusual.into(),
            unit: unusual.into(),
        }],
        clauses: vec![
            Clause {
                name: unusual.into(),
                conclusion: variable.clone(),
                premises: vec![application.clone(), Term::Bool(true)],
            },
            Clause {
                name: "fact".into(),
                conclusion: application.clone(),
                premises: vec![],
            },
        ],
        queries: vec![application.clone(), variable.clone()],
        rules: vec![
            Rule {
                name: unusual.into(),
                lhs: application.clone(),
                rhs: Term::Int(i64::MAX),
                condition: Some(Term::Bool(true)),
            },
            Rule {
                name: "unconditional".into(),
                lhs: variable.clone(),
                rhs: application.clone(),
                condition: None,
            },
        ],
        equations: vec![Equation {
            name: unusual.into(),
            lhs: application.clone(),
            rhs: variable.clone(),
        }],
        proofs: vec![
            Proof {
                name: unusual.into(),
                lhs: application.clone(),
                rhs: variable.clone(),
                steps: vec![
                    ProofStep {
                        axiom: unusual.into(),
                        reverse: true,
                        path: vec![0, 3, usize::MAX],
                        substitution: [
                            (unusual.into(), application.clone()),
                            ("z".into(), Term::Bool(true)),
                        ]
                        .into(),
                    },
                    ProofStep {
                        axiom: "empty".into(),
                        reverse: false,
                        path: vec![],
                        substitution: Default::default(),
                    },
                ],
            },
            Proof {
                name: "reflexive".into(),
                lhs: variable.clone(),
                rhs: variable.clone(),
                steps: vec![],
            },
        ],
        clause_details: vec![soda_ast::ClauseDetail {
            clause: unusual.into(),
            premise_labels: vec![Some(unusual.into()), None],
            bindings: vec![soda_ast::ComputedBinding {
                before_premise: 1,
                pattern: variable.clone(),
                value: application.clone(),
            }],
            conditions: vec![application.clone(), variable.clone()],
        }],
        domains: vec![
            soda_ast::DomainDecl {
                name: unusual.into(),
                expression: soda_ast::DomainExpr::Flat(vec![variable.clone()]),
            },
            soda_ast::DomainDecl {
                name: unusual.into(),
                expression: soda_ast::DomainExpr::Product(unusual.into(), unusual.into()),
            },
            soda_ast::DomainDecl {
                name: unusual.into(),
                expression: soda_ast::DomainExpr::Function(unusual.into(), unusual.into()),
            },
            soda_ast::DomainDecl {
                name: unusual.into(),
                expression: soda_ast::DomainExpr::Lifted(unusual.into()),
            },
        ],
        semantic_maps: vec![soda_ast::SemanticMap {
            name: unusual.into(),
            input: unusual.into(),
            output: unusual.into(),
            entries: vec![(variable.clone(), application.clone())],
        }],
        fixed_points: vec![unusual.into()],
        inductions: vec![soda_ast::InductionProof {
            name: unusual.into(),
            variable: unusual.into(),
            sort: unusual.into(),
            lhs: application.clone(),
            rhs: variable.clone(),
            cases: vec![soda_ast::InductionCase {
                constructor: unusual.into(),
                arguments: vec![unusual.into()],
                steps: vec![ProofStep {
                    axiom: unusual.into(),
                    reverse: true,
                    path: vec![0],
                    substitution: [(unusual.into(), variable.clone())].into(),
                }],
            }],
        }],
        sorts: vec![soda_ast::SortDecl {
            name: unusual.into(),
            inductive: true,
        }],
        signatures: vec![soda_ast::Signature {
            name: unusual.into(),
            arguments: vec![unusual.into()],
            result: unusual.into(),
            kind: soda_ast::SignatureKind::Constructor,
        }],
        judgments: vec![soda_ast::JudgmentDecl {
            name: unusual.into(),
            arguments: vec![unusual.into()],
            kind: soda_ast::JudgmentKind::Typing,
        }],
        notations: vec![soda_ast::NotationDecl {
            judgment: unusual.into(),
            unicode: "⟦ $0 ⟧".into(),
            ascii: "[| $0 |]".into(),
        }],
        evaluations: vec![
            Term::Name(unusual.into()),
            Term::Binder {
                parameter: unusual.into(),
                sort: unusual.into(),
                body: Box::new(Term::Name(unusual.into())),
            },
            application,
            Term::app("nullary", vec![]),
            Term::Int(i64::MAX),
            Term::Bool(true),
            variable,
        ],
    };

    for (index, original) in [Module::default(), module].into_iter().enumerate() {
        let source = fixture.0.join(format!("module_{index}.rs"));
        let binary = fixture
            .0
            .join(format!("module_{index}{}", std::env::consts::EXE_SUFFIX));
        let code = format!(
            "{}\nfn main() {{ println!(\"{{:?}}\", module()); }}\n",
            emit_rust(&original)
        );
        fs::write(&source, code).unwrap();
        succeeded(
            Command::new(&rustc)
                .args(["--edition=2024", "-Dwarnings", "--extern"])
                .arg(format!("soda_ast={}", ast_library.display()))
                .arg(&source)
                .arg("-o")
                .arg(&binary)
                .output()
                .expect("compile emitted Rust"),
        );
        let output = succeeded(Command::new(binary).output().expect("execute emitted Rust"));

        assert_eq!(
            String::from_utf8(output.stdout).unwrap().trim_end(),
            format!("{original:?}")
        );
    }
}
