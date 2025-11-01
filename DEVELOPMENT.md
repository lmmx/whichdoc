## Prek/pre-commit

Pre-commit tools bundle all the dependencies for CI and you can just run these and execute
the tests you need to check. I use `prek`, a Rust port of `pre-commit`, both will work.
The CI runs pre-commit.

```sh
prek run --all-files
pre-commit run --all-files
```

## Justfile

To run all development checks, install the `just` task runner and:

```sh
just full
```

Requirements:

- Rust toolchain ([install via rustup](https://rustup.rs/))
- `cargo-nextest` (for testing: `cargo install cargo-nextest`)
- `cargo-machete` (for unused deps: `cargo install cargo-machete`)
- `taplo` (for TOML formatting: `cargo install taplo-cli`)
- `echo-comment` (for shell echoing: `cargo install echo-comment`)

To install precommit hooks with prek, run `just install-hooks` and `just run-pc` to run them.

## Release

The Rust release process is two commands (if it works the first time it could be one)

```sh
just ship
```

If the dry run fails, you can revert and re-run the last step when it succeeds (but if all is OK you
won't need to):

```sh
just publish
```
