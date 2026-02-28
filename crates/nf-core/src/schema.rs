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
        reg.entity_types.insert("Person".to_string(), EntitySchema {
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
        });

        reg.entity_types.insert("Organization".to_string(), EntitySchema {
            required_fields: vec!["name".to_string(), "org_type".to_string()],
            optional_fields: vec![
                "aliases".to_string(),
                "jurisdiction".to_string(),
                "known_principals".to_string(),
                "foreign_connection".to_string(),
            ],
            source_required: true,
        });

        reg.entity_types.insert("Payment".to_string(), EntitySchema {
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
        });

        reg.entity_types.insert("TimingCorrelation".to_string(), EntitySchema {
            required_fields: vec![
                "event_a".to_string(),
                "event_b".to_string(),
                "days_between".to_string(),
                "correlation_type".to_string(),
            ],
            optional_fields: vec!["threshold_days".to_string()],
            source_required: true,
        });

        reg.entity_types.insert("ConductComparison".to_string(), EntitySchema {
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
        });

        reg
    }
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
}
