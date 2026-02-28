/// Field normalisation utilities for political-accountability data.
///
/// Each function is pure and allocation-cheap.  They accept messy real-world
/// input (mixed case, punctuation, locale variants) and return canonical forms
/// suitable for downstream deduplication and indexing.
use chrono::NaiveDate;
use regex::Regex;
use std::sync::OnceLock;

// ── regex helpers (compiled once) ────────────────────────────────────────────

fn re_name_suffix() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        // Suffixes to strip from person names (trailing, optionally with period)
        Regex::new(
            r"(?i)\s*,?\s*\b(Jr\.?|Sr\.?|II|III|IV|V|MD|M\.D\.?|PhD|Ph\.D\.?|Esq\.?|Esq|JD|J\.D\.?|CPA|CPA\.?|DDS|D\.D\.S\.?|DO|D\.O\.?|RN|R\.N\.?)\s*$"
        )
        .unwrap()
    })
}

fn re_whitespace_run() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\s+").unwrap())
}

fn re_dollar_amount() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        // Accepts optional leading $, optional commas, optional decimal,
        // optional suffix (K/M/B) with or without a decimal multiplier.
        // Examples: $1,234.56  1234.56  $1.2M  500K  .5B  $2.3 million
        Regex::new(
            r"^\$?\s*(\d{1,3}(?:,\d{3})*(?:\.\d+)?|\d+(?:\.\d+)?|\.\d+)\s*([KkMmBbTt]|(?i:thousand|million|billion|trillion))?$"
        )
        .unwrap()
    })
}

fn re_date_iso() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^(\d{4})-(\d{1,2})-(\d{1,2})$").unwrap())
}

fn re_date_mdy_slash() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^(\d{1,2})/(\d{1,2})/(\d{4})$").unwrap())
}

fn re_date_text() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        // "January 15 2024", "January 15, 2024", "Jan 15 2024", "Jan. 15, 2024"
        Regex::new(
            r"(?i)^(January|February|March|April|May|June|July|August|September|October|November|December|Jan\.?|Feb\.?|Mar\.?|Apr\.?|Jun\.?|Jul\.?|Aug\.?|Sep\.?|Sept\.?|Oct\.?|Nov\.?|Dec\.?)\s+(\d{1,2}),?\s+(\d{4})$"
        )
        .unwrap()
    })
}

// ── person name normalisation ─────────────────────────────────────────────────

/// Normalise a person name to `First Last` Title Case.
///
/// Handles:
/// - Stripping common name suffixes: `Jr`, `Sr`, `III`, `IV`, `MD`, `PhD`, `Esq`, etc.
/// - Converting `LAST, FIRST` → `First Last`
/// - Collapsing whitespace
/// - Title-casing (each word capitalised)
/// - Empty / whitespace-only input → empty string
pub fn normalize_person_name(name: &str) -> String {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    // Collapse whitespace runs first so regex works on clean input
    let collapsed = re_whitespace_run().replace_all(trimmed, " ").into_owned();

    // Strip trailing suffixes (may be repeated, e.g. "Smith Jr., Ph.D.")
    let mut stripped = collapsed.clone();
    loop {
        let next = re_name_suffix().replace(&stripped, "").into_owned();
        if next == stripped {
            break;
        }
        stripped = next;
    }
    let stripped = stripped.trim().trim_end_matches(',').trim().to_string();

    // Detect LAST, FIRST (or LAST, FIRST MIDDLE) format
    let reordered = if let Some((last, first)) = split_last_first(&stripped) {
        format!("{} {}", first.trim(), last.trim())
    } else {
        stripped
    };

    title_case(&reordered)
}

/// If `s` is in `Last, First` format, return `(last, first)`.
/// Only splits on the first comma; ignores if there are no commas or the part
/// after the comma looks like a suffix only.
fn split_last_first(s: &str) -> Option<(&str, &str)> {
    let comma_pos = s.find(',')?;
    let last = s[..comma_pos].trim();
    let rest = s[comma_pos + 1..].trim();

    // Guard: rest must be non-empty and not look like a lone suffix
    if rest.is_empty() {
        return None;
    }
    // If last part has a space it's likely already "First Last" — don't reorder
    // Only reorder when `last` is a single word (a surname) or all-caps
    if last.contains(' ') && !last.chars().all(|c| c.is_uppercase() || !c.is_alphabetic()) {
        return None;
    }
    Some((last, rest))
}

/// Convert a string to Title Case (first letter of each whitespace-separated
/// word uppercased, remainder lowercased).
fn title_case(s: &str) -> String {
    s.split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => {
                    let upper: String = first.to_uppercase().collect();
                    let rest: String = chars.collect::<String>().to_lowercase();
                    format!("{upper}{rest}")
                }
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

// ── dollar amount normalisation ───────────────────────────────────────────────

/// Normalise a raw monetary string to an `f64` value in dollars.
///
/// Handles:
/// - Optional leading `$`
/// - Comma-separated thousands: `1,234.56` → `1234.56`
/// - Suffix multipliers: `K`/`k` = ×1 000, `M`/`m` = ×1 000 000,
///   `B`/`b` = ×1 000 000 000, `T`/`t` = ×1 000 000 000 000
/// - Long-form suffixes: `thousand`, `million`, `billion`, `trillion`
/// - Leading-decimal shorthand: `.5M` → `500 000.0`
/// - Returns `None` for empty, whitespace-only, or unparseable input.
pub fn normalize_dollar_amount(raw: &str) -> Option<f64> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    let caps = re_dollar_amount().captures(trimmed)?;
    let numeric_str = caps.get(1)?.as_str();
    // Remove embedded commas before parsing
    let clean: String = numeric_str.chars().filter(|&c| c != ',').collect();
    let base: f64 = clean.parse().ok()?;

    let multiplier = match caps.get(2).map(|m| m.as_str().to_lowercase()).as_deref() {
        Some("k") | Some("thousand") => 1_000.0,
        Some("m") | Some("million") => 1_000_000.0,
        Some("b") | Some("billion") => 1_000_000_000.0,
        Some("t") | Some("trillion") => 1_000_000_000_000.0,
        _ => 1.0,
    };

    Some(base * multiplier)
}

// ── date normalisation ────────────────────────────────────────────────────────

/// Normalise a raw date string to a `NaiveDate`.
///
/// Supported formats (case-insensitive where applicable):
/// - ISO 8601: `2024-01-15`
/// - US slash: `01/15/2024` (month/day/year)
/// - Long month: `January 15 2024` or `January 15, 2024`
/// - Short month: `Jan 15 2024`, `Jan. 15, 2024`
///
/// Returns `None` for empty, whitespace-only, or unrecognised input, and for
/// out-of-range calendar values (e.g. month 13).
pub fn normalize_date(raw: &str) -> Option<NaiveDate> {
    let trimmed = re_whitespace_run()
        .replace_all(raw.trim(), " ")
        .into_owned();
    if trimmed.is_empty() {
        return None;
    }

    // Try ISO 8601: YYYY-MM-DD
    if let Some(caps) = re_date_iso().captures(&trimmed) {
        let y: i32 = caps[1].parse().ok()?;
        let m: u32 = caps[2].parse().ok()?;
        let d: u32 = caps[3].parse().ok()?;
        return NaiveDate::from_ymd_opt(y, m, d);
    }

    // Try US slash: MM/DD/YYYY
    if let Some(caps) = re_date_mdy_slash().captures(&trimmed) {
        let m: u32 = caps[1].parse().ok()?;
        let d: u32 = caps[2].parse().ok()?;
        let y: i32 = caps[3].parse().ok()?;
        return NaiveDate::from_ymd_opt(y, m, d);
    }

    // Try text month formats
    if let Some(caps) = re_date_text().captures(&trimmed) {
        let month_str = &caps[1];
        let day: u32 = caps[2].parse().ok()?;
        let year: i32 = caps[3].parse().ok()?;
        let month = parse_month(month_str)?;
        return NaiveDate::from_ymd_opt(year, month, day);
    }

    None
}

/// Parse a month string (full or abbreviated, optionally with trailing period)
/// to a month number 1–12.
fn parse_month(s: &str) -> Option<u32> {
    let lower = s.to_lowercase();
    let base = lower.trim_end_matches('.');
    match base {
        "january" | "jan" => Some(1),
        "february" | "feb" => Some(2),
        "march" | "mar" => Some(3),
        "april" | "apr" => Some(4),
        "may" => Some(5),
        "june" | "jun" => Some(6),
        "july" | "jul" => Some(7),
        "august" | "aug" => Some(8),
        "september" | "sep" | "sept" => Some(9),
        "october" | "oct" => Some(10),
        "november" | "nov" => Some(11),
        "december" | "dec" => Some(12),
        _ => None,
    }
}

// ── state normalisation ───────────────────────────────────────────────────────

/// Normalise a US state to its 2-letter USPS abbreviation.
///
/// Accepts:
/// - Full state names (case-insensitive): `"California"` → `"CA"`
/// - Valid 2-letter codes (case-insensitive): `"ca"` → `"CA"`
/// - Washington DC: `"District of Columbia"` / `"DC"` → `"DC"`
///
/// Returns `None` for empty, whitespace-only, or unrecognised input.
pub fn normalize_state(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    // First try direct 2-letter lookup
    let upper = trimmed.to_uppercase();
    if upper.len() == 2 {
        if VALID_CODES.contains(&upper.as_str()) {
            return Some(upper);
        }
        return None;
    }

    // Try full-name lookup (case-insensitive)
    let lower = trimmed.to_lowercase();
    for &(full, abbrev) in STATE_MAP {
        if full.to_lowercase() == lower {
            return Some(abbrev.to_string());
        }
    }

    None
}

/// All valid 2-letter USPS state codes (50 states + DC).
static VALID_CODES: &[&str] = &[
    "AL", "AK", "AZ", "AR", "CA", "CO", "CT", "DE", "FL", "GA", "HI", "ID", "IL", "IN", "IA", "KS",
    "KY", "LA", "ME", "MD", "MA", "MI", "MN", "MS", "MO", "MT", "NE", "NV", "NH", "NJ", "NM", "NY",
    "NC", "ND", "OH", "OK", "OR", "PA", "RI", "SC", "SD", "TN", "TX", "UT", "VT", "VA", "WA", "WV",
    "WI", "WY", "DC",
];

/// Full name → 2-letter code mapping for all 50 states + DC.
static STATE_MAP: &[(&str, &str)] = &[
    ("Alabama", "AL"),
    ("Alaska", "AK"),
    ("Arizona", "AZ"),
    ("Arkansas", "AR"),
    ("California", "CA"),
    ("Colorado", "CO"),
    ("Connecticut", "CT"),
    ("Delaware", "DE"),
    ("Florida", "FL"),
    ("Georgia", "GA"),
    ("Hawaii", "HI"),
    ("Idaho", "ID"),
    ("Illinois", "IL"),
    ("Indiana", "IN"),
    ("Iowa", "IA"),
    ("Kansas", "KS"),
    ("Kentucky", "KY"),
    ("Louisiana", "LA"),
    ("Maine", "ME"),
    ("Maryland", "MD"),
    ("Massachusetts", "MA"),
    ("Michigan", "MI"),
    ("Minnesota", "MN"),
    ("Mississippi", "MS"),
    ("Missouri", "MO"),
    ("Montana", "MT"),
    ("Nebraska", "NE"),
    ("Nevada", "NV"),
    ("New Hampshire", "NH"),
    ("New Jersey", "NJ"),
    ("New Mexico", "NM"),
    ("New York", "NY"),
    ("North Carolina", "NC"),
    ("North Dakota", "ND"),
    ("Ohio", "OH"),
    ("Oklahoma", "OK"),
    ("Oregon", "OR"),
    ("Pennsylvania", "PA"),
    ("Rhode Island", "RI"),
    ("South Carolina", "SC"),
    ("South Dakota", "SD"),
    ("Tennessee", "TN"),
    ("Texas", "TX"),
    ("Utah", "UT"),
    ("Vermont", "VT"),
    ("Virginia", "VA"),
    ("Washington", "WA"),
    ("West Virginia", "WV"),
    ("Wisconsin", "WI"),
    ("Wyoming", "WY"),
    ("District of Columbia", "DC"),
];

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ════════════════════════════════════════════════════════════════════════
    // normalize_person_name
    // ════════════════════════════════════════════════════════════════════════

    // -- basic happy paths ---------------------------------------------------

    #[test]
    fn test_name_plain_first_last() {
        assert_eq!(normalize_person_name("john smith"), "John Smith");
    }

    #[test]
    fn test_name_all_caps() {
        assert_eq!(normalize_person_name("JOHN SMITH"), "John Smith");
    }

    #[test]
    fn test_name_mixed_case() {
        assert_eq!(normalize_person_name("jOhN SmItH"), "John Smith");
    }

    #[test]
    fn test_name_title_case_passthrough() {
        assert_eq!(normalize_person_name("John Smith"), "John Smith");
    }

    #[test]
    fn test_name_three_words() {
        assert_eq!(normalize_person_name("john adam smith"), "John Adam Smith");
    }

    // -- LAST, FIRST reordering ---------------------------------------------

    #[test]
    fn test_name_last_comma_first() {
        assert_eq!(normalize_person_name("Smith, John"), "John Smith");
    }

    #[test]
    fn test_name_last_comma_first_all_caps() {
        assert_eq!(normalize_person_name("SMITH, JOHN"), "John Smith");
    }

    #[test]
    fn test_name_last_comma_first_middle() {
        assert_eq!(normalize_person_name("Smith, John Adam"), "John Adam Smith");
    }

    // -- suffix stripping ---------------------------------------------------

    #[test]
    fn test_name_strip_jr() {
        assert_eq!(normalize_person_name("John Smith Jr"), "John Smith");
    }

    #[test]
    fn test_name_strip_jr_period() {
        assert_eq!(normalize_person_name("John Smith Jr."), "John Smith");
    }

    #[test]
    fn test_name_strip_sr() {
        assert_eq!(normalize_person_name("John Smith Sr."), "John Smith");
    }

    #[test]
    fn test_name_strip_ii() {
        assert_eq!(normalize_person_name("John Smith II"), "John Smith");
    }

    #[test]
    fn test_name_strip_iii() {
        assert_eq!(normalize_person_name("John Smith III"), "John Smith");
    }

    #[test]
    fn test_name_strip_iv() {
        assert_eq!(normalize_person_name("John Smith IV"), "John Smith");
    }

    #[test]
    fn test_name_strip_md() {
        assert_eq!(normalize_person_name("Jane Doe MD"), "Jane Doe");
    }

    #[test]
    fn test_name_strip_md_period() {
        assert_eq!(normalize_person_name("Jane Doe M.D."), "Jane Doe");
    }

    #[test]
    fn test_name_strip_phd() {
        assert_eq!(normalize_person_name("Jane Doe PhD"), "Jane Doe");
    }

    #[test]
    fn test_name_strip_phd_period() {
        assert_eq!(normalize_person_name("Jane Doe Ph.D."), "Jane Doe");
    }

    #[test]
    fn test_name_strip_esq() {
        assert_eq!(normalize_person_name("Alice Brown Esq."), "Alice Brown");
    }

    #[test]
    fn test_name_strip_esq_no_period() {
        assert_eq!(normalize_person_name("Alice Brown Esq"), "Alice Brown");
    }

    #[test]
    fn test_name_suffix_with_comma_separator() {
        assert_eq!(normalize_person_name("John Smith, Jr."), "John Smith");
    }

    #[test]
    fn test_name_suffix_lowercase() {
        assert_eq!(normalize_person_name("john smith jr."), "John Smith");
    }

    // -- whitespace handling ------------------------------------------------

    #[test]
    fn test_name_extra_whitespace() {
        assert_eq!(normalize_person_name("  John   Smith  "), "John Smith");
    }

    #[test]
    fn test_name_tabs_and_newlines() {
        assert_eq!(normalize_person_name("John\t\nSmith"), "John Smith");
    }

    // -- edge cases ---------------------------------------------------------

    #[test]
    fn test_name_empty_string() {
        assert_eq!(normalize_person_name(""), "");
    }

    #[test]
    fn test_name_whitespace_only() {
        assert_eq!(normalize_person_name("   "), "");
    }

    #[test]
    fn test_name_single_word() {
        assert_eq!(normalize_person_name("madonna"), "Madonna");
    }

    #[test]
    fn test_name_international_characters() {
        // Characters with diacritics should be preserved
        let result = normalize_person_name("JOSÉ GARCÍA");
        assert!(
            result.contains("José")
                || result.contains("jose")
                || result.to_lowercase().contains("jos")
        );
        // At minimum it should not panic and should be non-empty
        assert!(!result.is_empty());
    }

    #[test]
    fn test_name_hyphenated() {
        // Hyphenated last names
        let result = normalize_person_name("mary-jane watson");
        assert!(!result.is_empty());
    }

    // ════════════════════════════════════════════════════════════════════════
    // normalize_dollar_amount
    // ════════════════════════════════════════════════════════════════════════

    // -- basic numeric inputs -----------------------------------------------

    #[test]
    fn test_dollar_integer() {
        assert_eq!(normalize_dollar_amount("1234"), Some(1234.0));
    }

    #[test]
    fn test_dollar_decimal() {
        assert_eq!(normalize_dollar_amount("1234.56"), Some(1234.56));
    }

    #[test]
    fn test_dollar_sign_prefix() {
        assert_eq!(normalize_dollar_amount("$1234.56"), Some(1234.56));
    }

    #[test]
    fn test_dollar_comma_thousands() {
        assert_eq!(normalize_dollar_amount("1,234.56"), Some(1234.56));
    }

    #[test]
    fn test_dollar_dollar_sign_and_commas() {
        assert_eq!(normalize_dollar_amount("$1,234.56"), Some(1234.56));
    }

    #[test]
    fn test_dollar_large_with_commas() {
        assert_eq!(normalize_dollar_amount("1,000,000.00"), Some(1_000_000.0));
    }

    #[test]
    fn test_dollar_leading_decimal() {
        // .5M  → 500_000
        assert_eq!(normalize_dollar_amount(".5M"), Some(500_000.0));
    }

    // -- K/M/B/T suffix multipliers ----------------------------------------

    #[test]
    fn test_dollar_k_suffix_uppercase() {
        assert_eq!(normalize_dollar_amount("500K"), Some(500_000.0));
    }

    #[test]
    fn test_dollar_k_suffix_lowercase() {
        assert_eq!(normalize_dollar_amount("500k"), Some(500_000.0));
    }

    #[test]
    fn test_dollar_m_suffix() {
        assert_eq!(normalize_dollar_amount("1.2M"), Some(1_200_000.0));
    }

    #[test]
    fn test_dollar_m_suffix_lowercase() {
        assert_eq!(normalize_dollar_amount("2.3m"), Some(2_300_000.0));
    }

    #[test]
    fn test_dollar_b_suffix() {
        let result = normalize_dollar_amount("1.5B").unwrap();
        assert!((result - 1_500_000_000.0).abs() < 1.0);
    }

    #[test]
    fn test_dollar_t_suffix() {
        let result = normalize_dollar_amount("1T").unwrap();
        assert!((result - 1_000_000_000_000.0).abs() < 1.0);
    }

    // -- long-form multipliers ---------------------------------------------

    #[test]
    fn test_dollar_thousand_longform() {
        assert_eq!(normalize_dollar_amount("500thousand"), Some(500_000.0));
    }

    #[test]
    fn test_dollar_million_longform() {
        assert_eq!(normalize_dollar_amount("2million"), Some(2_000_000.0));
    }

    #[test]
    fn test_dollar_billion_longform() {
        let result = normalize_dollar_amount("1billion").unwrap();
        assert!((result - 1_000_000_000.0).abs() < 1.0);
    }

    #[test]
    fn test_dollar_trillion_longform() {
        let result = normalize_dollar_amount("1trillion").unwrap();
        assert!((result - 1_000_000_000_000.0).abs() < 1.0);
    }

    // -- zero and small values ---------------------------------------------

    #[test]
    fn test_dollar_zero() {
        assert_eq!(normalize_dollar_amount("0"), Some(0.0));
    }

    #[test]
    fn test_dollar_zero_with_sign() {
        assert_eq!(normalize_dollar_amount("$0.00"), Some(0.0));
    }

    // -- invalid / edge cases ----------------------------------------------

    #[test]
    fn test_dollar_empty_string() {
        assert_eq!(normalize_dollar_amount(""), None);
    }

    #[test]
    fn test_dollar_whitespace_only() {
        assert_eq!(normalize_dollar_amount("   "), None);
    }

    #[test]
    fn test_dollar_non_numeric() {
        assert_eq!(normalize_dollar_amount("abc"), None);
    }

    #[test]
    fn test_dollar_letters_only() {
        assert_eq!(normalize_dollar_amount("N/A"), None);
    }

    #[test]
    fn test_dollar_just_dollar_sign() {
        assert_eq!(normalize_dollar_amount("$"), None);
    }

    // ════════════════════════════════════════════════════════════════════════
    // normalize_date
    // ════════════════════════════════════════════════════════════════════════

    // -- ISO 8601 -----------------------------------------------------------

    #[test]
    fn test_date_iso_basic() {
        assert_eq!(
            normalize_date("2024-01-15"),
            NaiveDate::from_ymd_opt(2024, 1, 15)
        );
    }

    #[test]
    fn test_date_iso_single_digit_month_day() {
        assert_eq!(
            normalize_date("2024-1-5"),
            NaiveDate::from_ymd_opt(2024, 1, 5)
        );
    }

    #[test]
    fn test_date_iso_end_of_year() {
        assert_eq!(
            normalize_date("2023-12-31"),
            NaiveDate::from_ymd_opt(2023, 12, 31)
        );
    }

    // -- US slash (MM/DD/YYYY) ----------------------------------------------

    #[test]
    fn test_date_us_slash() {
        assert_eq!(
            normalize_date("01/15/2024"),
            NaiveDate::from_ymd_opt(2024, 1, 15)
        );
    }

    #[test]
    fn test_date_us_slash_single_digits() {
        assert_eq!(
            normalize_date("1/5/2024"),
            NaiveDate::from_ymd_opt(2024, 1, 5)
        );
    }

    // -- long text month ----------------------------------------------------

    #[test]
    fn test_date_long_month_no_comma() {
        assert_eq!(
            normalize_date("January 15 2024"),
            NaiveDate::from_ymd_opt(2024, 1, 15)
        );
    }

    #[test]
    fn test_date_long_month_with_comma() {
        assert_eq!(
            normalize_date("January 15, 2024"),
            NaiveDate::from_ymd_opt(2024, 1, 15)
        );
    }

    #[test]
    fn test_date_all_12_long_months() {
        let months = [
            ("January", 1),
            ("February", 2),
            ("March", 3),
            ("April", 4),
            ("May", 5),
            ("June", 6),
            ("July", 7),
            ("August", 8),
            ("September", 9),
            ("October", 10),
            ("November", 11),
            ("December", 12),
        ];
        for (name, num) in months {
            let raw = format!("{name} 1 2024");
            assert_eq!(
                normalize_date(&raw),
                NaiveDate::from_ymd_opt(2024, num, 1),
                "failed for {name}"
            );
        }
    }

    // -- short text month ---------------------------------------------------

    #[test]
    fn test_date_short_month_no_period() {
        assert_eq!(
            normalize_date("Jan 15 2024"),
            NaiveDate::from_ymd_opt(2024, 1, 15)
        );
    }

    #[test]
    fn test_date_short_month_with_period() {
        assert_eq!(
            normalize_date("Jan. 15, 2024"),
            NaiveDate::from_ymd_opt(2024, 1, 15)
        );
    }

    #[test]
    fn test_date_all_12_short_months() {
        let months = [
            ("Jan", 1),
            ("Feb", 2),
            ("Mar", 3),
            ("Apr", 4),
            ("May", 5),
            ("Jun", 6),
            ("Jul", 7),
            ("Aug", 8),
            ("Sep", 9),
            ("Oct", 10),
            ("Nov", 11),
            ("Dec", 12),
        ];
        for (abbr, num) in months {
            let raw = format!("{abbr} 1 2024");
            assert_eq!(
                normalize_date(&raw),
                NaiveDate::from_ymd_opt(2024, num, 1),
                "failed for {abbr}"
            );
        }
    }

    #[test]
    fn test_date_sept_abbreviation() {
        assert_eq!(
            normalize_date("Sept 15, 2024"),
            NaiveDate::from_ymd_opt(2024, 9, 15)
        );
    }

    #[test]
    fn test_date_case_insensitive_month() {
        assert_eq!(
            normalize_date("JANUARY 15, 2024"),
            NaiveDate::from_ymd_opt(2024, 1, 15)
        );
    }

    // -- invalid / edge cases -----------------------------------------------

    #[test]
    fn test_date_empty_string() {
        assert_eq!(normalize_date(""), None);
    }

    #[test]
    fn test_date_whitespace_only() {
        assert_eq!(normalize_date("   "), None);
    }

    #[test]
    fn test_date_invalid_format() {
        assert_eq!(normalize_date("not-a-date"), None);
    }

    #[test]
    fn test_date_out_of_range_month() {
        assert_eq!(normalize_date("2024-13-01"), None);
    }

    #[test]
    fn test_date_out_of_range_day() {
        assert_eq!(normalize_date("2024-01-32"), None);
    }

    #[test]
    fn test_date_feb_30_invalid() {
        assert_eq!(normalize_date("2024-02-30"), None);
    }

    #[test]
    fn test_date_partial_date_no_year() {
        assert_eq!(normalize_date("January 15"), None);
    }

    // ════════════════════════════════════════════════════════════════════════
    // normalize_state
    // ════════════════════════════════════════════════════════════════════════

    // -- all 50 states + DC by full name ------------------------------------

    #[test]
    fn test_state_all_50_full_names() {
        let cases = [
            ("Alabama", "AL"),
            ("Alaska", "AK"),
            ("Arizona", "AZ"),
            ("Arkansas", "AR"),
            ("California", "CA"),
            ("Colorado", "CO"),
            ("Connecticut", "CT"),
            ("Delaware", "DE"),
            ("Florida", "FL"),
            ("Georgia", "GA"),
            ("Hawaii", "HI"),
            ("Idaho", "ID"),
            ("Illinois", "IL"),
            ("Indiana", "IN"),
            ("Iowa", "IA"),
            ("Kansas", "KS"),
            ("Kentucky", "KY"),
            ("Louisiana", "LA"),
            ("Maine", "ME"),
            ("Maryland", "MD"),
            ("Massachusetts", "MA"),
            ("Michigan", "MI"),
            ("Minnesota", "MN"),
            ("Mississippi", "MS"),
            ("Missouri", "MO"),
            ("Montana", "MT"),
            ("Nebraska", "NE"),
            ("Nevada", "NV"),
            ("New Hampshire", "NH"),
            ("New Jersey", "NJ"),
            ("New Mexico", "NM"),
            ("New York", "NY"),
            ("North Carolina", "NC"),
            ("North Dakota", "ND"),
            ("Ohio", "OH"),
            ("Oklahoma", "OK"),
            ("Oregon", "OR"),
            ("Pennsylvania", "PA"),
            ("Rhode Island", "RI"),
            ("South Carolina", "SC"),
            ("South Dakota", "SD"),
            ("Tennessee", "TN"),
            ("Texas", "TX"),
            ("Utah", "UT"),
            ("Vermont", "VT"),
            ("Virginia", "VA"),
            ("Washington", "WA"),
            ("West Virginia", "WV"),
            ("Wisconsin", "WI"),
            ("Wyoming", "WY"),
            ("District of Columbia", "DC"),
        ];
        for (full, abbrev) in cases {
            assert_eq!(
                normalize_state(full),
                Some(abbrev.to_string()),
                "failed for {full}"
            );
        }
    }

    // -- 2-letter code passthrough -----------------------------------------

    #[test]
    fn test_state_code_passthrough_uppercase() {
        assert_eq!(normalize_state("CA"), Some("CA".to_string()));
    }

    #[test]
    fn test_state_code_passthrough_lowercase() {
        assert_eq!(normalize_state("ca"), Some("CA".to_string()));
    }

    #[test]
    fn test_state_code_passthrough_mixed_case() {
        assert_eq!(normalize_state("Ca"), Some("CA".to_string()));
    }

    #[test]
    fn test_state_all_valid_codes_passthrough() {
        for &code in VALID_CODES {
            assert_eq!(
                normalize_state(code),
                Some(code.to_string()),
                "failed for {code}"
            );
            // lowercase should also work
            let lower = code.to_lowercase();
            assert_eq!(
                normalize_state(&lower),
                Some(code.to_string()),
                "failed lowercase for {lower}"
            );
        }
    }

    // -- case insensitive full names ----------------------------------------

    #[test]
    fn test_state_full_name_lowercase() {
        assert_eq!(normalize_state("california"), Some("CA".to_string()));
    }

    #[test]
    fn test_state_full_name_uppercase() {
        assert_eq!(normalize_state("CALIFORNIA"), Some("CA".to_string()));
    }

    #[test]
    fn test_state_full_name_mixed_case() {
        assert_eq!(normalize_state("cAlIfOrNiA"), Some("CA".to_string()));
    }

    #[test]
    fn test_state_dc() {
        assert_eq!(normalize_state("DC"), Some("DC".to_string()));
        assert_eq!(
            normalize_state("District of Columbia"),
            Some("DC".to_string())
        );
        assert_eq!(
            normalize_state("district of columbia"),
            Some("DC".to_string())
        );
    }

    // -- whitespace handling -----------------------------------------------

    #[test]
    fn test_state_leading_trailing_whitespace() {
        assert_eq!(normalize_state("  California  "), Some("CA".to_string()));
    }

    #[test]
    fn test_state_code_with_whitespace() {
        assert_eq!(normalize_state("  CA  "), Some("CA".to_string()));
    }

    // -- invalid / edge cases -----------------------------------------------

    #[test]
    fn test_state_empty_string() {
        assert_eq!(normalize_state(""), None);
    }

    #[test]
    fn test_state_whitespace_only() {
        assert_eq!(normalize_state("   "), None);
    }

    #[test]
    fn test_state_invalid_two_letter_code() {
        // "ZZ" is not a valid state code
        assert_eq!(normalize_state("ZZ"), None);
    }

    #[test]
    fn test_state_invalid_full_name() {
        assert_eq!(normalize_state("Narnia"), None);
    }

    #[test]
    fn test_state_numeric_input() {
        assert_eq!(normalize_state("12"), None);
    }

    #[test]
    fn test_state_three_letter_code() {
        // 3-letter strings should not match as code
        assert_eq!(normalize_state("CAL"), None);
    }
}
