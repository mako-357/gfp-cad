//! Load/save a `Building` as JSON — the same format cad-mcp / cad-db use.

use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::Path;

use cad_core::Building;

pub fn load(path: &Path) -> anyhow::Result<Building> {
    let file = File::open(path)?;
    let building = serde_json::from_reader(BufReader::new(file))?;
    Ok(building)
}

pub fn save(path: &Path, building: &Building) -> anyhow::Result<()> {
    let file = File::create(path)?;
    serde_json::to_writer_pretty(BufWriter::new(file), building)?;
    Ok(())
}
