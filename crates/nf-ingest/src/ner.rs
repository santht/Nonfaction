/// Entity mention extraction — basic named entity recognition (NER).
///
/// Extracts mentions of people, organisations, and monetary amounts from
/// plain text using pattern matching.  This is intentionally a lightweight,
/// dependency-light implementation suitable for political-accountability text
/// rather than a full ML-based NER system.
use regex::Regex;
use std::sync::OnceLock;

// ── regex patterns (compiled once) ──────────────────────────────────────────

fn re_amount() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        // Matches: $1,234.56  $1.2 million  $500K  $2.3 billion
        Regex::new(r"\$[\d,]+(?:\.\d+)?(?:\s*(?:million|billion|trillion|thousand|[Kk]))?").unwrap()
    })
}

fn re_person_titled() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        // Title followed by one or two capitalised words
        Regex::new(
            r"(?:Mr\.|Mrs\.|Ms\.|Dr\.|Prof\.|Sen\.|Rep\.|Gov\.|Sec\.|Gen\.|Lt\.|Col\.|Sgt\.|Amb\.)\s+[A-Z][a-zÀ-ž'-]+(?:\s+[A-Z][a-zÀ-ž'-]+){0,2}",
        )
        .unwrap()
    })
}

fn re_person_bare() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        // Two or three consecutive title-cased words (no all-caps abbreviations)
        // Must not be a standalone acronym (all caps)
        Regex::new(r"\b([A-Z][a-zÀ-ž'-]+(?:\s+[A-Z][a-zÀ-ž'-]+){1,2})\b").unwrap()
    })
}

fn re_org() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        // One or more title-cased words followed by a known org suffix
        Regex::new(
            r"[A-Z][A-Za-zÀ-ž'-]*(?:\s+[A-Z][A-Za-zÀ-ž'-]*)*\s+(?:LLC|LLP|Inc\.|Corp\.|Corporation|Foundation|Committee|Agency|Institute|Association|Coalition|Council|Bureau|Department|Authority|Commission|Board|Group|Fund|Trust|PAC|Super\s+PAC)",
        )
        .unwrap()
    })
}

fn re_date() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        // Matches:
        // - January 15, 2024 / Jan. 15, 2024 / Jan 15, 2024
        // - 01/15/2024
        // - Q1 2024
        // - FY2024 / FY 2024
        Regex::new(
            r"\b(?:(?:Jan(?:uary)?|Feb(?:ruary)?|Mar(?:ch)?|Apr(?:il)?|May|Jun(?:e)?|Jul(?:y)?|Aug(?:ust)?|Sep(?:t(?:ember)?)?|Oct(?:ober)?|Nov(?:ember)?|Dec(?:ember)?)\.?\s+\d{1,2},\s+\d{4}|\d{1,2}/\d{1,2}/\d{4}|Q[1-4]\s+\d{4}|FY\s?\d{4})\b",
        )
        .unwrap()
    })
}

fn re_filing_number() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        // Matches:
        // - FEC-2024-123
        // - 24-cv-12345 / 24-cr-12345
        // - H.R. 123 / S. 123 / H.Res. 123 / S.Res. 123
        Regex::new(
            r"\b(?:FEC-\d{4}-\d{3,}|\d{2}-(?:cv|cr)-\d{3,}|(?:H\.R\.|S\.|H\.Res\.|S\.Res\.)\s*\d+)\b",
        )
        .unwrap()
    })
}

// ── public types ─────────────────────────────────────────────────────────────

/// A single entity mention extracted from text
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityMention {
    /// The literal text span as it appears in the source
    pub text: String,
    /// Classified entity kind
    pub kind: EntityKind,
    /// Byte offset of the start of the match in the source text
    pub offset: usize,
}

/// High-level entity categories
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntityKind {
    /// A person's name (with or without an honorific title)
    Person,
    /// An organisation, company, committee, or agency
    Organization,
    /// A monetary amount (e.g. "$1.2 million")
    MonetaryAmount,
    /// A date mention (e.g. "January 15, 2024", "FY2024")
    Date,
    /// A filing or docket identifier (e.g. "FEC-2024-123")
    FilingNumber,
}

/// All entities extracted from a single document
#[derive(Debug, Clone, Default)]
pub struct ExtractedEntities {
    pub people: Vec<EntityMention>,
    pub organizations: Vec<EntityMention>,
    pub amounts: Vec<EntityMention>,
    pub dates: Vec<EntityMention>,
    pub filing_numbers: Vec<EntityMention>,
}

impl ExtractedEntities {
    /// Total number of entity mentions across all categories
    pub fn total(&self) -> usize {
        self.people.len()
            + self.organizations.len()
            + self.amounts.len()
            + self.dates.len()
            + self.filing_numbers.len()
    }

    /// Iterate all entity mentions in document order
    pub fn all_sorted(&self) -> Vec<&EntityMention> {
        let mut all: Vec<&EntityMention> = self
            .people
            .iter()
            .chain(self.organizations.iter())
            .chain(self.amounts.iter())
            .chain(self.dates.iter())
            .chain(self.filing_numbers.iter())
            .collect();
        all.sort_by_key(|e| e.offset);
        all
    }
}

// ── extraction ───────────────────────────────────────────────────────────────

/// Extract entity mentions from plain text.
///
/// Entities are extracted in three passes:
/// 1. Monetary amounts (highest priority, non-overlapping with person pass)
/// 2. People with honorific titles
/// 3. Bare consecutive title-cased names (2-3 words)
/// 4. Organisations (suffix-anchored)
///
/// Duplicate spans are de-duplicated (same start offset kept once).
pub fn extract_entities(text: &str) -> ExtractedEntities {
    let amounts = extract_amounts(text);
    let dates = extract_dates(text);
    let filing_numbers = extract_filing_numbers(text);
    let amount_spans: Vec<(usize, usize)> = amounts
        .iter()
        .map(|e| (e.offset, e.offset + e.text.len()))
        .collect();

    let organizations = extract_organizations(text, &amount_spans);
    let org_spans: Vec<(usize, usize)> = organizations
        .iter()
        .map(|e| (e.offset, e.offset + e.text.len()))
        .collect();

    let mut blocked: Vec<(usize, usize)> = amount_spans.clone();
    blocked.extend_from_slice(&org_spans);

    let people = extract_people(text, &blocked);

    ExtractedEntities {
        people,
        organizations,
        amounts,
        dates,
        filing_numbers,
    }
}

fn extract_amounts(text: &str) -> Vec<EntityMention> {
    re_amount()
        .find_iter(text)
        .map(|m| EntityMention {
            text: m.as_str().to_string(),
            kind: EntityKind::MonetaryAmount,
            offset: m.start(),
        })
        .collect()
}

fn extract_organizations(text: &str, blocked: &[(usize, usize)]) -> Vec<EntityMention> {
    let mut seen_offsets = std::collections::HashSet::new();
    re_org()
        .find_iter(text)
        .filter_map(|m| {
            let start = m.start();
            if is_blocked(start, m.end(), blocked) {
                return None;
            }
            if seen_offsets.contains(&start) {
                return None;
            }
            seen_offsets.insert(start);
            Some(EntityMention {
                text: m.as_str().trim().to_string(),
                kind: EntityKind::Organization,
                offset: start,
            })
        })
        .collect()
}

fn extract_dates(text: &str) -> Vec<EntityMention> {
    let mut seen_offsets = std::collections::HashSet::new();
    re_date()
        .find_iter(text)
        .filter_map(|m| {
            let start = m.start();
            if seen_offsets.contains(&start) {
                return None;
            }
            seen_offsets.insert(start);
            Some(EntityMention {
                text: m.as_str().trim().to_string(),
                kind: EntityKind::Date,
                offset: start,
            })
        })
        .collect()
}

fn extract_filing_numbers(text: &str) -> Vec<EntityMention> {
    let mut seen_offsets = std::collections::HashSet::new();
    re_filing_number()
        .find_iter(text)
        .filter_map(|m| {
            let start = m.start();
            if seen_offsets.contains(&start) {
                return None;
            }
            seen_offsets.insert(start);
            Some(EntityMention {
                text: m.as_str().trim().to_string(),
                kind: EntityKind::FilingNumber,
                offset: start,
            })
        })
        .collect()
}

fn extract_people(text: &str, blocked: &[(usize, usize)]) -> Vec<EntityMention> {
    let mut seen_offsets = std::collections::HashSet::new();
    let mut people = Vec::new();

    // Pass 1: titled names (high confidence)
    for m in re_person_titled().find_iter(text) {
        let start = m.start();
        if is_blocked(start, m.end(), blocked) || seen_offsets.contains(&start) {
            continue;
        }
        seen_offsets.insert(start);
        people.push(EntityMention {
            text: m.as_str().trim().to_string(),
            kind: EntityKind::Person,
            offset: start,
        });
    }

    // Pass 2: bare capitalised names not yet captured
    for m in re_person_bare().find_iter(text) {
        let start = m.start();
        if is_blocked(start, m.end(), blocked) || seen_offsets.contains(&start) {
            continue;
        }
        // Skip common non-name capitalised bigrams
        let span = m.as_str().trim();
        if is_common_phrase(span) {
            continue;
        }
        seen_offsets.insert(start);
        people.push(EntityMention {
            text: span.to_string(),
            kind: EntityKind::Person,
            offset: start,
        });
    }

    people.sort_by_key(|e| e.offset);
    people
}

/// Returns `true` if the span `[start, end)` overlaps any blocked interval.
fn is_blocked(start: usize, end: usize, blocked: &[(usize, usize)]) -> bool {
    blocked.iter().any(|&(bs, be)| start < be && end > bs)
}

/// Filter out common English title-cased bigrams that are not names.
fn is_common_phrase(span: &str) -> bool {
    const STOP: &[&str] = &[
        "The United",
        "United States",
        "New York",
        "New Jersey",
        "New Mexico",
        "New Hampshire",
        "Los Angeles",
        "San Francisco",
        "Las Vegas",
        "White House",
        "House Committee",
        "Senate Committee",
        "Federal Reserve",
        "Supreme Court",
        "District Court",
        "Circuit Court",
        "January February",
        "March April",
        "May June",
        "July August",
        "September October",
        "November December",
        "Monday Tuesday",
        "Wednesday Thursday",
        "Friday Saturday",
    ];
    STOP.iter().any(|s| span.eq_ignore_ascii_case(s))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_dollar_amounts() {
        let text = "The committee donated $1,500 and $2.3 million to the campaign.";
        let entities = extract_entities(text);
        assert_eq!(entities.amounts.len(), 2);
        assert_eq!(entities.amounts[0].text, "$1,500");
        assert_eq!(entities.amounts[1].text, "$2.3 million");
        assert!(
            entities
                .amounts
                .iter()
                .all(|e| e.kind == EntityKind::MonetaryAmount)
        );
    }

    #[test]
    fn test_extract_amount_billion() {
        let text = "The budget is $1.5 billion.";
        let entities = extract_entities(text);
        assert_eq!(entities.amounts.len(), 1);
        assert_eq!(entities.amounts[0].text, "$1.5 billion");
    }

    #[test]
    fn test_extract_titled_person() {
        let text = "Sen. John Smith proposed the bill. Rep. Jane Doe opposed it.";
        let entities = extract_entities(text);
        let names: Vec<&str> = entities.people.iter().map(|e| e.text.as_str()).collect();
        assert!(
            names.iter().any(|&n| n.contains("John Smith")),
            "got: {:?}",
            names
        );
        assert!(
            names.iter().any(|&n| n.contains("Jane Doe")),
            "got: {:?}",
            names
        );
    }

    #[test]
    fn test_extract_organisation() {
        let text = "Donations came from Acme Corporation and the National Education Foundation.";
        let entities = extract_entities(text);
        let orgs: Vec<&str> = entities
            .organizations
            .iter()
            .map(|e| e.text.as_str())
            .collect();
        assert!(
            orgs.iter().any(|&o| o.contains("Acme Corporation")),
            "got: {:?}",
            orgs
        );
        assert!(
            orgs.iter()
                .any(|&o| o.contains("National Education Foundation")),
            "got: {:?}",
            orgs
        );
    }

    #[test]
    fn test_extract_entities_empty_text() {
        let entities = extract_entities("");
        assert_eq!(entities.total(), 0);
    }

    #[test]
    fn test_extract_entities_no_amounts() {
        let text = "There were no monetary contributions mentioned.";
        let entities = extract_entities(text);
        assert!(entities.amounts.is_empty());
    }

    #[test]
    fn test_entity_mention_offset() {
        let text = "Paid $500 to the fund.";
        let entities = extract_entities(text);
        assert_eq!(entities.amounts.len(), 1);
        let e = &entities.amounts[0];
        assert_eq!(&text[e.offset..e.offset + e.text.len()], e.text.as_str());
    }

    #[test]
    fn test_extracted_entities_total() {
        let text = "Dr. Alice Brown received $10,000 from Beta Inc.";
        let entities = extract_entities(text);
        assert!(entities.total() >= 2); // at least the amount and the org
    }

    #[test]
    fn test_all_sorted_by_offset() {
        let text = "Sen. Bob White paid $500 to Gamma Foundation.";
        let entities = extract_entities(text);
        let sorted = entities.all_sorted();
        let offsets: Vec<usize> = sorted.iter().map(|e| e.offset).collect();
        let mut expected = offsets.clone();
        expected.sort();
        assert_eq!(offsets, expected, "entities should be in document order");
    }

    #[test]
    fn test_extract_amount_with_k_suffix() {
        let text = "A contribution of $250K was recorded.";
        let entities = extract_entities(text);
        assert!(!entities.amounts.is_empty());
        assert!(entities.amounts[0].text.contains("$250"));
    }

    #[test]
    fn test_pac_organisation() {
        let text = "The filing was submitted by Americans First PAC.";
        let entities = extract_entities(text);
        let orgs: Vec<&str> = entities
            .organizations
            .iter()
            .map(|e| e.text.as_str())
            .collect();
        assert!(orgs.iter().any(|&o| o.contains("PAC")), "got: {:?}", orgs);
    }

    #[test]
    fn test_person_kind_is_correct() {
        let text = "Rep. Maria Garcia spoke at the event.";
        let entities = extract_entities(text);
        assert!(entities.people.iter().all(|e| e.kind == EntityKind::Person));
    }

    #[test]
    fn test_extract_date_long_month_format() {
        let text = "The hearing is scheduled for January 15, 2024.";
        let entities = extract_entities(text);
        assert_eq!(entities.dates.len(), 1);
        assert_eq!(entities.dates[0].text, "January 15, 2024");
    }

    #[test]
    fn test_extract_date_short_month_with_period() {
        let text = "The memo was dated Jan. 15, 2024.";
        let entities = extract_entities(text);
        assert_eq!(entities.dates.len(), 1);
        assert_eq!(entities.dates[0].text, "Jan. 15, 2024");
    }

    #[test]
    fn test_extract_date_slash_format() {
        let text = "Submission date: 01/15/2024.";
        let entities = extract_entities(text);
        assert_eq!(entities.dates.len(), 1);
        assert_eq!(entities.dates[0].text, "01/15/2024");
    }

    #[test]
    fn test_extract_date_quarter_format() {
        let text = "Spending increased in Q1 2024.";
        let entities = extract_entities(text);
        assert_eq!(entities.dates.len(), 1);
        assert_eq!(entities.dates[0].text, "Q1 2024");
    }

    #[test]
    fn test_extract_date_fy_format_no_space() {
        let text = "The line item appears in FY2024 reporting.";
        let entities = extract_entities(text);
        assert_eq!(entities.dates.len(), 1);
        assert_eq!(entities.dates[0].text, "FY2024");
    }

    #[test]
    fn test_extract_date_fy_format_with_space() {
        let text = "The line item appears in FY 2024 reporting.";
        let entities = extract_entities(text);
        assert_eq!(entities.dates.len(), 1);
        assert_eq!(entities.dates[0].text, "FY 2024");
    }

    #[test]
    fn test_extract_multiple_dates() {
        let text = "Events occurred on Jan. 15, 2024 and 01/16/2024 in Q1 2024.";
        let entities = extract_entities(text);
        let dates: Vec<&str> = entities.dates.iter().map(|e| e.text.as_str()).collect();
        assert!(dates.contains(&"Jan. 15, 2024"));
        assert!(dates.contains(&"01/16/2024"));
        assert!(dates.contains(&"Q1 2024"));
        assert_eq!(entities.dates.len(), 3);
    }

    #[test]
    fn test_extract_fec_filing_number() {
        let text = "See filing FEC-2024-123 for details.";
        let entities = extract_entities(text);
        assert_eq!(entities.filing_numbers.len(), 1);
        assert_eq!(entities.filing_numbers[0].text, "FEC-2024-123");
    }

    #[test]
    fn test_extract_case_number_cv() {
        let text = "The action was docketed as 24-cv-12345.";
        let entities = extract_entities(text);
        assert_eq!(entities.filing_numbers.len(), 1);
        assert_eq!(entities.filing_numbers[0].text, "24-cv-12345");
    }

    #[test]
    fn test_extract_case_number_cr() {
        let text = "The criminal case is 22-cr-54321.";
        let entities = extract_entities(text);
        assert_eq!(entities.filing_numbers.len(), 1);
        assert_eq!(entities.filing_numbers[0].text, "22-cr-54321");
    }

    #[test]
    fn test_extract_congressional_bill_numbers() {
        let text = "Related measures include H.R. 123, S. 456, H.Res. 12, and S.Res. 34.";
        let entities = extract_entities(text);
        let filings: Vec<&str> = entities
            .filing_numbers
            .iter()
            .map(|e| e.text.as_str())
            .collect();
        assert!(filings.contains(&"H.R. 123"));
        assert!(filings.contains(&"S. 456"));
        assert!(filings.contains(&"H.Res. 12"));
        assert!(filings.contains(&"S.Res. 34"));
        assert_eq!(entities.filing_numbers.len(), 4);
    }

    #[test]
    fn test_extract_multiple_filing_number_types() {
        let text = "References: FEC-2023-999, 23-cv-11111, and H.R. 789.";
        let entities = extract_entities(text);
        let filings: Vec<&str> = entities
            .filing_numbers
            .iter()
            .map(|e| e.text.as_str())
            .collect();
        assert!(filings.contains(&"FEC-2023-999"));
        assert!(filings.contains(&"23-cv-11111"));
        assert!(filings.contains(&"H.R. 789"));
        assert_eq!(entities.filing_numbers.len(), 3);
    }

    #[test]
    fn test_date_kind_is_correct() {
        let text = "Deadline: January 15, 2024.";
        let entities = extract_entities(text);
        assert!(entities.dates.iter().all(|e| e.kind == EntityKind::Date));
    }

    #[test]
    fn test_filing_number_kind_is_correct() {
        let text = "Matter: 24-cr-12345.";
        let entities = extract_entities(text);
        assert!(
            entities
                .filing_numbers
                .iter()
                .all(|e| e.kind == EntityKind::FilingNumber)
        );
    }

    #[test]
    fn test_total_includes_dates_and_filing_numbers() {
        let text = "On January 15, 2024, filing FEC-2024-123 reported $500 from Acme Corporation.";
        let entities = extract_entities(text);
        assert!(entities.total() >= 4);
    }
}
