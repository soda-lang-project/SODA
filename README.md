<p align="center">
  <img width="150" src="https://avatars.githubusercontent.com/u/344663535?s=200&v=4">
</p>

<h1 align="center">SODA</h1>

<p align="center">A DSL for Operational, Denotational and Axiomatic Semantics.</p>

SODA uses `.soda` files to describe rewrite rules, inference rules and equational
proofs. The command-line tool evaluates terms, checks certificates and explores
reachable states.

## Build

```sh
cargo build --release --locked
```

## Example

Rules define how terms reduce. This module evaluates `factorial(6)` to `720`:

```soda
module Arithmetic;

rule factorial_zero: factorial(0) => 1;
rule factorial_step: factorial(?n) => mul(?n, factorial(sub(?n, 1))) when gt(?n, 0);

eval factorial(6);
```

Run the arithmetic example, search inference rules or check an induction proof:

```sh
soda run examples/arithmetic.soda 'factorial(6)'
soda infer examples/relations.soda
soda prove examples/induction.soda
```

## Commands

Use `soda <command>` from the project root.

| Command | Purpose |
| --- | --- |
| `check` | Validate a module and its proof certificates. |
| `run` | Reduce terms using rewrite rules. |
| `denote` | Evaluate terms compositionally. |
| `compare` | Compare both evaluators on the supplied inputs. |
| `infer` | Search inference rules and construct derivations. |
| `prove` | Check equational and induction certificates. |
| `explore` | Explore reachable states and find witnesses. |
| `domains` | Inspect finite domains, semantic maps and fixed points. |
| `correspond` | Compare evaluation with a finite semantic map. |
| `analyze` | Inspect syntactic termination measures and critical pairs. |
| `fmt` | Format `.soda` files. |
| `emit-rust` | Export a module using the `soda-ast` types. |

Add `--trace` to show reduction steps or derivations. Run
`soda --help` for options and command usage.

The [examples](examples) cover typed judgments, state, exceptions, probability,
lambda calculus and concurrency.
