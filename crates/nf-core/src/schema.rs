/// Schema definitions and validation rules for Nonfaction entities.
/// Extends the FollowTheMoney schema with custom types for US political accountability.
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Schema version — incremented when entity structure changes
pub const SCHEMA_VERSION: u32 = 1;

/// Entity type registry — maps type names to their validation rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaRegistry {
    pub version: u32,
    pub entity_types: HashMap<String, EntitySchema>,
    pub relationship_types: HashMap<String, RelationshipSchema>,
}

impl SchemaRegistry {
    pub fn default_registry() -> Self {
        let mut reg = Self {
            version: SCHEMA_VERSION,
            entity_types: HashMap::new(),
            relationship_types: HashMap::new(),
        };

        // Register all entity types
        reg.entity_types.insert(
            "Person".to_string(),
            EntitySchema {
                required_fields: vec!["name".to_string()],
                optional_fields: vec![
                    "aliases".to_string(),
                    "current_role".to_string(),
                    "party_affiliation".to_string(),
                    "jurisdiction".to_string(),
                    "status".to_string(),
                    "birth_date".to_string(),
                ],
                source_required: true,
            },
        );

        reg.entity_types.insert(
            "Organization".to_string(),
            EntitySchema {
                required_fields: vec!["name".to_string(), "org_type".to_string()],
                optional_fields: vec![
                    "aliases".to_string(),
                    "jurisdiction".to_string(),
                    "known_principals".to_string(),
                    "foreign_connection".to_string(),
                ],
                source_required: true,
            },
        );

        reg.entity_types.insert(
            "Payment".to_string(),
            EntitySchema {
                required_fields: vec![
                    "amount".to_string(),
                    "date".to_string(),
                    "donor".to_string(),
                    "recipient".to_string(),
                ],
                optional_fields: vec![
                    "filing_id".to_string(),
                    "election_cycle".to_string(),
                    "description".to_string(),
                ],
                source_required: true,
            },
        );

        reg.entity_types.insert(
            "TimingCorrelation".to_string(),
            EntitySchema {
                required_fields: vec![
                    "event_a".to_string(),
                    "event_b".to_string(),
                    "days_between".to_string(),
                    "correlation_type".to_string(),
                ],
                optional_fields: vec!["threshold_days".to_string()],
                source_required: true,
            },
        );

        reg.entity_types.insert(
            "ConductComparison".to_string(),
            EntitySchema {
                required_fields: vec![
                    "official_action".to_string(),
                    "official".to_string(),
                    "action_date".to_string(),
                    "action_source".to_string(),
                    "equivalent_private_conduct".to_string(),
                    "documented_consequence".to_string(),
                    "consequence_source".to_string(),
                ],
                optional_fields: vec![],
                source_required: true,
            },
        );

        reg
    }
}

/// Validate a FEC candidate ID.
///
/// Expected format: `[H|S|P]` + 1 digit (election cycle) + 2-letter state code + 5 digits (sequence)
/// Total: 9 characters. Examples: `H2NY05168`, `S6TX00194`, `P80003338`
///
/// For presidential candidates (`P`), the state code is `00` (two zeros) rather than letters.
pub fn validate_fec_candidate_id(id: &str) -> Result<(), String> {
    // Total length: 1 (type) + 1 (cycle digit) + 2 (state or "00") + 5 (sequence digits) = 9
    if id.len() != 9 {
        return Err(format!(
            "FEC candidate ID '{}' must be exactly 9 characters, got {}",
            id,
            id.len()
        ));
    }
    let bytes = id.as_bytes();

    // First character must be H, S, or P
    if !matches!(bytes[0], b'H' | b'S' | b'P') {
        return Err(format!(
            "FEC candidate ID '{}' must start with H, S, or P",
            id
        ));
    }

    // Second character must be a digit (election cycle)
    if !bytes[1].is_ascii_digit() {
        return Err(format!(
            "FEC candidate ID '{}': position 2 must be a digit (election cycle)",
            id
        ));
    }

    // Next 2 characters must be uppercase ASCII letters (state code) OR "00" for presidential
    let state_valid = if bytes[0] == b'P' {
        // Presidential candidates use "00" as the state placeholder
        (bytes[2].is_ascii_uppercase() || bytes[2] == b'0')
            && (bytes[3].is_ascii_uppercase() || bytes[3] == b'0')
    } else {
        bytes[2].is_ascii_uppercase() && bytes[3].is_ascii_uppercase()
    };
    if !state_valid {
        return Err(format!(
            "FEC candidate ID '{}': positions 3-4 must be uppercase letters (state code)",
            id
        ));
    }

    // Last 5 characters must be digits (sequence number)
    for i in 4..9 {
        if !bytes[i].is_ascii_digit() {
            return Err(format!(
                "FEC candidate ID '{}': positions 5-9 must be digits (sequence number)",
                id
            ));
        }
    }

    Ok(())
}

/// Validate a FEC committee ID.
///
/// Expected format: `C` + 8 digits
/// Examples: `C00100005`, `C00575795`
pub fn validate_fec_committee_id(id: &str) -> Result<(), String> {
    // Total length: 1 (C) + 8 (digits) = 9
    if id.len() != 9 {
        return Err(format!(
            "FEC committee ID '{}' must be exactly 9 characters, got {}",
            id,
            id.len()
        ));
    }
    let bytes = id.as_bytes();

    // First character must be 'C'
    if bytes[0] != b'C' {
        return Err(format!("FEC committee ID '{}' must start with 'C'", id));
    }

    // Remaining 8 characters must be digits
    for i in 1..9 {
        if !bytes[i].is_ascii_digit() {
            return Err(format!(
                "FEC committee ID '{}': positions 2-9 must be digits",
                id
            ));
        }
    }

    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntitySchema {
    pub required_fields: Vec<String>,
    pub optional_fields: Vec<String>,
    /// Every entity type requires sources — always true in Nonfaction
    pub source_required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipSchema {
    pub allowed_from_types: Vec<String>,
    pub allowed_to_types: Vec<String>,
    pub required_properties: Vec<String>,
    pub source_required: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_registry() {
        let reg = SchemaRegistry::default_registry();
        assert_eq!(reg.version, SCHEMA_VERSION);
        assert!(reg.entity_types.contains_key("Person"));
        assert!(reg.entity_types.contains_key("Payment"));
        assert!(reg.entity_types.contains_key("TimingCorrelation"));
    }

    #[test]
    fn test_all_types_require_sources() {
        let reg = SchemaRegistry::default_registry();
        for (name, schema) in &reg.entity_types {
            assert!(schema.source_required, "{name} must require sources");
        }
    }

    // ─── FEC Candidate ID validation ───────────────────────────────────────────

    #[test]
    fn test_validate_fec_candidate_id_valid_house() {
        assert!(validate_fec_candidate_id("H2NY05168").is_ok());
    }

    #[test]
    fn test_validate_fec_candidate_id_valid_senate() {
        assert!(validate_fec_candidate_id("S6TX00194").is_ok());
    }

    #[test]
    fn test_validate_fec_candidate_id_valid_president() {
        assert!(validate_fec_candidate_id("P80003338").is_ok());
    }

    #[test]
    fn test_validate_fec_candidate_id_wrong_length_short() {
        let err = validate_fec_candidate_id("H2NY051").unwrap_err();
        assert!(err.contains("9 characters"));
    }

    #[test]
    fn test_validate_fec_candidate_id_wrong_length_long() {
        let err = validate_fec_candidate_id("H2NY051680000").unwrap_err();
        assert!(err.contains("9 characters"));
    }

    #[test]
    fn test_validate_fec_candidate_id_invalid_prefix() {
        // 9 chars, wrong prefix letter
        let err = validate_fec_candidate_id("X2NY05168").unwrap_err();
        assert!(err.contains("H, S, or P"));
    }

    #[test]
    fn test_validate_fec_candidate_id_non_digit_in_digit_positions() {
        // position 2 should be a digit; 'X' is not
        let err = validate_fec_candidate_id("HXNY05168").unwrap_err();
        assert!(err.contains("digit"));
    }

    #[test]
    fn test_validate_fec_candidate_id_lowercase_state_code() {
        let err = validate_fec_candidate_id("H2ny05168").unwrap_err();
        assert!(err.contains("uppercase letters"));
    }

    #[test]
    fn test_validate_fec_candidate_id_non_digit_in_tail() {
        // positions 5-9 should be digits
        let err = validate_fec_candidate_id("H2NY0516X").unwrap_err();
        assert!(err.contains("digits"));
    }

    // ─── FEC Committee ID validation ───────────────────────────────────────────

    #[test]
    fn test_validate_fec_committee_id_valid() {
        assert!(validate_fec_committee_id("C00100005").is_ok());
    }

    #[test]
    fn test_validate_fec_committee_id_valid_another() {
        assert!(validate_fec_committee_id("C00575795").is_ok());
    }

    #[test]
    fn test_validate_fec_committee_id_wrong_length_short() {
        let err = validate_fec_committee_id("C001000").unwrap_err();
        assert!(err.contains("9 characters"));
    }

    #[test]
    fn test_validate_fec_committee_id_wrong_length_long() {
        let err = validate_fec_committee_id("C0010000500").unwrap_err();
        assert!(err.contains("9 characters"));
    }

    #[test]
    fn test_validate_fec_committee_id_invalid_prefix() {
        let err = validate_fec_committee_id("X00100005").unwrap_err();
        assert!(err.contains("'C'"));
    }

    #[test]
    fn test_validate_fec_committee_id_non_digit_after_c() {
        let err = validate_fec_committee_id("C0010000X").unwrap_err();
        assert!(err.contains("digits"));
    }

    #[test]
    fn test_validate_fec_committee_id_empty() {
        let err = validate_fec_committee_id("").unwrap_err();
        assert!(err.contains("9 characters"));
    }

    #[test]
    fn test_validate_fec_candidate_id_empty() {
        let err = validate_fec_candidate_id("").unwrap_err();
        assert!(err.contains("9 characters"));
    }
}
