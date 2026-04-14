use anyhow::{Context, Result};
use cad_core::Building;
use surrealdb::types::RecordId;

use crate::client::CadDbClient;
use crate::models::*;

impl CadDbClient {
    // === ユーザー ===

    pub async fn upsert_user_by_auth(
        &self,
        provider: &str,
        provider_user_id: &str,
        email: &str,
        name: Option<&str>,
        picture: Option<&str>,
    ) -> Result<User> {
        let provider = provider.to_string();
        let provider_user_id = provider_user_id.to_string();
        let email = email.to_string();
        let name = name.map(|s| s.to_string());
        let picture = picture.map(|s| s.to_string());

        // 既存ユーザーの検索（トランザクション外 — 読み取りのみ）
        let mut result = self
            .db
            .query("SELECT * FROM auth_identities WHERE provider = $provider AND provider_user_id = $pid LIMIT 1")
            .bind(("provider", provider.clone()))
            .bind(("pid", provider_user_id.clone()))
            .await?;
        let existing: Vec<AuthIdentity> = result.take(0)?;

        if let Some(identity) = existing.first()
            && let Some(ref user_id) = identity.user_id
        {
            let mut result = self
                .db
                .query("UPDATE $uid SET last_login_at = time::now()")
                .bind(("uid", user_id.clone()))
                .await?;
            let users: Vec<User> = result.take(0)?;
            if let Some(user) = users.into_iter().next() {
                return Ok(user);
            }
        }

        // 新規ユーザー作成はトランザクションでラップ（user + auth_identity の整合性保証）
        let mut result = self
            .db
            .query("BEGIN TRANSACTION; \
                    LET $user = CREATE users CONTENT { email: $email, name: $name, picture: $picture }; \
                    CREATE auth_identities CONTENT { user_id: $user[0].id, provider: $provider, provider_user_id: $pid, source: 'mcp-server' }; \
                    COMMIT TRANSACTION; \
                    SELECT * FROM users WHERE email = $email ORDER BY created_at DESC LIMIT 1")
            .bind(("email", email))
            .bind(("name", name))
            .bind(("picture", picture))
            .bind(("provider", provider))
            .bind(("pid", provider_user_id))
            .await
            .context("ユーザー作成トランザクション失敗")?;
        // トランザクション後の SELECT 結果を取得（ステートメントインデックス 4）
        let users: Vec<User> = result.take(4)?;
        let user = users.into_iter().next().context("ユーザー作成結果なし")?;

        let ws_name = format!("{}のワークスペース", user.name.as_deref().unwrap_or("My"));
        self.create_workspace(&user, &ws_name).await?;

        Ok(user)
    }

    // === ワークスペース ===

    pub async fn create_workspace(&self, user: &User, name: &str) -> Result<Workspace> {
        let name = name.to_string();
        let uid = user.id.clone();
        let mut result = self
            .db
            .query("CREATE workspaces CONTENT { name: $name, owner_id: $uid }")
            .bind(("name", name))
            .bind(("uid", uid))
            .await
            .context("ワークスペース作成失敗")?;
        let workspaces: Vec<Workspace> = result.take(0)?;
        let ws = workspaces.into_iter().next().context("ワークスペース作成結果なし")?;

        if let (Some(uid), Some(wsid)) = (&user.id, &ws.id) {
            self.db
                .query("RELATE $uid->workspace_member->$wsid SET role = 'owner'")
                .bind(("uid", uid.clone()))
                .bind(("wsid", wsid.clone()))
                .await?;
        }

        Ok(ws)
    }

    pub async fn list_workspaces(&self, user: &User) -> Result<Vec<Workspace>> {
        let Some(ref uid) = user.id else {
            return Ok(Vec::new());
        };
        let mut result = self
            .db
            .query("SELECT * FROM workspaces WHERE id IN (SELECT VALUE out FROM workspace_member WHERE `in` = $uid)")
            .bind(("uid", uid.clone()))
            .await?;
        let workspaces: Vec<Workspace> = result.take(0)?;
        Ok(workspaces)
    }

    // === プロジェクト ===

    pub async fn create_project(
        &self,
        workspace_id: &RecordId,
        name: &str,
        user: &User,
    ) -> Result<Project> {
        let name = name.to_string();
        let wsid = workspace_id.clone();
        let uid = user.id.clone();
        let mut result = self
            .db
            .query("CREATE projects CONTENT { name: $name, workspace_id: $wsid, created_by: $uid }")
            .bind(("name", name))
            .bind(("wsid", wsid))
            .bind(("uid", uid))
            .await
            .context("プロジェクト作成失敗")?;
        let projects: Vec<Project> = result.take(0)?;
        projects.into_iter().next().context("プロジェクト作成結果なし")
    }

    pub async fn list_projects(&self, workspace_id: &RecordId) -> Result<Vec<Project>> {
        let wsid = workspace_id.clone();
        let mut result = self
            .db
            .query("SELECT * FROM projects WHERE workspace_id = $wsid ORDER BY created_at DESC")
            .bind(("wsid", wsid))
            .await?;
        let projects: Vec<Project> = result.take(0)?;
        Ok(projects)
    }

    // === 建物 ===

    pub async fn save_building(
        &self,
        project_id: &RecordId,
        building: &Building,
        user: &User,
    ) -> Result<BuildingRecord> {
        let data = serde_json::to_value(building)?;
        let bname = building.name.clone();
        let pid = project_id.clone();
        let uid = user.id.clone();
        let mut result = self
            .db
            .query("CREATE buildings CONTENT { project_id: $pid, name: $name, data: $data, created_by: $uid }")
            .bind(("pid", pid))
            .bind(("name", bname))
            .bind(("data", data))
            .bind(("uid", uid))
            .await
            .context("建物保存失敗")?;
        let records: Vec<BuildingRecord> = result.take(0)?;
        records.into_iter().next().context("建物保存結果なし")
    }

    pub async fn load_building(&self, building_id: &RecordId) -> Result<Building> {
        let bid = building_id.clone();
        let mut result = self
            .db
            .query("SELECT * FROM $bid")
            .bind(("bid", bid))
            .await?;
        let records: Vec<BuildingRecord> = result.take(0)?;
        let record = records.into_iter().next().context("建物が見つかりません")?;
        let building: Building = serde_json::from_value(record.data)?;
        Ok(building)
    }

    pub async fn list_buildings(&self, project_id: &RecordId) -> Result<Vec<BuildingSummary>> {
        let pid = project_id.clone();
        let mut result = self
            .db
            .query("SELECT id, name, created_at, updated_at FROM buildings WHERE project_id = $pid ORDER BY updated_at DESC")
            .bind(("pid", pid))
            .await?;
        let buildings: Vec<BuildingSummary> = result.take(0)?;
        Ok(buildings)
    }
}
