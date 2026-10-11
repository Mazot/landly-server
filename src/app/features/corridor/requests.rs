use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateCorridorRequest {
    /// Home country the user comes from.
    pub from_country_id: Uuid,
    /// Destination country; must differ from `from_country_id`.
    pub to_country_id: Uuid,
    /// Make this the default corridor (replaces the current default). Defaults to false.
    pub is_default: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_corridor_request_deserialization() {
        let from = Uuid::new_v4();
        let to = Uuid::new_v4();
        let json = format!(
            r#"{{"from_country_id":"{}","to_country_id":"{}","is_default":true}}"#,
            from, to
        );

        let request: CreateCorridorRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(request.from_country_id, from);
        assert_eq!(request.to_country_id, to);
        assert_eq!(request.is_default, Some(true));
    }

    #[test]
    fn test_create_corridor_request_without_is_default() {
        let from = Uuid::new_v4();
        let to = Uuid::new_v4();
        let json = format!(
            r#"{{"from_country_id":"{}","to_country_id":"{}"}}"#,
            from, to
        );

        let request: CreateCorridorRequest = serde_json::from_str(&json).unwrap();
        assert!(request.is_default.is_none());
    }
}
