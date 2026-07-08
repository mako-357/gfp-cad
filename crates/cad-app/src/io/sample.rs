//! Built-in sample building for "Load Sample" and first-run display.

use cad_core::{Building, Floor, GridAxis, Opening, Point2D, Room, Wall, WallMaterial};

pub fn building() -> Building {
    let mut b = Building::new("Sample House");
    b.metadata.usage = Some("住宅".into());
    b.metadata.structure_type = Some("木造".into());

    b.grid.x_axes = vec![
        GridAxis::new("X1", 0.0),
        GridAxis::new("X2", 5000.0),
        GridAxis::new("X3", 10000.0),
    ];
    b.grid.y_axes = vec![GridAxis::new("Y1", 0.0), GridAxis::new("Y2", 8000.0)];

    let mut f = Floor::new("1F", 0.0, 3000.0);

    // Nodes（隅を共有: 角で同じノードに解決される）。
    let sw = f.add_node(Point2D::new(0.0, 0.0));
    let se = f.add_node(Point2D::new(10000.0, 0.0));
    let nw = f.add_node(Point2D::new(0.0, 8000.0));
    let ne = f.add_node(Point2D::new(10000.0, 8000.0));
    let ps = f.add_node(Point2D::new(5000.0, 0.0));
    let pn = f.add_node(Point2D::new(5000.0, 8000.0));

    // Exterior shell (150mm).
    let mut south = Wall::new(sw, se, 150.0);
    south.is_exterior = true;
    south.material = WallMaterial::Wood;
    let south_id = south.id;

    let mut north = Wall::new(nw, ne, 150.0);
    north.is_exterior = true;

    let mut west = Wall::new(sw, nw, 150.0);
    west.is_exterior = true;

    let mut east = Wall::new(se, ne, 150.0);
    east.is_exterior = true;

    // Interior partition (100mm) on grid X2.
    let partition = Wall::new(ps, pn, 100.0);
    let partition_id = partition.id;

    f.walls.extend([south, north, west, east, partition]);

    // Openings.
    f.openings
        .push(Opening::window(south_id, 2500.0, 1600.0, 1200.0, 900.0));
    f.openings
        .push(Opening::door(partition_id, 4000.0, 800.0, 2000.0));

    // Rooms.
    f.rooms.push(Room::new(
        "LDK",
        vec![
            Point2D::new(0.0, 0.0),
            Point2D::new(5000.0, 0.0),
            Point2D::new(5000.0, 8000.0),
            Point2D::new(0.0, 8000.0),
        ],
    ));
    f.rooms.push(Room::new(
        "寝室",
        vec![
            Point2D::new(5000.0, 0.0),
            Point2D::new(10000.0, 0.0),
            Point2D::new(10000.0, 8000.0),
            Point2D::new(5000.0, 8000.0),
        ],
    ));

    b.add_floor(f);
    b
}
