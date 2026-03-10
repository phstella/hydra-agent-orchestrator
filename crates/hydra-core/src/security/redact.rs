use std::borrow::Cow;

/// Secret patterns that should be redacted from logs and artifacts.
///
/// Patterns are applied in order; first match wins per token.
static SECRET_PATTERNS: &[(&str, SecretKind)] = &[
    ("sk-ant-", SecretKind::AnthropicApiKey),
    ("sk-proj-", SecretKind::OpenAiApiKey),
    ("sk-", SecretKind::GenericApiKey),
    ("ghp_", SecretKind::GitHubPat),
    ("gho_", SecretKind::GitHubOAuth),
    ("ghs_", SecretKind::GitHubAppToken),
    ("ghu_", SecretKind::GitHubUserToken),
    ("github_pat_", SecretKind::GitHubFinePat),
    ("xoxb-", SecretKind::SlackBotToken),
    ("xoxp-", SecretKind::SlackUserToken),
    ("AKIA", SecretKind::AwsAccessKey),
    ("eyJ", SecretKind::JwtToken),
    ("npm_", SecretKind::NpmToken),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretKind {
    AnthropicApiKey,
    OpenAiApiKey,
    GenericApiKey,
    GitHubPat,
    GitHubOAuth,
    GitHubAppToken,
    GitHubUserToken,
    GitHubFinePat,
    SlackBotToken,
    SlackUserToken,
    AwsAccessKey,
    JwtToken,
    NpmToken,
}

impl SecretKind {
    pub fn label(&self) -> &'static str {
        match self {
            Self::AnthropicApiKey => "ANTHROPIC_KEY",
            Self::OpenAiApiKey => "OPENAI_KEY",
            Self::GenericApiKey => "API_KEY",
            Self::GitHubPat => "GITHUB_PAT",
            Self::GitHubOAuth => "GITHUB_OAUTH",
            Self::GitHubAppToken => "GITHUB_APP_TOKEN",
            Self::GitHubUserToken => "GITHUB_USER_TOKEN",
            Self::GitHubFinePat => "GITHUB_FINE_PAT",
            Self::SlackBotToken => "SLACK_BOT_TOKEN",
            Self::SlackUserToken => "SLACK_USER_TOKEN",
            Self::AwsAccessKey => "AWS_ACCESS_KEY",
            Self::JwtToken => "JWT_TOKEN",
            Self::NpmToken => "NPM_TOKEN",
        }
    }
}

/// Wrapper around a string that has been redacted.
#[derive(Debug, Clone)]
pub struct RedactedString {
    pub value: String,
    pub redaction_count: usize,
}

/// Redacts known secret patterns from text.
pub struct SecretRedactor {
    custom_patterns: Vec<(String, String)>,
}

impl SecretRedactor {
    pub fn new() -> Self {
        Self {
            custom_patterns: Vec::new(),
        }
    }

    /// Add a custom pattern: any occurrence of `pattern` is replaced with `label`.
    pub fn add_pattern(&mut self, pattern: String, label: String) {
        self.custom_patterns.push((pattern, label));
    }

    /// Redact secrets from a single line of text.
    pub fn redact_line<'a>(&self, input: &'a str) -> Cow<'a, str> {
        let mut output = input.to_string();
        let mut changed = false;

        for (prefix, kind) in SECRET_PATTERNS {
            let replacement = format!("[REDACTED:{}]", kind.label());
            let mut search_from = 0;

            while let Some(rel_pos) = output[search_from..].find(prefix) {
                let abs_pos = search_from + rel_pos;
                let token_end = find_token_end(&output, abs_pos);
                output.replace_range(abs_pos..token_end, &replacement);
                changed = true;
                search_from = abs_pos + replacement.len();
            }
        }

        for (pattern, label) in &self.custom_patterns {
            if output.contains(pattern.as_str()) {
                let replacement = format!("[REDACTED:{label}]");
                output = output.replace(pattern.as_str(), &replacement);
                changed = true;
            }
        }

        if changed {
            Cow::Owned(output)
        } else {
            Cow::Borrowed(input)
        }
    }

    /// Redact all lines in a multi-line string.
    pub fn redact(&self, input: &str) -> RedactedString {
        let mut count = 0;
        let output: Vec<String> = input
            .lines()
            .map(|line| {
                let redacted = self.redact_line(line);
                if let Cow::Owned(_) = &redacted {
                    count += 1;
                }
                redacted.into_owned()
            })
            .collect();

        RedactedString {
            value: output.join("\n"),
            redaction_count: count,
        }
    }
}

impl Default for SecretRedactor {
    fn default() -> Self {
        Self::new()
    }
}

/// Find the end of a token starting at `start` (token = non-whitespace run).
fn find_token_end(s: &str, start: usize) -> usize {
    s[start..]
        .find(|c: char| {
            c.is_whitespace()
                || c == '"'
                || c == '\''
                || c == ','
                || c == ';'
                || c == ')'
                || c == ']'
                || c == '}'
        })
        .map(|pos| start + pos)
        .unwrap_or(s.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_anthropic_api_key() {
        let r = SecretRedactor::new();
        let input = "key=sk-ant-abc123XYZ789-rest";
        let output = r.redact_line(input);
        assert!(!output.contains("sk-ant-"));
        assert!(output.contains("[REDACTED:ANTHROPIC_KEY]"));
    }

    #[test]
    fn redacts_openai_api_key() {
        let r = SecretRedactor::new();
        let input = "OPENAI_API_KEY=sk-proj-abcdefghijk";
        let output = r.redact_line(input);
        assert!(!output.contains("sk-proj-"));
        assert!(output.contains("[REDACTED:OPENAI_KEY]"));
    }

    #[test]
    fn redacts_github_pat() {
        let r = SecretRedactor::new();
        let input = "token: ghp_1234567890abcdef1234567890abcdef12345678";
        let output = r.redact_line(input);
        assert!(!output.contains("ghp_"));
        assert!(output.contains("[REDACTED:GITHUB_PAT]"));
    }

    #[test]
    fn redacts_aws_access_key() {
        let r = SecretRedactor::new();
        let input = "AWS_ACCESS_KEY_ID=AKIAIOSFODNN7EXAMPLE";
        let output = r.redact_line(input);
        assert!(!output.contains("AKIAIOSFODNN7EXAMPLE"));
        assert!(output.contains("[REDACTED:AWS_ACCESS_KEY]"));
    }

    #[test]
    fn redacts_jwt_token() {
        let r = SecretRedactor::new();
        let input = "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.payload.sig";
        let output = r.redact_line(input);
        assert!(!output.contains("eyJhbGci"));
        assert!(output.contains("[REDACTED:JWT_TOKEN]"));
    }

    #[test]
    fn redacts_slack_token() {
        let r = SecretRedactor::new();
        let input = "SLACK_TOKEN=xoxb-123456789-abcdefghij";
        let output = r.redact_line(input);
        assert!(!output.contains("xoxb-"));
        assert!(output.contains("[REDACTED:SLACK_BOT_TOKEN]"));
    }

    #[test]
    fn redacts_npm_token() {
        let r = SecretRedactor::new();
        let input = "//registry.npmjs.org/:_authToken=npm_abcdef1234567890";
        let output = r.redact_line(input);
        assert!(!output.contains("npm_abcdef"));
        assert!(output.contains("[REDACTED:NPM_TOKEN]"));
    }

    #[test]
    fn no_secrets_returns_unchanged() {
        let r = SecretRedactor::new();
        let input = "This is a normal log line with no secrets.";
        let output = r.redact_line(input);
        assert!(matches!(output, Cow::Borrowed(_)));
        assert_eq!(&*output, input);
    }

    #[test]
    fn multiline_redaction_counts_correctly() {
        let r = SecretRedactor::new();
        let input = "line1: ok\nline2: sk-ant-secret123\nline3: ghp_tokenabc\nline4: clean";
        let result = r.redact(input);
        assert_eq!(result.redaction_count, 2);
        assert!(!result.value.contains("sk-ant-"));
        assert!(!result.value.contains("ghp_"));
    }

    #[test]
    fn custom_pattern_redaction() {
        let mut r = SecretRedactor::new();
        r.add_pattern("my-secret-value".to_string(), "CUSTOM".to_string());
        let input = "config: my-secret-value is here";
        let output = r.redact_line(input);
        assert!(!output.contains("my-secret-value"));
        assert!(output.contains("[REDACTED:CUSTOM]"));
    }

    #[test]
    fn secret_in_json_context() {
        let r = SecretRedactor::new();
        let input = r#"{"api_key":"sk-ant-abc123","other":"value"}"#;
        let output = r.redact_line(input);
        assert!(!output.contains("sk-ant-"));
        assert!(output.contains("[REDACTED:ANTHROPIC_KEY]"));
    }

    #[test]
    fn github_fine_grained_pat() {
        let r = SecretRedactor::new();
        let input = "token=github_pat_11AABBCC_deadbeefcafebabe";
        let output = r.redact_line(input);
        assert!(!output.contains("github_pat_"));
        assert!(output.contains("[REDACTED:GITHUB_FINE_PAT]"));
    }

    #[test]
    fn redacts_multiple_occurrences_of_same_prefix() {
        let r = SecretRedactor::new();
        let input = "a=sk-ant-first b=sk-ant-second";
        let output = r.redact_line(input);
        assert_eq!(output.matches("[REDACTED:ANTHROPIC_KEY]").count(), 2);
        assert!(!output.contains("sk-ant-"));
    }

    #[test]
    fn custom_pattern_redacts_all_occurrences() {
        let mut r = SecretRedactor::new();
        r.add_pattern("abc123".to_string(), "CUSTOM".to_string());
        let input = "abc123 and again abc123";
        let output = r.redact_line(input);
        assert_eq!(output.matches("[REDACTED:CUSTOM]").count(), 2);
        assert!(!output.contains("abc123"));
    }
}
