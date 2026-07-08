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
    let mut floor = Floor::new("1F", 200.0, 3000.0);
    floor.add_wall(Point2D::new(0.0, 0.0), Point2D::new(6523.0, 0.0), 150.0);
    let wall = &floor.walls[0];
    let length = floor.wall_length(wall).unwrap();
    assert!((length - 6523.0).abs() < 0.1);
    // 壁面積: 6523mm × 2700mm = 17.61 sqm
    let area = length * 2700.0 / 1_000_000.0;
    assert!((area - 17.61).abs() < 0.1);
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
    let mut floor = Floor::new("1F", 200.0, 3000.0);
    let wall_id = floor.add_wall(Point2D::new(0.0, 0.0), Point2D::new(6000.0, 0.0), 150.0);

    let door = Opening::door(wall_id, 1000.0, 900.0, 2100.0);
    assert_eq!(door.sill_height, 0.0);

    let window = Opening::window(wall_id, 3000.0, 1600.0, 1200.0, 800.0);
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

#[test]
fn test_shared_node_join() {
    // 端点を共有する2壁は同じノードに解決される（構造的接合）。
    let mut floor = Floor::new("1F", 0.0, 3000.0);
    floor.add_wall(Point2D::new(0.0, 0.0), Point2D::new(1000.0, 0.0), 100.0);
    floor.add_wall(
        Point2D::new(1000.0, 0.0),
        Point2D::new(1000.0, 1000.0),
        100.0,
    );
    // 端点: (0,0),(1000,0),(1000,1000) の3ノード（(1000,0)を共有）。
    assert_eq!(floor.nodes.len(), 3);
    assert_eq!(floor.walls[0].end, floor.walls[1].start);
    // 健全なモデルは validate で問題ゼロ。
    assert!(floor.validate().is_empty());
}

#[test]
fn test_validate_detects_dangling_and_orphan() {
    let mut floor = Floor::new("1F", 0.0, 3000.0);
    floor.add_wall(Point2D::new(0.0, 0.0), Point2D::new(1000.0, 0.0), 100.0);
    // 実在しないノードを参照する壁 → dangling（floor に追加していない Node の id）。
    floor.walls[0].end = Node::new(Point2D::new(-1.0, -1.0)).id;
    // どの壁も参照しないノードを追加 → orphan。
    floor.add_node(Point2D::new(9999.0, 9999.0));
    let issues = floor.validate();
    assert!(issues.iter().any(|s| s.contains("dangling")));
    assert!(issues.iter().any(|s| s.contains("orphan")));
}
