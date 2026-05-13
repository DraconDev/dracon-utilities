/// Returns true if the error indicates the remote repo already exists
/// (GitHub, GitLab, and Codeberg all use slightly different messages).
pub(crate) fn is_repo_already_exists(stderr: &str) -> bool {
    stderr.contains("Name already exists")
        || stderr.contains("already exists")
        || stderr.contains("has already been taken")
}

/// Returns true if the error indicates an auth failure (401/403/unauthorized).
pub(crate) fn is_auth_error(msg: &str) -> bool {
    msg.contains("401")
        || msg.contains("403")
        || msg.contains("unauthorized")
        || msg.contains("api key")
}

/// Returns true if the error indicates a rate limit (429).
pub(crate) fn is_rate_limited(msg: &str) -> bool {
    msg.contains("429") || msg.contains("rate limit")
}