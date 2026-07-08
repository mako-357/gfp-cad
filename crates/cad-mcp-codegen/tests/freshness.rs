//! CI guard: cad-mcp の生成物と tool 表面が KDL(SSOT)と一致することを検証する。
//! 落ちたら `cargo run -p cad-mcp-codegen` で再生成 or server.rs を KDL に合わせる。

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use cad_mcp_codegen::{emit_manifest, emit_params, parse, parse_server_tools};

/// CRLF チェックアウト(git autocrlf on Windows)でも誤検出しないよう改行正規化。
fn lf(s: &str) -> String {
    s.replace("\r\n", "\n")
}

fn read(rel: &str) -> String {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    fs::read_to_string(root.join(rel)).unwrap_or_else(|e| panic!("{rel} が読めない: {e}"))
}

/// 生成物(params.rs / manifest.rs)が KDL と一致するか。
#[test]
fn generated_matches_kdl() {
    let schema = parse(&read("../cad-mcp/schema/cad-mcp.kdl")).expect("KDL parse 失敗");
    let cases = [
        ("../cad-mcp/src/generated/params.rs", emit_params(&schema)),
        (
            "../cad-mcp/src/generated/manifest.rs",
            emit_manifest(&schema),
        ),
    ];
    for (rel, expected) in cases {
        assert_eq!(
            lf(&read(rel)),
            lf(&expected),
            "{rel} が KDL と不一致。`cargo run -p cad-mcp-codegen` で再生成せよ"
        );
    }
}

/// server.rs の #[tool] 表面(name + description)が KDL(SSOT)と一致するか。
/// これで tool 名・説明の二重管理 drift（KDL と server.rs のズレ）を CI で防ぐ。
#[test]
fn server_tool_surface_matches_kdl() {
    let schema = parse(&read("../cad-mcp/schema/cad-mcp.kdl")).expect("KDL parse 失敗");
    let server = read("../cad-mcp/src/server.rs");
    let server_tools = parse_server_tools(&server).expect("server.rs の #[tool] 抽出失敗");

    let kdl_map: BTreeMap<&str, &str> = schema
        .tools
        .iter()
        .map(|t| (t.name.as_str(), t.description.as_str()))
        .collect();
    let srv_map: BTreeMap<&str, &str> = server_tools
        .iter()
        .map(|(n, d)| (n.as_str(), d.as_str()))
        .collect();

    assert_eq!(
        srv_map, kdl_map,
        "server.rs の #[tool] 表面が KDL(SSOT)と不一致。KDL を真として揃えよ"
    );
}
