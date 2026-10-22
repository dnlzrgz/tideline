use url::Url;

use crate::repo::error::RepoError;

/// Trims and validates a feed URL, dropping any fragment. Only `http` and `https` URLs are accepted.
pub fn normalize_url(input: &str) -> Result<String, RepoError> {
    let input = input.trim();
    let invalid = || RepoError::InvalidUrl(input.to_owned());
    let mut url = Url::parse(input).map_err(|_| invalid())?;

    if !matches!(url.scheme(), "http" | "https") {
        return Err(invalid());
    }

    url.set_fragment(None);
    Ok(url.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_url_removes_fragment() {
        let normalized =
            normalize_url("https://example.com/feed#section").expect("url should be valid");
        assert_eq!(normalized, "https://example.com/feed");
    }

    #[test]
    fn normalize_url_keeps_query() {
        let normalized =
            normalize_url("https://example.com/feed?tag=rust").expect("url should be valid");
        assert_eq!(normalized, "https://example.com/feed?tag=rust");
    }

    #[test]
    fn normalize_url_accepts_http() {
        let normalized = normalize_url("http://example.com/feed").expect("url should be valid");
        assert_eq!(normalized, "http://example.com/feed");
    }

    #[test]
    fn normalize_url_rejects_missing_scheme() {
        let error = normalize_url("example.com/feed").expect_err("url should be rejected");
        assert!(matches!(error, RepoError::InvalidUrl(_)));
    }

    #[test]
    fn normalize_url_rejects_ftp_scheme() {
        let error = normalize_url("ftp://example.com/feed").expect_err("url should be rejected");
        assert!(matches!(error, RepoError::InvalidUrl(_)));
    }

    #[test]
    fn normalize_url_rejects_mailto_scheme() {
        let error =
            normalize_url("mailto:someone@example.com").expect_err("url should be rejected");
        assert!(matches!(error, RepoError::InvalidUrl(_)));
    }

    #[test]
    fn normalize_url_rejects_empty_input() {
        let error = normalize_url("").expect_err("url should be rejected");
        assert!(matches!(error, RepoError::InvalidUrl(_)));
    }

    #[test]
    fn normalize_url_rejects_whitespace_only_input() {
        let error = normalize_url("   \t\n ").expect_err("url should be rejected");
        assert!(matches!(error, RepoError::InvalidUrl(_)));
    }
}
