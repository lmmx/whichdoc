//! Configuration to acknowledge developer preferences as well as set defaults.
//!
//! Specifically, we try to find a rustfmt.toml, and if present we load the max width from there.
//! This could be done more extensively in future, for now it serves as a placeholder.
use facet::Facet;
use std::fs;

#[derive(Facet, Clone)]
pub struct Config {
    #[facet(default = 100)]
    pub max_width: usize,
}

impl Config {
    #[must_use]
    ///
    /// # Panics
    ///
    /// Panics if the default configuration cannot be parsed.
    pub fn load() -> Self {
        if let Ok(contents) = fs::read_to_string("rustfmt.toml") {
            if let Ok(config) = facet_toml::from_str::<Config>(&contents) {
                return config;
            }
        }
        facet_toml::from_str::<Config>("").unwrap()
    }
}
