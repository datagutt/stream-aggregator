//! ID generation utilities

use sha2::{Digest, Sha256};

/// Generate a unique stream ID from platform and user_id
///
/// Uses SHA256 hash to create a deterministic, collision-resistant identifier
pub fn generate_stream_id(platform: &str, user_id: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(platform.as_bytes());
    hasher.update(b":");
    hasher.update(user_id.as_bytes());
    let result = hasher.finalize();
    hex::encode(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_stream_id() {
        let id1 = generate_stream_id("twitch", "user123");
        let id2 = generate_stream_id("twitch", "user123");
        let id3 = generate_stream_id("twitch", "user456");
        let id4 = generate_stream_id("youtube", "user123");

        // Same inputs produce same ID
        assert_eq!(id1, id2);

        // Different user IDs produce different IDs
        assert_ne!(id1, id3);

        // Different platforms produce different IDs
        assert_ne!(id1, id4);

        // IDs are hex-encoded SHA256 (64 characters)
        assert_eq!(id1.len(), 64);
    }
}
