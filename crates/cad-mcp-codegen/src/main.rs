//! Regenerate cad-mcp の生成物を KDL(SSOT)から書き出す。
//!
//! 使い方: `cargo run -p cad-mcp-codegen`
//! 検証:   `cargo test -p cad-mcp-codegen`（生成物が最新かを assert）

use std::fs;
use std::path::Path;

use cad_mcp_codegen::{emit_manifest, emit_params, parse};

fn main() -> anyhow::Result<()> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let kdl = root.join("../cad-mcp/schema/cad-mcp.kdl");
    let src = fs::read_to_string(&kdl)?;
    let schema = parse(&src)?;

    let gen_dir = root.join("../cad-mcp/src/generated");
    fs::create_dir_all(&gen_dir)?;
    fs::write(gen_dir.join("params.rs"), emit_params(&schema))?;
    fs::write(gen_dir.join("manifest.rs"), emit_manifest(&schema))?;

    println!(
        "generated: {} params records, {} tools",
        schema.records.len(),
        schema.tools.len()
    );
    Ok(())
}
