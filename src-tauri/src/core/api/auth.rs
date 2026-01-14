//! YouTube authentication utilities

use sha1::{Digest, Sha1};
use crate::core::models::YouTubeCookies;

/// Generate SAPISIDHASH for YouTube API authentication
///
/// Format: {timestamp}_{sha1(timestamp + " " + SAPISID + " " + origin)}
pub fn generate_sapisidhash(sapisid: &str, origin: &str) -> String {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    
    let data = format!("{} {} {}", timestamp, sapisid, origin);
    let mut hasher = Sha1::new();
    hasher.update(data.as_bytes());
    let hash = hex::encode(hasher.finalize());
    
    format!("{}_{}", timestamp, hash)
}

/// Build authentication headers for YouTube API requests
pub fn build_auth_headers(cookies: &YouTubeCookies) -> Vec<(String, String)> {
    let origin = "https://www.youtube.com";
    let sapisidhash = generate_sapisidhash(&cookies.sapisid, origin);
    
    vec![
        ("Authorization".to_string(), format!("SAPISIDHASH {}", sapisidhash)),
        ("Cookie".to_string(), cookies.to_cookie_string()),
        ("X-Origin".to_string(), origin.to_string()),
        ("Origin".to_string(), origin.to_string()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_sapisidhash() {
        let hash = generate_sapisidhash("test_sapisid", "https://www.youtube.com");
        assert!(hash.contains('_'));
        let parts: Vec<&str> = hash.split('_').collect();
        assert_eq!(parts.len(), 2);
        // Timestamp should be numeric
        assert!(parts[0].parse::<u64>().is_ok());
        // Hash should be hex
        assert_eq!(parts[1].len(), 40);
    }
}
