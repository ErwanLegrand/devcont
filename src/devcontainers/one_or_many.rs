use serde::Deserialize;
use serde::de::{self, Deserializer, SeqAccess, Visitor};
use std::fmt;

/// A lifecycle hook value that accepts either a plain string or an array of strings.
///
/// String form: `"npm install"` → executed as `sh -c "npm install"`
/// Array form:  `["npm", "install"]` → executed as `npm install` (no shell)
#[derive(Debug, Clone, PartialEq)]
pub enum OneOrMany {
    One(String),
    Many(Vec<String>),
}

impl OneOrMany {
    /// Parse a JSON5 string into a `OneOrMany` value.
    ///
    /// This is a thin wrapper over `json_five::from_str` exposed for fuzz harnesses
    /// that need to avoid a direct dependency on the `json_five` crate.
    ///
    /// # Errors
    /// Returns an error if `s` is not valid JSON5 or does not deserialise as
    /// a string or array-of-strings.
    ///
    /// # Examples
    /// ```
    /// use devcont::devcontainers::one_or_many::OneOrMany;
    /// let h = OneOrMany::parse_str(r#""npm install""#).unwrap();
    /// assert!(matches!(h, OneOrMany::One(_)));
    /// let h = OneOrMany::parse_str(r#"["npm", "install"]"#).unwrap();
    /// assert!(matches!(h, OneOrMany::Many(_)));
    /// ```
    pub fn parse_str(s: &str) -> Result<Self, json_five::de::SerdeJSON5Error> {
        json_five::from_str(s)
    }

    /// Return the program and arguments for executing this hook.
    ///
    /// - `One(cmd)` → `("sh", ["-c", cmd])`
    /// - `Many(parts)` → `(parts[0], parts[1..])`
    ///
    /// Returns `None` if `Many` is empty.
    #[must_use]
    pub fn to_exec_parts(&self) -> Option<(String, Vec<String>)> {
        match self {
            Self::One(cmd) => Some(("sh".to_string(), vec!["-c".to_string(), cmd.clone()])),
            Self::Many(parts) if parts.is_empty() => None,
            Self::Many(parts) => Some((parts[0].clone(), parts[1..].to_vec())),
        }
    }
}

impl fmt::Display for OneOrMany {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::One(cmd) => f.write_str(cmd),
            Self::Many(parts) => {
                let joined = parts.join(" ");
                f.write_str(&joined)
            }
        }
    }
}

struct OneOrManyVisitor;

impl<'de> Visitor<'de> for OneOrManyVisitor {
    type Value = OneOrMany;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a string or an array of strings")
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<OneOrMany, E> {
        Ok(OneOrMany::One(value.to_string()))
    }

    fn visit_string<E: de::Error>(self, value: String) -> Result<OneOrMany, E> {
        Ok(OneOrMany::One(value))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<OneOrMany, A::Error> {
        let mut parts = Vec::new();
        while let Some(item) = seq.next_element::<String>()? {
            parts.push(item);
        }
        Ok(OneOrMany::Many(parts))
    }
}

impl<'de> Deserialize<'de> for OneOrMany {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(OneOrManyVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_string_form() {
        let json = r#""npm install""#;
        let v: OneOrMany = json_five::from_str(json).expect("parse string form");
        assert_eq!(v, OneOrMany::One("npm install".to_string()));
    }

    #[test]
    fn deserialize_array_form() {
        let json = r#"["npm", "install"]"#;
        let v: OneOrMany = json_five::from_str(json).expect("parse array form");
        assert_eq!(
            v,
            OneOrMany::Many(vec!["npm".to_string(), "install".to_string()])
        );
    }

    #[test]
    fn to_exec_parts_string() {
        let v = OneOrMany::One("echo hello".to_string());
        let (prog, args) = v.to_exec_parts().unwrap();
        assert_eq!(prog, "sh");
        assert_eq!(args, vec!["-c", "echo hello"]);
    }

    #[test]
    fn to_exec_parts_array() {
        let v = OneOrMany::Many(vec!["npm".to_string(), "install".to_string()]);
        let (prog, args) = v.to_exec_parts().unwrap();
        assert_eq!(prog, "npm");
        assert_eq!(args, vec!["install"]);
    }

    #[test]
    fn to_exec_parts_empty_array() {
        let v = OneOrMany::Many(vec![]);
        assert!(v.to_exec_parts().is_none());
    }

    // --- Edge-case deserialization ---

    #[test]
    fn deserialize_empty_string_is_one() {
        let v: OneOrMany = json_five::from_str(r#""""#).expect("empty string should parse");
        assert_eq!(v, OneOrMany::One(String::new()));
    }

    #[test]
    fn deserialize_empty_array_is_many_empty() {
        let v: OneOrMany = json_five::from_str("[]").expect("empty array should parse");
        assert_eq!(v, OneOrMany::Many(vec![]));
    }

    #[test]
    fn deserialize_null_fails() {
        let result: Result<OneOrMany, _> = json_five::from_str("null");
        assert!(
            result.is_err(),
            "null should fail to deserialize as OneOrMany"
        );
    }

    #[test]
    fn deserialize_integer_fails() {
        let result: Result<OneOrMany, _> = json_five::from_str("42");
        assert!(
            result.is_err(),
            "integer should fail to deserialize as OneOrMany"
        );
    }

    #[test]
    fn deserialize_boolean_fails() {
        let result: Result<OneOrMany, _> = json_five::from_str("true");
        assert!(
            result.is_err(),
            "boolean should fail to deserialize as OneOrMany"
        );
    }

    #[test]
    fn deserialize_nested_array_fails() {
        let result: Result<OneOrMany, _> = json_five::from_str(r#"[["a"]]"#);
        assert!(
            result.is_err(),
            "nested array should fail to deserialize as OneOrMany"
        );
    }

    #[test]
    fn to_exec_parts_one_wraps_in_sh() {
        let v = OneOrMany::One("echo hello".to_string());
        let (prog, args) = v.to_exec_parts().unwrap();
        assert_eq!(prog, "sh");
        assert_eq!(args, vec!["-c", "echo hello"]);
    }

    #[test]
    fn display_one_shows_command() {
        let v = OneOrMany::One("npm install".to_string());
        assert_eq!(v.to_string(), "npm install");
    }

    #[test]
    fn display_many_joins_with_spaces() {
        let v = OneOrMany::Many(vec!["npm".to_string(), "install".to_string()]);
        assert_eq!(v.to_string(), "npm install");
    }

    #[test]
    fn display_empty_many_is_empty_string() {
        let v = OneOrMany::Many(vec![]);
        assert_eq!(v.to_string(), "");
    }

    #[test]
    fn to_exec_parts_single_element_many_has_no_args() {
        let v = OneOrMany::Many(vec!["ls".to_string()]);
        let (prog, args) = v.to_exec_parts().unwrap();
        assert_eq!(prog, "ls");
        assert!(args.is_empty());
    }
}
