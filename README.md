# 🧙🪄 ⠶ whichdoc

[![crates.io](https://img.shields.io/crates/v/whichdoc.svg)](https://crates.io/crates/whichdoc)
[![documentation](https://docs.rs/whichdoc/badge.svg)](https://docs.rs/whichdoc)
[![MIT/Apache-2.0 licensed](https://img.shields.io/crates/l/whichdoc.svg)](./LICENSE)
[![pre-commit.ci status](https://results.pre-commit.ci/badge/github/lmmx/whichdoc/master.svg)](https://results.pre-commit.ci/latest/github/lmmx/whichdoc/master)

A cargo documentation diagnostics-driven editor.

## vim-like docstring editor

whichdoc uses ratatui to give a "diagnostics picker" list of cargo docs `missing_docs` errors,
and [edtui][edtui] to emulate a vim editor in which to write your docstrings.

[edtui]: https://github.com/preiter93/edtui

## Licensing

WhichDoc is [MIT licensed](https://github.com/lmmx/whichdoc/blob/master/LICENSE), a permissive open source license.
