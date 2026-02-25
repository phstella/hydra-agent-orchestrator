use thiserror::Error;

pub const MAX_AGENT_KEY_LEN: usize = 64;
pub const MAX_BRANCH_NAME_LEN: usize = 255;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RefNameError {
    #[error("value cannot be empty")]
    Empty,

    #[error("value exceeds maximum length of {max} characters")]
    TooLong { max: usize },

    #[error("contains unsupported character '{ch}'")]
    InvalidChar { ch: char },

    #[error("cannot start with '/'")]
    StartsWithSlash,

    #[error("cannot end with '/'")]
    EndsWithSlash,

    #[error("cannot contain consecutive '/'")]
    RepeatedSlash,

    #[error("cannot contain '..'")]
    ParentTraversal,

    #[error("cannot contain '@{{'")]
    ReflogSyntax,

    #[error("cannot end with '.lock'")]
    LockSuffix,

    #[error("path segment cannot start with '.'")]
    HiddenSegment,

    #[error("path segment cannot end with '.'")]
    TrailingDotSegment,
}

pub fn validate_agent_key(agent_key: &str) -> Result<(), RefNameError> {
    if agent_key.is_empty() {
        return Err(RefNameError::Empty);
    }
    if agent_key.len() > MAX_AGENT_KEY_LEN {
        return Err(RefNameError::TooLong {
            max: MAX_AGENT_KEY_LEN,
        });
    }

    for ch in agent_key.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
            continue;
        }
        return Err(RefNameError::InvalidChar { ch });
    }

    Ok(())
}

pub fn validate_branch_name(branch: &str) -> Result<(), RefNameError> {
    if branch.is_empty() {
        return Err(RefNameError::Empty);
    }
    if branch.len() > MAX_BRANCH_NAME_LEN {
        return Err(RefNameError::TooLong {
            max: MAX_BRANCH_NAME_LEN,
        });
    }
    if branch.starts_with('/') {
        return Err(RefNameError::StartsWithSlash);
    }
    if branch.ends_with('/') {
        return Err(RefNameError::EndsWithSlash);
    }
    if branch.contains("//") {
        return Err(RefNameError::RepeatedSlash);
    }
    if branch.contains("..") {
        return Err(RefNameError::ParentTraversal);
    }
    if branch.contains("@{") {
        return Err(RefNameError::ReflogSyntax);
    }
    if branch.ends_with(".lock") {
        return Err(RefNameError::LockSuffix);
    }

    for ch in branch.chars() {
        if ch.is_ascii_alphanumeric() || ch == '/' || ch == '-' || ch == '_' || ch == '.' {
            continue;
        }
        return Err(RefNameError::InvalidChar { ch });
    }

    for segment in branch.split('/') {
        if segment.starts_with('.') {
            return Err(RefNameError::HiddenSegment);
        }
        if segment.ends_with('.') {
            return Err(RefNameError::TrailingDotSegment);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_key_accepts_expected_values() {
        assert!(validate_agent_key("claude").is_ok());
        assert!(validate_agent_key("codex_1").is_ok());
        assert!(validate_agent_key("agent-A").is_ok());
    }

    #[test]
    fn agent_key_rejects_invalid_values() {
        assert_eq!(validate_agent_key(""), Err(RefNameError::Empty));
        assert!(matches!(
            validate_agent_key("bad key"),
            Err(RefNameError::InvalidChar { ch: ' ' })
        ));
        assert!(matches!(
            validate_agent_key("../oops"),
            Err(RefNameError::InvalidChar { ch: '.' })
        ));
    }

    #[test]
    fn branch_name_accepts_expected_values() {
        assert!(validate_branch_name("hydra/run/agent/claude").is_ok());
        assert!(validate_branch_name("feature/harden-timeouts").is_ok());
        assert!(validate_branch_name("release/v1.2.3").is_ok());
    }

    #[test]
    fn branch_name_rejects_ref_unsafe_patterns() {
        assert_eq!(validate_branch_name(""), Err(RefNameError::Empty));
        assert_eq!(
            validate_branch_name("/leading/slash"),
            Err(RefNameError::StartsWithSlash)
        );
        assert_eq!(
            validate_branch_name("trailing/slash/"),
            Err(RefNameError::EndsWithSlash)
        );
        assert_eq!(
            validate_branch_name("nested//slash"),
            Err(RefNameError::RepeatedSlash)
        );
        assert_eq!(
            validate_branch_name("bad/../path"),
            Err(RefNameError::ParentTraversal)
        );
        assert_eq!(
            validate_branch_name("heads/main@{1}"),
            Err(RefNameError::ReflogSyntax)
        );
        assert_eq!(
            validate_branch_name("refs/main.lock"),
            Err(RefNameError::LockSuffix)
        );
        assert_eq!(
            validate_branch_name("refs/.hidden"),
            Err(RefNameError::HiddenSegment)
        );
        assert_eq!(
            validate_branch_name("refs/main."),
            Err(RefNameError::TrailingDotSegment)
        );
        assert!(matches!(
            validate_branch_name("refs/contains space"),
            Err(RefNameError::InvalidChar { ch: ' ' })
        ));
    }
}
