use facet::Facet;
use std::fs;

#[derive(Facet, Clone)]
pub struct Config {
    #[facet(default = 100)]
    pub max_width: usize,
}

impl Config {
    pub fn load() -> Self {
        if let Ok(contents) = fs::read_to_string("rustfmt.toml") {
            if let Ok(config) = facet_toml::from_str::<Config>(&contents) {
                return config;
            }
        }
        facet_toml::from_str::<Config>("").unwrap()
    }
}
