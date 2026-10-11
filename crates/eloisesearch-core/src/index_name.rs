use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::{Error, Result};

/// A validated index name.
///
/// Rules:
/// - 1 to [`IndexName::MAX_LEN`] bytes;
/// - only lowercase ASCII letters, digits, `-` and `_`;
/// - starts with a lowercase letter or a digit.
///
/// Names are used as directory names under the data directory, so the rules
/// are intentionally strict and portable across file systems.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct IndexName(String);

impl IndexName {
    /// Maximum length of an index name, in bytes.
    pub const MAX_LEN: usize = 128;

    /// Validates `name` and wraps it.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidIndexName`] when a naming rule is violated.
    pub fn new(name: impl Into<String>) -> Result<Self> {
        let name = name.into();
        let invalid = |reason| Error::InvalidIndexName {
            name: name.clone(),
            reason,
        };

        let Some(first) = name.bytes().next() else {
            return Err(invalid("must not be empty"));
        };
        if name.len() > Self::MAX_LEN {
            return Err(invalid("must be at most 128 bytes long"));
        }
        if !(first.is_ascii_lowercase() || first.is_ascii_digit()) {
            return Err(invalid("must start with a lowercase letter or a digit"));
        }
        if !name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_')
        {
            return Err(invalid(
                "may only contain lowercase letters, digits, '-' and '_'",
            ));
        }
        Ok(Self(name))
    }

    /// The name as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for IndexName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for IndexName {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        Self::new(s)
    }
}

impl TryFrom<String> for IndexName {
    type Error = Error;

    fn try_from(value: String) -> Result<Self> {
        Self::new(value)
    }
}

impl From<IndexName> for String {
    fn from(value: IndexName) -> Self {
        value.0
    }
}

impl AsRef<str> for IndexName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_valid_names() {
        for name in [
            "movies",
            "movies-2026",
            "a",
            "0day",
            "user_events",
            &"a".repeat(128),
        ] {
            assert!(IndexName::new(name).is_ok(), "{name} should be valid");
        }
    }

    #[test]
    fn rejects_invalid_names() {
        let cases = [
            ("", "must not be empty"),
            ("Movies", "must start with a lowercase letter or a digit"),
            ("-movies", "must start with a lowercase letter or a digit"),
            ("_movies", "must start with a lowercase letter or a digit"),
            (
                "movies!",
                "may only contain lowercase letters, digits, '-' and '_'",
            ),
            (
                "mo vies",
                "may only contain lowercase letters, digits, '-' and '_'",
            ),
            ("../etc", "must start with a lowercase letter or a digit"),
            (
                "movi/es",
                "may only contain lowercase letters, digits, '-' and '_'",
            ),
            (
                "filmés",
                "may only contain lowercase letters, digits, '-' and '_'",
            ),
        ];
        for (name, expected) in cases {
            match IndexName::new(name) {
                Err(Error::InvalidIndexName { reason, .. }) => {
                    assert_eq!(reason, expected, "unexpected reason for {name:?}");
                }
                other => panic!("{name:?} should be rejected, got {other:?}"),
            }
        }
        assert!(IndexName::new("a".repeat(129)).is_err());
    }

    #[test]
    fn serde_round_trip_validates() {
        let name: IndexName = serde_json::from_str("\"movies\"").unwrap();
        assert_eq!(serde_json::to_string(&name).unwrap(), "\"movies\"");
        assert!(serde_json::from_str::<IndexName>("\"Bad Name\"").is_err());
    }
}
