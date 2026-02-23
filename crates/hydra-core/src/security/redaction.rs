use regex::Regex;
use std::sync::LazyLock;

struct SecretPattern {
    regex: Regex,
    _label: &'static str,
}

static SECRET_PATTERNS: LazyLock<Vec<SecretPattern>> = LazyLock::new(|| {
    vec![
        SecretPattern {
            regex: Regex::new(r"sk-[a-zA-Z0-9]{20,}").unwrap(),
            _label: "OpenAI/Anthropic API key",
        },
        SecretPattern {
            regex: Regex::new(
                r"(?i)(ANTHROPIC_API_KEY|OPENAI_API_KEY|API_KEY|SECRET_KEY)\s*=\s*\S+",
            )
            .unwrap(),
            _label: "env var secret assignment",
        },
        SecretPattern {
            regex: Regex::new(r"ghp_[a-zA-Z0-9]{36}").unwrap(),
            _label: "GitHub personal access token",
        },
        SecretPattern {
            regex: Regex::new(r"Bearer\s+[a-zA-Z0-9._\-]{10,}").unwrap(),
            _label: "Bearer token",
        },
        SecretPattern {
            regex: Regex::new(r"(?i)password\s*=\s*\S+").unwrap(),
            _label: "password assignment",
        },
    ]
});

/// Replace all detected secrets in `text` with `[REDACTED]`.
pub fn redact(text: &str) -> String {
    let mut result = text.to_string();
    for pattern in SECRET_PATTERNS.iter() {
        result = pattern
            .regex
            .replace_all(&result, "[REDACTED]")
            .into_owned();
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_openai_key() {
        let input = "Using key sk-abc123def456ghi789jkl012mno345";
        let output = redact(input);
        assert_eq!(output, "Using key [REDACTED]");
        assert!(!output.contains("sk-"));
    }

    #[test]
    fn redacts_anthropic_env_var() {
        let input = "ANTHROPIC_API_KEY=sk-ant-some-long-key-value-here";
        let output = redact(input);
        assert_eq!(output, "[REDACTED]");
    }

    #[test]
    fn redacts_openai_env_var() {
        let input = "export OPENAI_API_KEY=sk-abc123def456ghi789jkl";
        let output = redact(input);
        assert!(output.contains("[REDACTED]"));
        assert!(!output.contains("OPENAI_API_KEY="));
    }

    #[test]
    fn redacts_github_pat() {
        let input = "token: ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghij";
        let output = redact(input);
        assert_eq!(output, "token: [REDACTED]");
        assert!(!output.contains("ghp_"));
    }

    #[test]
    fn redacts_bearer_token() {
        let input = "Authorization: Bearer eyJhbGciOiJIUzI1NiJ9.payload.signature";
        let output = redact(input);
        assert!(output.contains("[REDACTED]"));
        assert!(!output.contains("eyJhbGci"));
    }

    #[test]
    fn redacts_password_assignment() {
        let input = "db password = s3cret!value";
        let output = redact(input);
        assert!(output.contains("[REDACTED]"));
        assert!(!output.contains("s3cret"));
    }

    #[test]
    fn leaves_clean_text_unchanged() {
        let input = "Hello world, this is normal text with no secrets.";
        let output = redact(input);
        assert_eq!(output, input);
    }

    #[test]
    fn redacts_multiple_secrets_in_one_string() {
        let input =
            "key=sk-abcdefghijklmnopqrstuvwxyz and ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghij";
        let output = redact(input);
        assert!(!output.contains("sk-"));
        assert!(!output.contains("ghp_"));
    }

    #[test]
    fn case_insensitive_password() {
        let input = "PASSWORD = hunter2";
        let output = redact(input);
        assert!(output.contains("[REDACTED]"));
        assert!(!output.contains("hunter2"));
    }
}
