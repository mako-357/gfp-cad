use serde::{Deserialize, Serialize};
use surrealdb::types::{RecordId, SurrealValue};

#[derive(Debug, Clone, Serialize, Deserialize, SurrealValue)]
pub struct User {
    pub id: Option<RecordId>,
    pub email: String,
    pub name: Option<String>,
    pub picture: Option<String>,
    #[serde(default)]
    pub created_at: Option<serde_json::Value>,
    #[serde(default)]
    pub last_login_at: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, SurrealValue)]
pub struct AuthIdentity {
    pub id: Option<RecordId>,
    pub user_id: Option<RecordId>,
    pub provider: String,
    pub provider_user_id: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, SurrealValue)]
pub struct Workspace {
    pub id: Option<RecordId>,
    pub name: String,
    pub owner_id: Option<RecordId>,
    #[serde(default)]
    pub created_at: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, SurrealValue)]
pub struct Project {
    pub id: Option<RecordId>,
    pub name: String,
    pub workspace_id: Option<RecordId>,
    pub created_by: Option<RecordId>,
    #[serde(default)]
    pub created_at: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, SurrealValue)]
pub struct BuildingRecord {
    pub id: Option<RecordId>,
    pub project_id: Option<RecordId>,
    pub name: String,
    pub data: serde_json::Value,
    pub created_by: Option<RecordId>,
    #[serde(default)]
    pub created_at: Option<serde_json::Value>,
    #[serde(default)]
    pub updated_at: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, SurrealValue)]
pub struct BuildingSummary {
    pub id: Option<RecordId>,
    pub name: String,
    #[serde(default)]
    pub created_at: Option<serde_json::Value>,
    #[serde(default)]
    pub updated_at: Option<serde_json::Value>,
}
