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
    let wall_s_id = {
        let w = f1.add_wall_mut(Point2D::new(0.0, 0.0), Point2D::new(12857.0, 0.0), 150.0);
        w.is_exterior = true;
        w.material = WallMaterial::Wood;
        w.finish_exterior = Some("サイディング".into());
        w.finish_interior = Some("PB+VP".into());
        w.id
    };

    // 北壁 (D 通り)
    let wall_n_id = {
        let w = f1.add_wall_mut(
            Point2D::new(0.0, 10680.0),
            Point2D::new(12857.0, 10680.0),
            150.0,
        );
        w.is_exterior = true;
        w.material = WallMaterial::Wood;
        w.finish_exterior = Some("サイディング".into());
        w.id
    };

    // 西壁 (C 通り)
    let wall_w_id = {
        let w = f1.add_wall_mut(Point2D::new(0.0, 0.0), Point2D::new(0.0, 10680.0), 150.0);
        w.is_exterior = true;
        w.material = WallMaterial::Wood;
        w.id
    };

    // 東壁 (A 通り)
    let wall_e_id = {
        let w = f1.add_wall_mut(
            Point2D::new(12857.0, 0.0),
            Point2D::new(12857.0, 10680.0),
            150.0,
        );
        w.is_exterior = true;
        w.material = WallMaterial::Wood;
        w.id
    };

    // --- 間仕切壁 ---
    // B 通り上の壁（キッチン/リビング仕切り）
    f1.add_wall(
        Point2D::new(6523.0, 0.0),
        Point2D::new(6523.0, 2875.0),
        80.0,
    );

    // E 通り上の壁（水回り/リビング仕切り）
    f1.add_wall(
        Point2D::new(0.0, 8957.0),
        Point2D::new(6523.0, 8957.0),
        80.0,
    );

    // F 通り上の壁
    f1.add_wall(
        Point2D::new(0.0, 2875.0),
        Point2D::new(12857.0, 2875.0),
        80.0,
    );

    // fireplace 周りの腰壁 (H500)
    {
        let w = f1.add_wall_mut(
            Point2D::new(5000.0, 10000.0),
            Point2D::new(8000.0, 10000.0),
            150.0,
        );
        w.height = Some(500.0);
    }

    // --- 開口部 ---
    // 南壁の窓（リビング大開口）
    f1.openings
        .push(Opening::window(wall_s_id, 3000.0, 3600.0, 2000.0, 400.0));
    f1.openings
        .push(Opening::window(wall_s_id, 8000.0, 2400.0, 1200.0, 800.0));

    // 北壁の窓
    f1.openings
        .push(Opening::window(wall_n_id, 2000.0, 1600.0, 1200.0, 800.0));
    f1.openings
        .push(Opening::window(wall_n_id, 8000.0, 1600.0, 1200.0, 800.0));

    // 西壁の玄関ドア
    let mut entrance = Opening::door(wall_w_id, 5000.0, 900.0, 2100.0);
    entrance.kind = OpeningKind::SingleDoor;
    f1.openings.push(entrance);

    // 東壁の窓
    f1.openings
        .push(Opening::window(wall_e_id, 5000.0, 1600.0, 1200.0, 800.0));

    // --- 部屋 ---
    // リビング (B-A × F-G)
    // 重心: (6523,0),(12857,0),(12857,2875),(6523,2875) の平均 = (9690, 1437.5)
    let mut living = Room::new("リビング", Point2D::new(9690.0, 1437.5));
    living.has_floor_heating = true;
    living.floor_finish = Some("無垢フローリング".into());
    living.wall_finish = Some("PB+VP".into());

    // ダイニング・キッチン (C-B × F-G)
    // 重心: (0,0),(6523,0),(6523,2875),(0,2875) の平均 = (3261.5, 1437.5)
    let mut dk = Room::new("ダイニング・キッチン", Point2D::new(3261.5, 1437.5));
    dk.has_floor_heating = true;
    dk.floor_finish = Some("無垢フローリング".into());

    // メインルーム (C-A × E-F) — fireplace あり
    // 重心: (0,2875),(12857,2875),(12857,8957),(0,8957) の平均 = (6428.5, 5916)
    let mut main_room = Room::new("メインルーム", Point2D::new(6428.5, 5916.0));
    main_room.has_floor_heating = true;
    main_room.floor_finish = Some("無垢フローリング".into());

    // ユーティリティ (C-B × D-E)
    // 重心: (0,8957),(6523,8957),(6523,10680),(0,10680) の平均 = (3261.5, 9818.5)
    let utility = Room::new("ユーティリティ", Point2D::new(3261.5, 9818.5));

    // 寝室 (B-A × D-E)
    // 重心: (6523,8957),(12857,8957),(12857,10680),(6523,10680) の平均 = (9690, 9818.5)
    let mut bedroom = Room::new("寝室", Point2D::new(9690.0, 9818.5));
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
        println!(
            "\n{} (FL+{:.0}mm, CH={:.0}mm):",
            floor.name, floor.level, floor.ceiling_height
        );
        println!("  壁: {} 本", floor.walls.len());
        println!("  開口: {} 箇所", floor.openings.len());
        println!("  部屋: {} 室", floor.rooms.len());

        let ext_walls: Vec<_> = floor.walls.iter().filter(|w| w.is_exterior).collect();
        let int_walls: Vec<_> = floor.walls.iter().filter(|w| !w.is_exterior).collect();
        println!(
            "    外壁: {} 本 (総長 {:.1}m)",
            ext_walls.len(),
            ext_walls
                .iter()
                .map(|w| floor.wall_length(w).unwrap_or(0.0))
                .sum::<f64>()
                / 1000.0
        );
        println!(
            "    内壁: {} 本 (総長 {:.1}m)",
            int_walls.len(),
            int_walls
                .iter()
                .map(|w| floor.wall_length(w).unwrap_or(0.0))
                .sum::<f64>()
                / 1000.0
        );

        for room in &floor.rooms {
            let heating = if room.has_floor_heating {
                " [床暖房]"
            } else {
                ""
            };
            println!(
                "    {} — {:.1}sqm{}",
                room.name,
                floor.room_area(room).unwrap_or(0.0),
                heating
            );
        }

        println!("\n  床面積合計: {:.1}sqm", floor.area());
    }

    println!("\n延べ面積: {:.1}sqm", bldg.total_floor_area());

    // JSON 出力
    let json = serde_json::to_string_pretty(&bldg).unwrap();
    std::fs::write("/tmp/yamanakako-b1.json", &json).unwrap();
    println!(
        "\nJSON saved to /tmp/yamanakako-b1.json ({} bytes)",
        json.len()
    );
}
