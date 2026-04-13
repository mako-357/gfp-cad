use cad_core::*;

#[test]
fn test_building_creation() {
    let mut bldg = Building::new("山中湖別荘 B-1");
    assert_eq!(bldg.name, "山中湖別荘 B-1");
    assert!(bldg.floors.is_empty());

    let floor = Floor::new("1F", 200.0, 3000.0);
    bldg.add_floor(floor);
    assert_eq!(bldg.floors.len(), 1);
}

#[test]
fn test_grid_system() {
    let grid = GridSystem {
        x_axes: vec![
            GridAxis::new("C", 0.0),
            GridAxis::new("B", 6523.0),
            GridAxis::new("A", 12857.0),
        ],
        y_axes: vec![
            GridAxis::new("G", 0.0),
            GridAxis::new("F", 2875.0),
            GridAxis::new("E", 8957.0),
            GridAxis::new("D", 10680.0),
        ],
    };

    let x_spans = grid.x_spans();
    assert_eq!(x_spans.len(), 2);
    assert_eq!(x_spans[0].0, "C-B");
    assert!((x_spans[0].1 - 6523.0).abs() < 0.1);

    let y_spans = grid.y_spans();
    assert_eq!(y_spans.len(), 3);
}

#[test]
fn test_wall() {
    let wall = Wall::new(Point2D::new(0.0, 0.0), Point2D::new(6523.0, 0.0), 150.0);
    assert!((wall.length() - 6523.0).abs() < 0.1);
    // 壁面積: 6523mm × 2700mm = 17.61 sqm
    assert!((wall.area(2700.0) - 17.61).abs() < 0.1);
}

#[test]
fn test_room_area() {
    // 6.3m × 6.1m の部屋
    let room = Room::new(
        "リビング",
        vec![
            Point2D::new(0.0, 0.0),
            Point2D::new(6334.0, 0.0),
            Point2D::new(6334.0, 6082.0),
            Point2D::new(0.0, 6082.0),
        ],
    );
    // 面積: 6.334m × 6.082m = 38.5 sqm
    assert!((room.area() - 38.5).abs() < 0.5);
    assert!((room.perimeter() - 24832.0).abs() < 1.0);
}

#[test]
fn test_opening() {
    let wall = Wall::new(Point2D::new(0.0, 0.0), Point2D::new(6000.0, 0.0), 150.0);

    let door = Opening::door(wall.id, 1000.0, 900.0, 2100.0);
    assert_eq!(door.sill_height, 0.0);

    let window = Opening::window(wall.id, 3000.0, 1600.0, 1200.0, 800.0);
    assert_eq!(window.sill_height, 800.0);
}

#[test]
fn test_floor_area() {
    let mut floor = Floor::new("1F", 200.0, 3000.0);
    assert_eq!(floor.ceiling_height, 2700.0);

    floor.rooms.push(Room::new(
        "リビング",
        vec![
            Point2D::new(0.0, 0.0),
            Point2D::new(6000.0, 0.0),
            Point2D::new(6000.0, 5000.0),
            Point2D::new(0.0, 5000.0),
        ],
    ));
    floor.rooms.push(Room::new(
        "キッチン",
        vec![
            Point2D::new(6000.0, 0.0),
            Point2D::new(9000.0, 0.0),
            Point2D::new(9000.0, 5000.0),
            Point2D::new(6000.0, 5000.0),
        ],
    ));

    // 30sqm + 15sqm = 45sqm
    assert!((floor.area() - 45.0).abs() < 0.1);
}

#[test]
fn test_serialization() {
    let bldg = Building::new("テスト");
    let json = serde_json::to_string_pretty(&bldg).unwrap();
    let restored: Building = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.name, "テスト");
}
