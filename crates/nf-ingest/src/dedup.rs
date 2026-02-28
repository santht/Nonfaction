/// Content-based deduplication engine using SHA-256 fingerprints.
///
/// Entities are fingerprinted by normalising their canonical fields (lowercase,
/// trim, collapse internal whitespace) and hashing them with SHA-256.  This
/// gives a stable, order-sensitive 32-byte key that can be compared without
/// storing the raw field values.
use sha2::{Digest, Sha256};
use std::collections::HashSet;

// ── public types ─────────────────────────────────────────────────────────────

/// Statistics accumulated by the deduplicator over its lifetime.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DedupStats {
    /// Total number of fingerprints presented (including duplicates).
    pub total_seen: u64,
    /// Number of fingerprints that were already in the store (duplicate).
    pub duplicates_skipped: u64,
    /// Number of unique fingerprints that passed through.
    pub unique_passed: u64,
}

/// Content-based deduplication store.
///
/// Maintains an in-memory `HashSet` of 32-byte SHA-256 fingerprints.  Two
/// entity records are considered duplicates when their normalised, hashed
/// canonical fields produce the same fingerprint.
#[derive(Debug, Default)]
pub struct IngestDeduplicator {
    seen: HashSet<[u8; 32]>,
    total_seen: u64,
    duplicates_skipped: u64,
}

impl IngestDeduplicator {
    /// Create a new, empty deduplicator.
    pub fn new() -> Self {
        Self::default()
    }

    /// Build a SHA-256 fingerprint for the given entity.
    ///
    /// The fingerprint is computed over:
    /// 1. The `entity_type` string (lowercased, trimmed).
    /// 2. Each `(key, value)` pair in `canonical_fields` in the order provided,
    ///    with each key and value individually lowercased, trimmed, and with
    ///    interior whitespace collapsed to a single space.
    ///
    /// Fields are delimited with `\x00` bytes so that adjacent concatenations
    /// cannot collide.
    pub fn fingerprint(entity_type: &str, canonical_fields: &[(&str, &str)]) -> [u8; 32] {
        let mut hasher = Sha256::new();

        // Include entity type
        let normalised_type = normalise_field(entity_type);
        hasher.update(normalised_type.as_bytes());
        hasher.update(b"\x00");

        // Include each canonical field pair
        for (key, value) in canonical_fields {
            hasher.update(normalise_field(key).as_bytes());
            hasher.update(b"\x01");
            hasher.update(normalise_field(value).as_bytes());
            hasher.update(b"\x00");
        }

        hasher.finalize().into()
    }

    /// Check whether a fingerprint has been seen before and record it.
    ///
    /// Returns `true` if the fingerprint is a duplicate (already present).
    /// Returns `false` for new fingerprints and inserts them into the store.
    pub fn is_duplicate(&mut self, fingerprint: [u8; 32]) -> bool {
        self.total_seen += 1;
        if self.seen.insert(fingerprint) {
            false
        } else {
            self.duplicates_skipped += 1;
            true
        }
    }

    /// Return current deduplication statistics.
    pub fn stats(&self) -> DedupStats {
        DedupStats {
            total_seen: self.total_seen,
            duplicates_skipped: self.duplicates_skipped,
            unique_passed: self.total_seen - self.duplicates_skipped,
        }
    }

    /// Number of unique fingerprints stored.
    pub fn len(&self) -> usize {
        self.seen.len()
    }

    /// Returns `true` if no fingerprints have been stored yet.
    pub fn is_empty(&self) -> bool {
        self.seen.is_empty()
    }

    /// Clear all stored fingerprints and reset statistics.
    pub fn clear(&mut self) {
        self.seen.clear();
        self.total_seen = 0;
        self.duplicates_skipped = 0;
    }
}

// ── internal helpers ──────────────────────────────────────────────────────────

/// Normalise a single field value: lowercase, trim surrounding whitespace,
/// and collapse runs of internal whitespace to a single ASCII space.
fn normalise_field(s: &str) -> String {
    let lowered = s.to_lowercase();
    // Collapse interior whitespace
    lowered.split_whitespace().collect::<Vec<&str>>().join(" ")
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── fingerprint tests ────────────────────────────────────────────────────

    #[test]
    fn test_fingerprint_is_deterministic() {
        let fp1 = IngestDeduplicator::fingerprint("person", &[("name", "John Smith")]);
        let fp2 = IngestDeduplicator::fingerprint("person", &[("name", "John Smith")]);
        assert_eq!(fp1, fp2);
    }

    #[test]
    fn test_fingerprint_normalises_case() {
        let fp1 = IngestDeduplicator::fingerprint("PERSON", &[("NAME", "JOHN SMITH")]);
        let fp2 = IngestDeduplicator::fingerprint("person", &[("name", "john smith")]);
        assert_eq!(
            fp1, fp2,
            "case differences should produce the same fingerprint"
        );
    }

    #[test]
    fn test_fingerprint_normalises_whitespace() {
        let fp1 = IngestDeduplicator::fingerprint("person", &[("name", "  John   Smith  ")]);
        let fp2 = IngestDeduplicator::fingerprint("person", &[("name", "john smith")]);
        assert_eq!(fp1, fp2, "extra whitespace should be collapsed");
    }

    #[test]
    fn test_fingerprint_different_entity_types_differ() {
        let fp1 = IngestDeduplicator::fingerprint("person", &[("name", "John Smith")]);
        let fp2 = IngestDeduplicator::fingerprint("organisation", &[("name", "John Smith")]);
        assert_ne!(fp1, fp2);
    }

    #[test]
    fn test_fingerprint_different_values_differ() {
        let fp1 = IngestDeduplicator::fingerprint("person", &[("name", "John Smith")]);
        let fp2 = IngestDeduplicator::fingerprint("person", &[("name", "Jane Doe")]);
        assert_ne!(fp1, fp2);
    }

    #[test]
    fn test_fingerprint_multiple_fields_ordered() {
        let fp1 = IngestDeduplicator::fingerprint(
            "contribution",
            &[
                ("donor", "alice"),
                ("amount", "500"),
                ("date", "2024-01-15"),
            ],
        );
        // Different order → different fingerprint
        let fp2 = IngestDeduplicator::fingerprint(
            "contribution",
            &[
                ("amount", "500"),
                ("donor", "alice"),
                ("date", "2024-01-15"),
            ],
        );
        assert_ne!(fp1, fp2, "field order should matter for the fingerprint");
    }

    #[test]
    fn test_fingerprint_empty_entity_type() {
        // Should not panic
        let fp = IngestDeduplicator::fingerprint("", &[("name", "Alice")]);
        // Just check it's 32 bytes (it always is for SHA-256)
        assert_eq!(fp.len(), 32);
    }

    #[test]
    fn test_fingerprint_empty_fields_slice() {
        let fp1 = IngestDeduplicator::fingerprint("person", &[]);
        let fp2 = IngestDeduplicator::fingerprint("person", &[]);
        assert_eq!(fp1, fp2);
    }

    #[test]
    fn test_fingerprint_empty_value() {
        let fp1 = IngestDeduplicator::fingerprint("fec_donor", &[("name", "")]);
        let fp2 = IngestDeduplicator::fingerprint("fec_donor", &[("name", "alice")]);
        assert_ne!(fp1, fp2);
    }

    #[test]
    fn test_fingerprint_tabs_and_newlines_collapsed() {
        let fp1 = IngestDeduplicator::fingerprint("person", &[("name", "john\t \nsmith")]);
        let fp2 = IngestDeduplicator::fingerprint("person", &[("name", "john smith")]);
        assert_eq!(fp1, fp2, "tabs and newlines should collapse to spaces");
    }

    #[test]
    fn test_fingerprint_unicode_fields() {
        // Non-ASCII characters should be preserved after lowercasing
        let fp1 = IngestDeduplicator::fingerprint("person", &[("name", "José García")]);
        let fp2 = IngestDeduplicator::fingerprint("person", &[("name", "JOSÉ GARCÍA")]);
        assert_eq!(fp1, fp2, "unicode lowercase should match");
    }

    // ── is_duplicate tests ───────────────────────────────────────────────────

    #[test]
    fn test_new_deduplicator_is_empty() {
        let d = IngestDeduplicator::new();
        assert!(d.is_empty());
        assert_eq!(d.len(), 0);
    }

    #[test]
    fn test_first_record_is_not_duplicate() {
        let mut d = IngestDeduplicator::new();
        let fp = IngestDeduplicator::fingerprint("person", &[("name", "Alice")]);
        assert!(!d.is_duplicate(fp));
    }

    #[test]
    fn test_second_same_fingerprint_is_duplicate() {
        let mut d = IngestDeduplicator::new();
        let fp = IngestDeduplicator::fingerprint("person", &[("name", "Alice")]);
        d.is_duplicate(fp);
        assert!(d.is_duplicate(fp));
    }

    #[test]
    fn test_different_fingerprints_not_duplicate() {
        let mut d = IngestDeduplicator::new();
        let fp1 = IngestDeduplicator::fingerprint("person", &[("name", "Alice")]);
        let fp2 = IngestDeduplicator::fingerprint("person", &[("name", "Bob")]);
        assert!(!d.is_duplicate(fp1));
        assert!(!d.is_duplicate(fp2));
        assert_eq!(d.len(), 2);
    }

    #[test]
    fn test_is_duplicate_inserts_into_store() {
        let mut d = IngestDeduplicator::new();
        let fp = IngestDeduplicator::fingerprint("fec", &[("filing", "123")]);
        assert!(!d.is_duplicate(fp));
        assert_eq!(d.len(), 1);
    }

    #[test]
    fn test_clear_resets_store_and_stats() {
        let mut d = IngestDeduplicator::new();
        let fp = IngestDeduplicator::fingerprint("person", &[("name", "Alice")]);
        d.is_duplicate(fp);
        d.clear();
        assert!(d.is_empty());
        assert_eq!(d.stats(), DedupStats::default());
        // After clearing, the same fingerprint is no longer a duplicate
        assert!(!d.is_duplicate(fp));
    }

    // ── stats tests ──────────────────────────────────────────────────────────

    #[test]
    fn test_stats_initial_zeroes() {
        let d = IngestDeduplicator::new();
        let s = d.stats();
        assert_eq!(s.total_seen, 0);
        assert_eq!(s.duplicates_skipped, 0);
        assert_eq!(s.unique_passed, 0);
    }

    #[test]
    fn test_stats_after_unique_records() {
        let mut d = IngestDeduplicator::new();
        for i in 0u8..5 {
            let fp = IngestDeduplicator::fingerprint("record", &[("id", &i.to_string())]);
            d.is_duplicate(fp);
        }
        let s = d.stats();
        assert_eq!(s.total_seen, 5);
        assert_eq!(s.duplicates_skipped, 0);
        assert_eq!(s.unique_passed, 5);
    }

    #[test]
    fn test_stats_after_duplicates() {
        let mut d = IngestDeduplicator::new();
        let fp = IngestDeduplicator::fingerprint("person", &[("name", "Alice")]);
        // First seen: unique
        d.is_duplicate(fp);
        // Three duplicates
        d.is_duplicate(fp);
        d.is_duplicate(fp);
        d.is_duplicate(fp);
        let s = d.stats();
        assert_eq!(s.total_seen, 4);
        assert_eq!(s.duplicates_skipped, 3);
        assert_eq!(s.unique_passed, 1);
    }

    #[test]
    fn test_stats_unique_passed_equals_len() {
        let mut d = IngestDeduplicator::new();
        let fp1 = IngestDeduplicator::fingerprint("a", &[("x", "1")]);
        let fp2 = IngestDeduplicator::fingerprint("b", &[("x", "2")]);
        let fp3 = IngestDeduplicator::fingerprint("c", &[("x", "3")]);
        d.is_duplicate(fp1);
        d.is_duplicate(fp1); // dup
        d.is_duplicate(fp2);
        d.is_duplicate(fp3);
        let s = d.stats();
        assert_eq!(s.unique_passed as usize, d.len());
    }

    #[test]
    fn test_stats_total_seen_invariant() {
        let mut d = IngestDeduplicator::new();
        let fp = IngestDeduplicator::fingerprint("x", &[]);
        for _ in 0..100 {
            d.is_duplicate(fp);
        }
        let s = d.stats();
        assert_eq!(s.total_seen, 100);
        assert_eq!(s.duplicates_skipped + s.unique_passed, s.total_seen);
    }

    // ── integration: fingerprint → is_duplicate end-to-end ──────────────────

    #[test]
    fn test_fec_donor_deduplication() {
        let mut d = IngestDeduplicator::new();
        // Same donor, different casing/spacing in the source data
        let fp1 = IngestDeduplicator::fingerprint(
            "fec_individual_donor",
            &[
                ("name", "  SMITH, JOHN  "),
                ("employer", "Acme Corp"),
                ("zip", "90210"),
            ],
        );
        let fp2 = IngestDeduplicator::fingerprint(
            "fec_individual_donor",
            &[
                ("name", "smith, john"),
                ("employer", "acme corp"),
                ("zip", "90210"),
            ],
        );
        assert_eq!(
            fp1, fp2,
            "normalised donor records should have equal fingerprints"
        );
        assert!(!d.is_duplicate(fp1));
        assert!(d.is_duplicate(fp2));
    }

    #[test]
    fn test_lobbying_disclosure_deduplication() {
        let mut d = IngestDeduplicator::new();
        let fp = IngestDeduplicator::fingerprint(
            "lobbying_registration",
            &[
                ("registrant", "Apex Strategies LLC"),
                ("client", "Big Oil Inc"),
                ("period", "2024-Q1"),
            ],
        );
        assert!(!d.is_duplicate(fp));
        assert!(d.is_duplicate(fp));
        assert_eq!(d.stats().duplicates_skipped, 1);
    }
}
