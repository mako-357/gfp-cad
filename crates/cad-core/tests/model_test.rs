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
    // 6334×6082 の矩形を壁で囲み、内部シードから面積・周長を導出。
    let mut floor = Floor::new("1F", 200.0, 3000.0);
    floor.add_wall(Point2D::new(0.0, 0.0), Point2D::new(6334.0, 0.0), 100.0);
    floor.add_wall(
        Point2D::new(6334.0, 0.0),
        Point2D::new(6334.0, 6082.0),
        100.0,
    );
    floor.add_wall(
        Point2D::new(6334.0, 6082.0),
        Point2D::new(0.0, 6082.0),
        100.0,
    );
    floor.add_wall(Point2D::new(0.0, 6082.0), Point2D::new(0.0, 0.0), 100.0);
    let room = Room::new("リビング", Point2D::new(3000.0, 3000.0));
    // 面積: 6.334m × 6.082m = 38.5 sqm（壁芯基準）
    assert!((floor.room_area(&room).unwrap() - 38.5).abs() < 0.5);
    assert!((floor.room_perimeter(&room).unwrap() - 24832.0).abs() < 1.0);
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
    // 9000×5000 の外周を X=6000 の仕切りで2分割（左 30sqm / 右 15sqm）。
    let mut floor = Floor::new("1F", 200.0, 3000.0);
    assert_eq!(floor.ceiling_height, 2700.0);

    // 外周
    floor.add_wall(Point2D::new(0.0, 0.0), Point2D::new(9000.0, 0.0), 100.0);
    floor.add_wall(
        Point2D::new(9000.0, 0.0),
        Point2D::new(9000.0, 5000.0),
        100.0,
    );
    floor.add_wall(
        Point2D::new(9000.0, 5000.0),
        Point2D::new(0.0, 5000.0),
        100.0,
    );
    floor.add_wall(Point2D::new(0.0, 5000.0), Point2D::new(0.0, 0.0), 100.0);
    // 仕切り（端点は外周スパン上＝T字。planarize で連結）
    floor.add_wall(
        Point2D::new(6000.0, 0.0),
        Point2D::new(6000.0, 5000.0),
        100.0,
    );

    floor
        .rooms
        .push(Room::new("リビング", Point2D::new(3000.0, 2500.0)));
    floor
        .rooms
        .push(Room::new("キッチン", Point2D::new(7500.0, 2500.0)));

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
fn nested_room_subtracts_hole() {
    // 外周 10000×10000 の中に、非連結の 2000×2000 クローゼットが浮いている。
    let mut floor = Floor::new("1F", 0.0, 3000.0);
    for (a, b) in [
        ((0.0, 0.0), (10000.0, 0.0)),
        ((10000.0, 0.0), (10000.0, 10000.0)),
        ((10000.0, 10000.0), (0.0, 10000.0)),
        ((0.0, 10000.0), (0.0, 0.0)),
        // 非連結の内側ループ
        ((4000.0, 4000.0), (6000.0, 4000.0)),
        ((6000.0, 4000.0), (6000.0, 6000.0)),
        ((6000.0, 6000.0), (4000.0, 6000.0)),
        ((4000.0, 6000.0), (4000.0, 4000.0)),
    ] {
        floor.add_wall(Point2D::new(a.0, a.1), Point2D::new(b.0, b.1), 100.0);
    }
    let hall = Room::new("hall", Point2D::new(1000.0, 1000.0)); // 穴の外
    let closet = Room::new("closet", Point2D::new(5000.0, 5000.0)); // 穴の中
    // hall = 100 - 4 = 96㎡（穴を差し引く）、closet = 4㎡。
    assert!((floor.room_area(&hall).unwrap() - 96.0).abs() < 0.1);
    assert!((floor.room_area(&closet).unwrap() - 4.0).abs() < 0.1);
    floor.rooms.push(hall);
    floor.rooms.push(closet);
    // 延べ面積は実フットプリント 100㎡（二重計上なし）。
    assert!((floor.area() - 100.0).abs() < 0.1);
}

#[test]
fn duplicate_seed_counts_area_once() {
    // 5000×4000=20㎡ の1部屋に2シード → 面積は1回だけ計上、validate が競合を検出。
    let mut floor = Floor::new("1F", 0.0, 3000.0);
    for (a, b) in [
        ((0.0, 0.0), (5000.0, 0.0)),
        ((5000.0, 0.0), (5000.0, 4000.0)),
        ((5000.0, 4000.0), (0.0, 4000.0)),
        ((0.0, 4000.0), (0.0, 0.0)),
    ] {
        floor.add_wall(Point2D::new(a.0, a.1), Point2D::new(b.0, b.1), 100.0);
    }
    floor
        .rooms
        .push(Room::new("a", Point2D::new(2000.0, 2000.0)));
    floor
        .rooms
        .push(Room::new("b", Point2D::new(3000.0, 2000.0)));
    assert!((floor.area() - 20.0).abs() < 0.1, "同一面は1回だけ");
    assert!(
        floor
            .validate()
            .iter()
            .any(|s| s.contains("同一の面に複数"))
    );
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
