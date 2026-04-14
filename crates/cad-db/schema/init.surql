-- gfp-cad データベーススキーマ
-- NS: gfp / DB: cad

-- ユーザー（Auth0 認証）
DEFINE TABLE users SCHEMAFULL;
DEFINE FIELD email ON users TYPE string
  ASSERT string::is_email($value);
DEFINE FIELD name ON users TYPE option<string>;
DEFINE FIELD picture ON users TYPE option<string>;
DEFINE FIELD created_at ON users TYPE datetime DEFAULT time::now()
  VALUE $before OR time::now();
DEFINE FIELD updated_at ON users TYPE datetime DEFAULT time::now()
  VALUE time::now();
DEFINE FIELD last_login_at ON users TYPE option<datetime>;
DEFINE INDEX email_idx ON users FIELDS email UNIQUE;

-- 認証アイデンティティ（Auth0 の multi-provider 対応）
DEFINE TABLE auth_identities SCHEMAFULL;
DEFINE FIELD user_id ON auth_identities TYPE record<users>;
DEFINE FIELD provider ON auth_identities TYPE string;        -- google-oauth2, github, auth0
DEFINE FIELD provider_user_id ON auth_identities TYPE string; -- Auth0 の sub claim
DEFINE FIELD source ON auth_identities TYPE string DEFAULT 'web'
  ASSERT $value INSIDE ['web', 'mcp-server', 'cli', 'api'];
DEFINE FIELD created_at ON auth_identities TYPE datetime DEFAULT time::now()
  VALUE $before OR time::now();
DEFINE FIELD last_used_at ON auth_identities TYPE datetime DEFAULT time::now()
  VALUE time::now();
DEFINE INDEX provider_idx ON auth_identities FIELDS provider, provider_user_id UNIQUE;

-- ワークスペース
DEFINE TABLE workspaces SCHEMAFULL;
DEFINE FIELD name ON workspaces TYPE string;
DEFINE FIELD owner_id ON workspaces TYPE record<users>;
DEFINE FIELD created_at ON workspaces TYPE datetime DEFAULT time::now()
  VALUE $before OR time::now();
DEFINE FIELD updated_at ON workspaces TYPE datetime DEFAULT time::now()
  VALUE time::now();

-- ワークスペースメンバー（リレーション）
DEFINE TABLE workspace_member TYPE RELATION IN users OUT workspaces SCHEMAFULL;
DEFINE FIELD role ON workspace_member TYPE string DEFAULT 'member'
  ASSERT $value INSIDE ['owner', 'admin', 'member'];
DEFINE FIELD joined_at ON workspace_member TYPE datetime DEFAULT time::now()
  VALUE $before OR time::now();

-- プロジェクト
DEFINE TABLE projects SCHEMAFULL;
DEFINE FIELD name ON projects TYPE string;
DEFINE FIELD workspace_id ON projects TYPE record<workspaces>;
DEFINE FIELD created_by ON projects TYPE record<users>;
DEFINE FIELD created_at ON projects TYPE datetime DEFAULT time::now()
  VALUE $before OR time::now();
DEFINE FIELD updated_at ON projects TYPE datetime DEFAULT time::now()
  VALUE time::now();
DEFINE INDEX ws_name_idx ON projects FIELDS workspace_id, name UNIQUE;

-- 建物
DEFINE TABLE buildings SCHEMAFULL;
DEFINE FIELD project_id ON buildings TYPE record<projects>;
DEFINE FIELD name ON buildings TYPE string;
DEFINE FIELD data ON buildings TYPE object FLEXIBLE;
DEFINE FIELD created_by ON buildings TYPE record<users>;
DEFINE FIELD created_at ON buildings TYPE datetime DEFAULT time::now()
  VALUE $before OR time::now();
DEFINE FIELD updated_at ON buildings TYPE datetime DEFAULT time::now()
  VALUE time::now();
DEFINE INDEX proj_name_idx ON buildings FIELDS project_id, name;
