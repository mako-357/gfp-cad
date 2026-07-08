//! KDL(SSOT `schema/cad-mcp.kdl`)から cad-mcp-codegen が生成したコード。手編集しない。
//!
//! - `params`: MCP tool の入力構造体（server.rs が使用）。
//! - `manifest`: tool 一覧。将来の tool_router 生成 + transparency guard の土台（現状は未宣言）。
//!
//! manifest の最新性は `cargo test -p cad-mcp-codegen` が検証する。

pub mod params;
