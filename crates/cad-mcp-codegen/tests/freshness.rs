//! CI guard: cad-mcp の生成物が KDL(SSOT)と一致することを検証する。
//! 落ちたら `cargo run -p cad-mcp-codegen` で再生成してコミットする。

use std::fs;
use std::path::Path;

use cad_mcp_codegen::{emit_manifest, emit_params, parse};

#[test]
fn generated_matches_kdl() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let src = fs::read_to_string(root.join("../cad-mcp/schema/cad-mcp.kdl"))
        .expect("schema/cad-mcp.kdl が読めない");
    let schema = parse(&src).expect("KDL parse 失敗");

    let cases = [
        ("../cad-mcp/src/generated/params.rs", emit_params(&schema)),
        (
            "../cad-mcp/src/generated/manifest.rs",
            emit_manifest(&schema),
        ),
    ];
    for (rel, expected) in cases {
        let on_disk = fs::read_to_string(root.join(rel)).unwrap_or_default();
        assert_eq!(
            on_disk, expected,
            "{rel} が KDL と不一致。`cargo run -p cad-mcp-codegen` で再生成せよ"
        );
    }
}
