//! Parse the compiler's diagnostic output (cargo doc diagnostics).
//!
//! These either come from a pipe over STDIN or we execute cargo doc ourselves.
//! We filter out all but the `missing_docs` diagnostics.
//! Parse the compiler's diagnostic output (cargo doc diagnostics).
//!
//! These either come from a pipe over STDIN or we execute cargo doc ourselves.
//! We filter out all but the `missing_docs` diagnostics.
//! Parse the compiler's diagnostic output (cargo doc diagnostics).
//!
//! These either come from a pipe over STDIN or we execute cargo doc ourselves.
//! We filter out all but the `missing_docs` diagnostics.
use crate::types::Coordinate;
use std::io::{self, BufRead};
use std::process::{Command, Stdio};

pub fn read_cargo_diagnostics() -> io::Result<Vec<Coordinate>> {
    let input: Box<dyn BufRead> = if atty::is(atty::Stream::Stdin) {
        let child = Command::new("cargo")
            .args(["doc", "--message-format=json"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;
        Box::new(io::BufReader::new(child.stdout.unwrap()))
    } else {
        Box::new(io::BufReader::new(io::stdin()))
    };

    let mut diagnostics = Vec::new();
    for line in input.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(coord) = serde_json::from_str::<Coordinate>(&line) {
            if let Some(ref msg) = coord.message {
                if msg.code.code == "missing_docs" {
                    diagnostics.push(coord);
                }
            }
        }
    }
    Ok(diagnostics)
}
