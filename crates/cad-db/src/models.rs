use serde::{Deserialize, Serialize};
use surrealdb::RecordId;

/// ユーザー
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Option<RecordId>,
    pub email: String,
    pub name: Option<String>,
    pub picture: Option<String>,
    pub created_at: Option<String>,
    pub last_login_at: Option<String>,
}

/// 認証アイデンティティ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthIdentity {
    pub id: Option<RecordId>,
    pub user_id: Option<RecordId>,
    pub provider: String,
    pub provider_user_id: String,
    pub source: String,
}

/// ワークスペース
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    pub id: Option<RecordId>,
    pub name: String,
    pub owner_id: Option<RecordId>,
    pub created_at: Option<String>,
}

/// プロジェクト
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: Option<RecordId>,
    pub name: String,
    pub workspace_id: Option<RecordId>,
    pub created_by: Option<RecordId>,
    pub created_at: Option<String>,
}

/// 建物レコード（DB 保存用）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildingRecord {
    pub id: Option<RecordId>,
    pub project_id: Option<RecordId>,
    pub name: String,
    pub data: serde_json::Value,
    pub created_by: Option<RecordId>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

/// 建物サマリ（一覧用）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildingSummary {
    pub id: Option<RecordId>,
    pub name: String,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}
