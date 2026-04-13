/// 山中湖別荘 B-1 — AutoCAD 図面から抽出したデータを gfp-cad モデルに変換
use cad_core::*;

fn main() {
    let mut bldg = Building::new("山中湖別荘 B-1");
    bldg.metadata.building_area = Some(137.3);
    bldg.metadata.total_floor_area = Some(165.6);
    bldg.metadata.usage = Some("別荘".into());
    bldg.metadata.structure_type = Some("木造".into());

    // === 通り芯 ===
    // X方向（南北）: C, B, A
    bldg.grid.x_axes = vec![
        GridAxis::new("C", 0.0),
        GridAxis::new("B", 6523.0),
        GridAxis::new("A", 12857.0),
    ];
    // Y方向（東西）: G, F, E, D
    bldg.grid.y_axes = vec![
        GridAxis::new("G", 0.0),
        GridAxis::new("F", 2875.0),
        GridAxis::new("E", 8957.0),
        GridAxis::new("D", 10680.0),
    ];

    // === 1階 ===
    let mut f1 = Floor::new("1F", 200.0, 3000.0);
    f1.ceiling_height = 2700.0;

    // --- 外壁 ---
    // 南壁 (G 通り)
    let mut wall_s = Wall::new(
        Point2D::new(0.0, 0.0),
        Point2D::new(12857.0, 0.0),
        150.0,
    );
    wall_s.is_exterior = true;
    wall_s.material = WallMaterial::Wood;
    wall_s.finish_exterior = Some("サイディング".into());
    wall_s.finish_interior = Some("PB+VP".into());
    let wall_s_id = wall_s.id;

    // 北壁 (D 通り)
    let mut wall_n = Wall::new(
        Point2D::new(0.0, 10680.0),
        Point2D::new(12857.0, 10680.0),
        150.0,
    );
    wall_n.is_exterior = true;
    wall_n.material = WallMaterial::Wood;
    wall_n.finish_exterior = Some("サイディング".into());
    let wall_n_id = wall_n.id;

    // 西壁 (C 通り)
    let mut wall_w = Wall::new(
        Point2D::new(0.0, 0.0),
        Point2D::new(0.0, 10680.0),
        150.0,
    );
    wall_w.is_exterior = true;
    wall_w.material = WallMaterial::Wood;
    let wall_w_id = wall_w.id;

    // 東壁 (A 通り)
    let mut wall_e = Wall::new(
        Point2D::new(12857.0, 0.0),
        Point2D::new(12857.0, 10680.0),
        150.0,
    );
    wall_e.is_exterior = true;
    wall_e.material = WallMaterial::Wood;
    let wall_e_id = wall_e.id;

    // --- 間仕切壁 ---
    // B 通り上の壁（キッチン/リビング仕切り）
    let wall_b = Wall::new(
        Point2D::new(6523.0, 0.0),
        Point2D::new(6523.0, 2875.0),
        80.0,
    );

    // E 通り上の壁（水回り/リビング仕切り）
    let wall_e_inner = Wall::new(
        Point2D::new(0.0, 8957.0),
        Point2D::new(6523.0, 8957.0),
        80.0,
    );

    // F 通り上の壁
    let wall_f = Wall::new(
        Point2D::new(0.0, 2875.0),
        Point2D::new(12857.0, 2875.0),
        80.0,
    );

    // fireplace 周りの腰壁 (H500)
    let mut wall_fp = Wall::new(
        Point2D::new(5000.0, 10000.0),
        Point2D::new(8000.0, 10000.0),
        150.0,
    );
    wall_fp.height = Some(500.0);

    f1.walls = vec![wall_s, wall_n, wall_w, wall_e, wall_b, wall_e_inner, wall_f, wall_fp];

    // --- 開口部 ---
    // 南壁の窓（リビング大開口）
    f1.openings.push(Opening::window(wall_s_id, 3000.0, 3600.0, 2000.0, 400.0));
    f1.openings.push(Opening::window(wall_s_id, 8000.0, 2400.0, 1200.0, 800.0));

    // 北壁の窓
    f1.openings.push(Opening::window(wall_n_id, 2000.0, 1600.0, 1200.0, 800.0));
    f1.openings.push(Opening::window(wall_n_id, 8000.0, 1600.0, 1200.0, 800.0));

    // 西壁の玄関ドア
    let mut entrance = Opening::door(wall_w_id, 5000.0, 900.0, 2100.0);
    entrance.kind = OpeningKind::SingleDoor;
    f1.openings.push(entrance);

    // 東壁の窓
    f1.openings.push(Opening::window(wall_e_id, 5000.0, 1600.0, 1200.0, 800.0));

    // --- 部屋 ---
    // リビング (B-A × F-G)
    let mut living = Room::new(
        "リビング",
        vec![
            Point2D::new(6523.0, 0.0),
            Point2D::new(12857.0, 0.0),
            Point2D::new(12857.0, 2875.0),
            Point2D::new(6523.0, 2875.0),
        ],
    );
    living.has_floor_heating = true;
    living.floor_finish = Some("無垢フローリング".into());
    living.wall_finish = Some("PB+VP".into());

    // ダイニング・キッチン (C-B × F-G)
    let mut dk = Room::new(
        "ダイニング・キッチン",
        vec![
            Point2D::new(0.0, 0.0),
            Point2D::new(6523.0, 0.0),
            Point2D::new(6523.0, 2875.0),
            Point2D::new(0.0, 2875.0),
        ],
    );
    dk.has_floor_heating = true;
    dk.floor_finish = Some("無垢フローリング".into());

    // メインルーム (C-A × E-F) — fireplace あり
    let mut main_room = Room::new(
        "メインルーム",
        vec![
            Point2D::new(0.0, 2875.0),
            Point2D::new(12857.0, 2875.0),
            Point2D::new(12857.0, 8957.0),
            Point2D::new(0.0, 8957.0),
        ],
    );
    main_room.has_floor_heating = true;
    main_room.floor_finish = Some("無垢フローリング".into());

    // ユーティリティ (C-B × D-E)
    let utility = Room::new(
        "ユーティリティ",
        vec![
            Point2D::new(0.0, 8957.0),
            Point2D::new(6523.0, 8957.0),
            Point2D::new(6523.0, 10680.0),
            Point2D::new(0.0, 10680.0),
        ],
    );

    // 寝室 (B-A × D-E)
    let mut bedroom = Room::new(
        "寝室",
        vec![
            Point2D::new(6523.0, 8957.0),
            Point2D::new(12857.0, 8957.0),
            Point2D::new(12857.0, 10680.0),
            Point2D::new(6523.0, 10680.0),
        ],
    );
    bedroom.floor_finish = Some("無垢フローリング".into());

    f1.rooms = vec![living, dk, main_room, utility, bedroom];

    bldg.add_floor(f1);

    // === 出力 ===
    println!("=== {} ===", bldg.name);
    println!("ID: {}", bldg.id);

    // 通り芯
    println!("\n通り芯:");
    for (name, span) in bldg.grid.x_spans() {
        println!("  X: {} = {:.0}mm ({:.2}m)", name, span, span / 1000.0);
    }
    for (name, span) in bldg.grid.y_spans() {
        println!("  Y: {} = {:.0}mm ({:.2}m)", name, span, span / 1000.0);
    }

    // 階情報
    for floor in &bldg.floors {
        println!("\n{} (FL+{:.0}mm, CH={:.0}mm):", floor.name, floor.level, floor.ceiling_height);
        println!("  壁: {} 本", floor.walls.len());
        println!("  開口: {} 箇所", floor.openings.len());
        println!("  部屋: {} 室", floor.rooms.len());

        let ext_walls: Vec<_> = floor.walls.iter().filter(|w| w.is_exterior).collect();
        let int_walls: Vec<_> = floor.walls.iter().filter(|w| !w.is_exterior).collect();
        println!("    外壁: {} 本 (総長 {:.1}m)", ext_walls.len(),
            ext_walls.iter().map(|w| w.length()).sum::<f64>() / 1000.0);
        println!("    内壁: {} 本 (総長 {:.1}m)", int_walls.len(),
            int_walls.iter().map(|w| w.length()).sum::<f64>() / 1000.0);

        for room in &floor.rooms {
            let heating = if room.has_floor_heating { " [床暖房]" } else { "" };
            println!("    {} — {:.1}sqm{}", room.name, room.area(), heating);
        }

        println!("\n  床面積合計: {:.1}sqm", floor.area());
    }

    println!("\n延べ面積: {:.1}sqm", bldg.total_floor_area());

    // JSON 出力
    let json = serde_json::to_string_pretty(&bldg).unwrap();
    std::fs::write("/tmp/yamanakako-b1.json", &json).unwrap();
    println!("\nJSON saved to /tmp/yamanakako-b1.json ({} bytes)", json.len());
}
