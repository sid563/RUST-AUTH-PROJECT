//! Shared field validators reused across domains.

/// Minimal email shape check: non-empty, single `@`, with text on both sides.
pub fn is_valid_email(email: &str) -> bool {
    let mut parts = email.split('@');
    match (parts.next(), parts.next(), parts.next()) {
        (Some(local), Some(domain), None) => !local.is_empty() && domain.contains('.'),
        _ => false,
    }
}
