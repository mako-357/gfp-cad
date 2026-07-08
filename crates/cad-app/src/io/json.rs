//! Load/save a `Building` as JSON — the same format cad-mcp / cad-db use.

use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::Path;

use cad_core::Building;

pub fn load(path: &Path) -> anyhow::Result<Building> {
    let file = File::open(path)?;
    let building: Building = serde_json::from_reader(BufReader::new(file))?;
    // モデル整合の問題（dangling ノード等）は silent なデータ欠落になるので警告する。
    for issue in building.validate() {
        log::warn!("読み込んだモデルに不整合: {issue}");
    }
    Ok(building)
}

pub fn save(path: &Path, building: &Building) -> anyhow::Result<()> {
    let file = File::create(path)?;
    serde_json::to_writer_pretty(BufWriter::new(file), building)?;
    Ok(())
}
