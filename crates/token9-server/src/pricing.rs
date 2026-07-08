use std::collections::HashMap;

use serde::Deserialize;
use tracing::info;

use crate::config::expand_tilde;

/// USD price per 1M tokens. JSON schema mirrors the Tokei reference so a user
/// can drop in its `pricing.json`: `{ "in", "out", "cache_read", "cache_write" }`.
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct ModelPrice {
    #[serde(rename = "in", default)]
    pub input: f64,
    #[serde(default)]
    pub out: f64,
    #[serde(default)]
    pub cache_read: f64,
    #[serde(default)]
    pub cache_write: f64,
}

#[derive(Debug, Deserialize)]
struct PricingFile {
    #[serde(default)]
    models: HashMap<String, ModelPrice>,
}

/// Conservative fallback for unknown models (Opus-level), matching Tokei's
/// "assume expensive" policy so cost is never silently under-reported.
const CONSERVATIVE: ModelPrice = ModelPrice {
    input: 15.0,
    out: 75.0,
    cache_read: 1.5,
    cache_write: 18.75,
};

/// Family-keyword fallbacks (substring match on the model name), checked when
/// there's no exact entry. Order matters: more specific keywords first.
const FAMILIES: &[(&str, ModelPrice)] = &[
    ("opus", ModelPrice { input: 15.0, out: 75.0, cache_read: 1.5, cache_write: 18.75 }),
    ("sonnet", ModelPrice { input: 3.0, out: 15.0, cache_read: 0.3, cache_write: 3.75 }),
    ("haiku", ModelPrice { input: 0.8, out: 4.0, cache_read: 0.08, cache_write: 1.0 }),
    ("gpt-4o-mini", ModelPrice { input: 0.15, out: 0.6, cache_read: 0.075, cache_write: 0.0 }),
    ("gpt", ModelPrice { input: 2.5, out: 10.0, cache_read: 1.25, cache_write: 0.0 }),
    ("deepseek", ModelPrice { input: 0.27, out: 1.1, cache_read: 0.07, cache_write: 0.0 }),
];

/// Price table: user overrides from disk, then family fallbacks, then conservative default.
#[derive(Debug, Default)]
pub struct Pricing {
    overrides: HashMap<String, ModelPrice>,
}

impl Pricing {
    /// Load overrides from `~/.Oraculo/config/token9/pricing.json` if present.
    pub fn load() -> Self {
        let path = expand_tilde("~/.Oraculo/config/token9/pricing.json");
        match std::fs::read_to_string(&path) {
            Ok(text) => match serde_json::from_str::<PricingFile>(&text) {
                Ok(pf) => {
                    info!(count = pf.models.len(), path = %path, "loaded pricing overrides");
                    Pricing { overrides: pf.models }
                }
                Err(e) => {
                    tracing::warn!(error = %e, "invalid pricing.json, using built-in prices");
                    Pricing::default()
                }
            },
            Err(_) => Pricing::default(),
        }
    }

    fn lookup(&self, model: &str) -> ModelPrice {
        if let Some(p) = self.overrides.get(model) {
            return *p;
        }
        let m = model.to_lowercase();
        for (kw, price) in FAMILIES {
            if m.contains(kw) {
                return *price;
            }
        }
        CONSERVATIVE
    }

    /// Estimated cost in USD for one bucket's token counts.
    pub fn cost(&self, model: &str, input: i64, output: i64, cache_read: i64, cache_write: i64) -> f64 {
        let p = self.lookup(model);
        (input as f64 * p.input
            + output as f64 * p.out
            + cache_read as f64 * p.cache_read
            + cache_write as f64 * p.cache_write)
            / 1_000_000.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_family_cost() {
        let p = Pricing::default();
        // 1M input @ opus $15, 1M output @ $75 = $90
        let c = p.cost("claude-opus-4-6", 1_000_000, 1_000_000, 0, 0);
        assert!((c - 90.0).abs() < 1e-9, "got {c}");
    }

    #[test]
    fn sonnet_family() {
        let p = Pricing::default();
        let c = p.cost("claude-sonnet-4-6", 1_000_000, 0, 0, 0);
        assert!((c - 3.0).abs() < 1e-9, "got {c}");
    }

    #[test]
    fn unknown_falls_back_to_conservative() {
        let p = Pricing::default();
        let c = p.cost("some-random-model", 1_000_000, 0, 0, 0);
        assert!((c - 15.0).abs() < 1e-9, "got {c}"); // conservative opus input price
    }

    #[test]
    fn cache_read_is_cheaper() {
        let p = Pricing::default();
        let full = p.cost("claude-opus-4-6", 1_000_000, 0, 0, 0);
        let cached = p.cost("claude-opus-4-6", 0, 0, 1_000_000, 0);
        assert!(cached < full);
    }
}
