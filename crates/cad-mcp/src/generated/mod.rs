//! KDL(SSOT `schema/cad-mcp.kdl`)から cad-mcp-codegen が生成したコード。手編集しない。
//!
//! - `params`   : MCP tool の入力構造体（server.rs が使用）
//! - `manifest` : tool 一覧。将来の tool_router 生成 + transparency guard 用（現状は
//!                cad-mcp からは未消費のため未宣言。`cargo test -p cad-mcp-codegen` が
//!                最新性を検証する）。

pub mod params;
