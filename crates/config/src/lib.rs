//! Environment loading and typed lookups.

use std::fmt::Display;
use std::str::FromStr;

/// Loads a `.env` file if present. Existing process variables win.
pub fn load_dotenv() {
    let _ = dotenvy::dotenv();
}

/// Reads a required variable, returning an actionable error when missing.
pub fn required(key: &str) -> anyhow::Result<String> {
    std::env::var(key)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| anyhow::anyhow!("missing required environment variable `{key}`"))
}

/// Reads an optional variable, treating empty values as absent.
pub fn optional(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

/// Reads a variable and parses it, falling back to `default`.
pub fn parse_or<T>(key: &str, default: T) -> anyhow::Result<T>
where
    T: FromStr,
    T::Err: Display,
{
    match optional(key) {
        Some(raw) => raw
            .parse::<T>()
            .map_err(|err| anyhow::anyhow!("invalid value for `{key}`: {err}")),
        None => Ok(default),
    }
}

/// Reads a comma separated variable into a list of trimmed, non-empty values.
pub fn csv(key: &str) -> Vec<String> {
    optional(key)
        .map(|raw| {
            raw.split(',')
                .map(|item| item.trim().to_string())
                .filter(|item| !item.is_empty())
                .collect()
        })
        .unwrap_or_default()
}
