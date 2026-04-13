# gfp-cad

Lightweight BIM for AI-driven architectural design.

Claude (LLM) が建築設計を自動化するためのセマンティック建築モデル。壁・開口・部屋を「意味」として持ち、AutoCAD / DXF / Jw_cad に出力する。

## Architecture

```
Building (Semantic Model)
    |
    +-- cad-acad --> AutoCAD (via autocad-mcp plugin)
    +-- cad-dxf  --> DXF UTF-8 (AutoCAD)
    +-- cad-dxf  --> DXF Shift-JIS (Jw_cad, no garbled text)
```

## Data Model

```
Building
  +-- GridSystem (grid axes with names & positions)
  +-- Floor[]
        +-- Wall[] (centerline, thickness, material, finish)
        +-- Opening[] (door/window, position on wall, dimensions)
        +-- Room[] (name, boundary polygon, finishes, floor heating)
```

All dimensions in **mm**. Areas computed in **sqm**.

## Crates

| Crate | Description |
|-------|-------------|
| `cad-core` | Domain model: Building, Floor, Wall, Opening, Room, GridSystem |
| `cad-dxf` | DXF R12 export (UTF-8 for AutoCAD, Shift-JIS for Jw_cad) |
| `cad-acad` | AutoCAD output via [autocad-mcp](https://github.com/mako-357/autocad-mcp) |

## Quick Start

```bash
# Build
cargo build

# Run tests
cargo test

# Export sample building to DXF
cargo run --example export_yamanakako

# Render to AutoCAD (requires autocad-mcp plugin loaded)
cargo run --example render_yamanakako
```

## Example: Define a Building

```rust
use cad_core::*;

let mut bldg = Building::new("My House");

// Grid system
bldg.grid.x_axes = vec![
    GridAxis::new("A", 0.0),
    GridAxis::new("B", 6000.0),
];
bldg.grid.y_axes = vec![
    GridAxis::new("1", 0.0),
    GridAxis::new("2", 8000.0),
];

// Floor
let mut floor = Floor::new("1F", 200.0, 3000.0);

// Walls
let mut wall = Wall::new(
    Point2D::new(0.0, 0.0),
    Point2D::new(6000.0, 0.0),
    150.0,
);
wall.is_exterior = true;
let wall_id = wall.id;
floor.walls.push(wall);

// Openings
floor.openings.push(Opening::window(wall_id, 2000.0, 1600.0, 1200.0, 800.0));

// Rooms
floor.rooms.push(Room::new("Living", vec![
    Point2D::new(0.0, 0.0),
    Point2D::new(6000.0, 0.0),
    Point2D::new(6000.0, 8000.0),
    Point2D::new(0.0, 8000.0),
]));

bldg.add_floor(floor);

// Computed values
println!("Floor area: {:.1} sqm", bldg.total_floor_area());
```

## Export to DXF

```rust
use cad_dxf::DxfExporter;

// For Jw_cad (Shift-JIS, no garbled Japanese text)
let exporter = DxfExporter::for_jwcad();
let mut file = std::fs::File::create("output.dxf").unwrap();
exporter.export(&bldg, &mut file).unwrap();

// For AutoCAD (UTF-8)
let exporter = DxfExporter::for_autocad();
```

## Render to AutoCAD

Requires [autocad-mcp](https://github.com/mako-357/autocad-mcp) plugin loaded in AutoCAD.

```rust
use cad_acad::AcadRenderer;

let renderer = AcadRenderer::new();
let report = renderer.render_building(&bldg).unwrap();
println!("{report}"); // "Rendered: 6 layers, 7 grid lines, 8 walls, ..."
```

## Related

- [autocad-mcp](https://github.com/mako-357/autocad-mcp) - MCP server for AutoCAD (ObjectARX plugin + Rust MCP server)

## License

MIT
