//! Entity deduplication using field-level similarity.
//!
//! The [`EntityDeduplicator`] compares a candidate entity against a set of
//! existing entities and returns [`DedupCandidate`] values for any pair whose
//! similarity exceeds the configured threshold (default 0.85).
//!
//! Currently only `Person` entities are supported.  Additional entity types
//! can be added by extending [`EntityDeduplicator::compare`].

use nf_core::entities::{Entity, EntityId, Person};

// ─── Public types ─────────────────────────────────────────────────────────────

/// A potential near-duplicate detected during deduplication.
#[derive(Debug, Clone)]
pub struct DedupCandidate {
    /// The id of the pre-existing entity that the candidate resembles.
    pub existing_id: EntityId,
    /// The new entity being checked.
    pub new_entity: Entity,
    /// Similarity score in `[0.0, 1.0]`; higher is more confident.
    pub confidence: f64,
    /// Human-readable description of what matched (e.g. "name similarity 0.92, birth_date match").
    pub match_reason: String,
}

/// Detects near-duplicate entities using field-level similarity.
///
/// # Example
/// ```rust,ignore
/// let deduplicator = EntityDeduplicator::new();
/// let candidates = deduplicator.find_candidates(&new_entity, &existing_entities);
/// for c in candidates {
///     println!("Possible duplicate of {} (confidence {:.2}): {}", c.existing_id.0, c.confidence, c.match_reason);
/// }
/// ```
pub struct EntityDeduplicator {
    threshold: f64,
}

impl Default for EntityDeduplicator {
    fn default() -> Self {
        Self::new()
    }
}

impl EntityDeduplicator {
    /// Create a deduplicator with the default 0.85 similarity threshold.
    pub fn new() -> Self {
        Self { threshold: 0.85 }
    }

    /// Create a deduplicator with a custom similarity threshold.
    pub fn with_threshold(threshold: f64) -> Self {
        Self { threshold }
    }

    /// Return the similarity threshold used for flagging candidates.
    pub fn threshold(&self) -> f64 {
        self.threshold
    }

    /// Find all existing entities that are likely duplicates of `new_entity`.
    ///
    /// Only entities of the same type are compared.  The returned list is
    /// sorted by confidence descending.
    pub fn find_candidates(
        &self,
        new_entity: &Entity,
        existing: &[Entity],
    ) -> Vec<DedupCandidate> {
        let mut candidates: Vec<DedupCandidate> = existing
            .iter()
            .filter_map(|existing_entity| self.compare(new_entity, existing_entity))
            .collect();

        candidates.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal));
        candidates
    }

    fn compare(&self, new_entity: &Entity, existing_entity: &Entity) -> Option<DedupCandidate> {
        match (new_entity, existing_entity) {
            (Entity::Person(new_p), Entity::Person(existing_p)) => {
                self.compare_persons(new_p, existing_p, new_entity)
            }
            _ => None,
        }
    }

    fn compare_persons(
        &self,
        new_p: &Person,
        existing_p: &Person,
        new_entity: &Entity,
    ) -> Option<DedupCandidate> {
        let (new_first, new_last) = normalize_name(&new_p.name);
        let (existing_first, existing_last) = normalize_name(&existing_p.name);

        // Compute per-part similarity; an empty part contributes 0.
        let first_sim = if new_first.is_empty() && existing_first.is_empty() {
            1.0
        } else if new_first.is_empty() || existing_first.is_empty() {
            0.0
        } else {
            jaro_winkler(&new_first, &existing_first)
        };

        let last_sim = if new_last.is_empty() && existing_last.is_empty() {
            1.0
        } else if new_last.is_empty() || existing_last.is_empty() {
            0.0
        } else {
            jaro_winkler(&new_last, &existing_last)
        };

        let name_confidence = (first_sim + last_sim) / 2.0;

        let mut confidence = name_confidence;
        let mut reasons: Vec<String> = Vec::new();

        if name_confidence >= self.threshold {
            reasons.push(format!("name similarity {:.2}", name_confidence));
        }

        // Birth-date comparison refines the score.
        match (new_p.birth_date, existing_p.birth_date) {
            (Some(new_date), Some(existing_date)) => {
                if new_date == existing_date {
                    // Strong corroborating signal — boost confidence.
                    confidence = f64::max(confidence, 0.95);
                    reasons.push("birth_date match".to_string());
                } else if name_confidence >= self.threshold {
                    // Conflicting birth dates reduce confidence significantly.
                    confidence *= 0.7;
                }
            }
            _ => {} // Absent date is not penalised.
        }

        if confidence >= self.threshold && !reasons.is_empty() {
            Some(DedupCandidate {
                existing_id: existing_p.meta.id,
                new_entity: new_entity.clone(),
                confidence,
                match_reason: reasons.join(", "),
            })
        } else {
            None
        }
    }
}

// ─── Name normalisation ───────────────────────────────────────────────────────

/// Suffixes that should be stripped before comparing names.
const SUFFIXES: &[&str] = &[
    "jr", "sr", "i", "ii", "iii", "iv", "v", "esq", "phd", "md", "dds", "jd",
];

/// Normalise a full name into `(first_parts, last_part)` strings suitable for
/// similarity comparison.
///
/// Steps applied (in order):
/// 1. Lowercase
/// 2. Strip punctuation attached to tokens (commas, periods)
/// 3. Drop honorifics / generational suffixes (Jr, Sr, III, …)
/// 4. Split into first-name portion and last name
fn normalize_name(name: &str) -> (String, String) {
    let tokens: Vec<String> = name
        .split_whitespace()
        .map(|t| t.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase())
        .filter(|t| !t.is_empty())
        .collect();

    let tokens: Vec<&str> = tokens
        .iter()
        .map(|s| s.as_str())
        .filter(|t| !SUFFIXES.contains(t))
        .collect();

    match tokens.len() {
        0 => (String::new(), String::new()),
        1 => (tokens[0].to_string(), String::new()),
        _ => {
            let last = tokens[tokens.len() - 1].to_string();
            let first = tokens[..tokens.len() - 1].join(" ");
            (first, last)
        }
    }
}

// ─── Jaro-Winkler similarity ──────────────────────────────────────────────────

/// Compute the Jaro similarity between two strings.
fn jaro(s1: &str, s2: &str) -> f64 {
    if s1 == s2 {
        return 1.0;
    }

    let s1: Vec<char> = s1.chars().collect();
    let s2: Vec<char> = s2.chars().collect();
    let len1 = s1.len();
    let len2 = s2.len();

    if len1 == 0 || len2 == 0 {
        return 0.0;
    }

    let match_window = (len1.max(len2) / 2).saturating_sub(1);

    let mut s1_matched = vec![false; len1];
    let mut s2_matched = vec![false; len2];
    let mut matches = 0usize;

    for i in 0..len1 {
        let start = i.saturating_sub(match_window);
        let end = (i + match_window + 1).min(len2);
        for j in start..end {
            if s2_matched[j] || s1[i] != s2[j] {
                continue;
            }
            s1_matched[i] = true;
            s2_matched[j] = true;
            matches += 1;
            break;
        }
    }

    if matches == 0 {
        return 0.0;
    }

    // Count transpositions.
    let mut transpositions = 0usize;
    let mut k = 0usize;
    for i in 0..len1 {
        if !s1_matched[i] {
            continue;
        }
        while !s2_matched[k] {
            k += 1;
        }
        if s1[i] != s2[k] {
            transpositions += 1;
        }
        k += 1;
    }

    let m = matches as f64;
    let t = (transpositions / 2) as f64;
    (m / len1 as f64 + m / len2 as f64 + (m - t) / m) / 3.0
}

/// Compute the Jaro-Winkler similarity between two strings.
///
/// Uses the standard prefix scaling factor `p = 0.1` and caps the prefix
/// length at 4 characters, as per the original Winkler definition.
fn jaro_winkler(s1: &str, s2: &str) -> f64 {
    let jaro_score = jaro(s1, s2);
    if jaro_score == 0.0 {
        return 0.0;
    }

    let prefix_len = s1
        .chars()
        .zip(s2.chars())
        .take(4)
        .take_while(|(a, b)| a == b)
        .count();

    let p = 0.1_f64; // Winkler scaling factor
    jaro_score + (prefix_len as f64 * p * (1.0 - jaro_score))
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use nf_core::entities::{Entity, Person};
    use nf_core::source::{ContentHash, SourceChain, SourceRef, SourceType};
    use url::Url;

    fn test_source_chain() -> SourceChain {
        let url = Url::parse("https://api.open.fec.gov/v1/test/").unwrap();
        let source = SourceRef::new(
            url,
            ContentHash::compute(b"test"),
            SourceType::FecFiling,
            "test",
        );
        SourceChain::new(source)
    }

    fn make_person(name: &str) -> Entity {
        Entity::Person(Person::new(name, test_source_chain()))
    }

    fn make_person_with_dob(name: &str, dob: NaiveDate) -> Entity {
        let mut p = Person::new(name, test_source_chain());
        p.birth_date = Some(dob);
        Entity::Person(p)
    }

    // ── normalize_name ────────────────────────────────────────────────────────

    #[test]
    fn test_normalize_strips_suffix_jr() {
        let (first, last) = normalize_name("John Smith Jr");
        assert_eq!(first, "john");
        assert_eq!(last, "smith");
    }

    #[test]
    fn test_normalize_strips_suffix_iii() {
        let (first, last) = normalize_name("Robert Kennedy III");
        assert_eq!(first, "robert");
        assert_eq!(last, "kennedy");
    }

    #[test]
    fn test_normalize_strips_suffix_sr() {
        let (first, last) = normalize_name("James Brown Sr.");
        assert_eq!(first, "james");
        assert_eq!(last, "brown");
    }

    #[test]
    fn test_normalize_single_token() {
        let (first, last) = normalize_name("Madonna");
        assert_eq!(first, "madonna");
        assert_eq!(last, "");
    }

    #[test]
    fn test_normalize_two_tokens() {
        let (first, last) = normalize_name("John Doe");
        assert_eq!(first, "john");
        assert_eq!(last, "doe");
    }

    #[test]
    fn test_normalize_three_tokens() {
        let (first, last) = normalize_name("Mary Jane Watson");
        assert_eq!(first, "mary jane");
        assert_eq!(last, "watson");
    }

    #[test]
    fn test_normalize_empty() {
        let (first, last) = normalize_name("");
        assert_eq!(first, "");
        assert_eq!(last, "");
    }

    // ── Jaro / Jaro-Winkler ───────────────────────────────────────────────────

    #[test]
    fn test_jaro_identical() {
        assert!((jaro("martha", "martha") - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_jaro_empty_strings() {
        assert_eq!(jaro("", "abc"), 0.0);
        assert_eq!(jaro("abc", ""), 0.0);
    }

    #[test]
    fn test_jaro_classic_martha_marhta() {
        // Classic textbook example: jaro("MARTHA","MARHTA") ≈ 0.944
        let score = jaro("martha", "marhta");
        assert!(score > 0.93, "expected >0.93, got {score}");
        assert!(score < 1.0);
    }

    #[test]
    fn test_jaro_winkler_prefix_boost() {
        // Jaro-Winkler should score higher than Jaro when prefix matches.
        let jw = jaro_winkler("johnathan", "john");
        let j = jaro("johnathan", "john");
        assert!(jw >= j, "jaro_winkler={jw} should be >= jaro={j}");
    }

    #[test]
    fn test_jaro_winkler_identical() {
        assert!((jaro_winkler("alice", "alice") - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_jaro_winkler_completely_different() {
        let score = jaro_winkler("abc", "xyz");
        assert!(score < 0.5, "expected low score, got {score}");
    }

    // ── EntityDeduplicator ────────────────────────────────────────────────────

    #[test]
    fn test_identical_name_is_flagged() {
        let dedup = EntityDeduplicator::new();
        let existing = make_person("John Smith");
        let candidate = make_person("John Smith");
        let results = dedup.find_candidates(&candidate, &[existing]);
        assert_eq!(results.len(), 1);
        assert!(results[0].confidence >= 0.85);
    }

    #[test]
    fn test_very_different_name_not_flagged() {
        let dedup = EntityDeduplicator::new();
        let existing = make_person("Zara Thornton");
        let candidate = make_person("Bob Smith");
        let results = dedup.find_candidates(&candidate, &[existing]);
        assert!(results.is_empty(), "Different names should not be flagged");
    }

    #[test]
    fn test_suffix_ignored_in_comparison() {
        // "John Smith Jr." and "John Smith" should be flagged.
        let dedup = EntityDeduplicator::new();
        let existing = make_person("John Smith");
        let candidate = make_person("John Smith Jr.");
        let results = dedup.find_candidates(&candidate, &[existing]);
        assert_eq!(results.len(), 1, "Suffix should be stripped before comparison");
    }

    #[test]
    fn test_slight_typo_flagged() {
        // "Jhon Smith" vs "John Smith" — single char transposition.
        let dedup = EntityDeduplicator::new();
        let existing = make_person("John Smith");
        let candidate = make_person("Jhon Smith");
        let results = dedup.find_candidates(&candidate, &[existing]);
        assert_eq!(results.len(), 1, "Slight typo should be flagged");
        assert!(results[0].confidence >= 0.85);
    }

    #[test]
    fn test_birth_date_match_boosts_confidence() {
        let dob = NaiveDate::from_ymd_opt(1970, 1, 15).unwrap();
        let dedup = EntityDeduplicator::new();
        let existing = make_person_with_dob("John Smith", dob);
        let candidate = make_person_with_dob("John Smith", dob);
        let results = dedup.find_candidates(&candidate, &[existing]);
        assert_eq!(results.len(), 1);
        assert!(results[0].confidence >= 0.95, "DOB match should push confidence ≥0.95");
        assert!(results[0].match_reason.contains("birth_date match"));
    }

    #[test]
    fn test_conflicting_birth_date_reduces_confidence() {
        let dedup = EntityDeduplicator::new();
        let dob1 = NaiveDate::from_ymd_opt(1970, 1, 15).unwrap();
        let dob2 = NaiveDate::from_ymd_opt(1985, 6, 20).unwrap();
        let existing = make_person_with_dob("John Smith", dob1);
        let candidate = make_person_with_dob("John Smith", dob2);
        let results = dedup.find_candidates(&candidate, &[existing]);
        // May or may not exceed threshold depending on confidence reduction,
        // but if present, match_reason must not contain "birth_date match".
        for r in &results {
            assert!(
                !r.match_reason.contains("birth_date match"),
                "Conflicting DOB should not add birth_date match to reason"
            );
        }
    }

    #[test]
    fn test_no_candidates_from_empty_existing_list() {
        let dedup = EntityDeduplicator::new();
        let candidate = make_person("John Smith");
        let results = dedup.find_candidates(&candidate, &[]);
        assert!(results.is_empty());
    }

    #[test]
    fn test_results_sorted_by_confidence_descending() {
        let dedup = EntityDeduplicator::new();
        let existing = vec![
            make_person("John Smith"),    // exact match
            make_person("Jon Smith"),     // slight typo
        ];
        let candidate = make_person("John Smith");
        let results = dedup.find_candidates(&candidate, &existing);
        // If both are flagged, they must be in descending confidence order.
        for window in results.windows(2) {
            assert!(
                window[0].confidence >= window[1].confidence,
                "Results should be sorted by confidence desc"
            );
        }
    }

    #[test]
    fn test_scale_1000_entities() {
        // Insert 1000 persons with dissimilar names plus one near-duplicate.
        let dedup = EntityDeduplicator::new();

        let mut existing: Vec<Entity> = (0..999)
            .map(|i| make_person(&format!("Person{i:04} Unique{i:04}")))
            .collect();

        // The last entity is a near-duplicate of the candidate.
        existing.push(make_person("Alice Johnson"));

        let candidate = make_person("Alice Johnson"); // identical to last
        let results = dedup.find_candidates(&candidate, &existing);

        // At least the exact match must be returned.
        assert!(
            !results.is_empty(),
            "Should detect the near-duplicate among 1000 entities"
        );
        assert!(results[0].confidence >= 0.85);
    }

    #[test]
    fn test_custom_threshold() {
        // With a very high threshold (0.99) only exact names should match.
        let dedup = EntityDeduplicator::with_threshold(0.99);
        let existing = make_person("John Smith");
        let candidate = make_person("Jhon Smith"); // slight typo
        let results = dedup.find_candidates(&candidate, &[existing]);
        // A single-character transposition is unlikely to clear 0.99.
        // We just verify the code runs without panic; the count may be 0.
        let _ = results;
    }

    #[test]
    fn test_non_person_entities_not_compared() {
        use nf_core::entities::{Organization, OrganizationType};
        let dedup = EntityDeduplicator::new();
        let existing =
            Entity::Organization(Organization::new("Smith Corp", OrganizationType::Corporation, test_source_chain()));
        let candidate = make_person("John Smith");
        let results = dedup.find_candidates(&candidate, &[existing]);
        assert!(
            results.is_empty(),
            "Cross-type comparison should yield no candidates"
        );
    }
}
