/// 山中湖別荘 B-1 を gfp-cad モデルから AutoCAD に描画
use cad_acad::AcadRenderer;
use cad_core::*;

fn build_yamanakako() -> Building {
    let mut bldg = Building::new("山中湖別荘 B-1");
    bldg.metadata.usage = Some("別荘".into());
    bldg.metadata.structure_type = Some("木造".into());

    bldg.grid.x_axes = vec![
        GridAxis::new("C", 0.0),
        GridAxis::new("B", 6523.0),
        GridAxis::new("A", 12857.0),
    ];
    bldg.grid.y_axes = vec![
        GridAxis::new("G", 0.0),
        GridAxis::new("F", 2875.0),
        GridAxis::new("E", 8957.0),
        GridAxis::new("D", 10680.0),
    ];

    let mut f1 = Floor::new("1F", 200.0, 3000.0);
    f1.ceiling_height = 2700.0;

    // 外壁
    let mut ws = Wall::new(Point2D::new(0.0, 0.0), Point2D::new(12857.0, 0.0), 150.0);
    ws.is_exterior = true;
    ws.material = WallMaterial::Wood;
    let ws_id = ws.id;

    let mut wn = Wall::new(Point2D::new(0.0, 10680.0), Point2D::new(12857.0, 10680.0), 150.0);
    wn.is_exterior = true;
    let wn_id = wn.id;

    let mut ww = Wall::new(Point2D::new(0.0, 0.0), Point2D::new(0.0, 10680.0), 150.0);
    ww.is_exterior = true;
    let ww_id = ww.id;

    let mut we = Wall::new(Point2D::new(12857.0, 0.0), Point2D::new(12857.0, 10680.0), 150.0);
    we.is_exterior = true;
    let we_id = we.id;

    // 間仕切壁
    let w_b = Wall::new(Point2D::new(6523.0, 0.0), Point2D::new(6523.0, 2875.0), 80.0);
    let w_f = Wall::new(Point2D::new(0.0, 2875.0), Point2D::new(12857.0, 2875.0), 80.0);
    let w_e = Wall::new(Point2D::new(0.0, 8957.0), Point2D::new(6523.0, 8957.0), 80.0);
    let w_e2 = Wall::new(Point2D::new(6523.0, 8957.0), Point2D::new(12857.0, 8957.0), 80.0);

    f1.walls = vec![ws, wn, ww, we, w_b, w_f, w_e, w_e2];

    // 開口
    f1.openings.push(Opening::window(ws_id, 3000.0, 3600.0, 2000.0, 400.0));
    f1.openings.push(Opening::window(ws_id, 9000.0, 2400.0, 1200.0, 800.0));
    f1.openings.push(Opening::window(wn_id, 3000.0, 1600.0, 1200.0, 800.0));
    f1.openings.push(Opening::window(wn_id, 9000.0, 1600.0, 1200.0, 800.0));
    f1.openings.push(Opening::door(ww_id, 5000.0, 900.0, 2100.0));
    f1.openings.push(Opening::window(we_id, 5000.0, 1600.0, 1200.0, 800.0));

    // 部屋
    let mut living = Room::new("リビング", vec![
        Point2D::new(6523.0, 0.0), Point2D::new(12857.0, 0.0),
        Point2D::new(12857.0, 2875.0), Point2D::new(6523.0, 2875.0),
    ]);
    living.has_floor_heating = true;

    let mut dk = Room::new("DK", vec![
        Point2D::new(0.0, 0.0), Point2D::new(6523.0, 0.0),
        Point2D::new(6523.0, 2875.0), Point2D::new(0.0, 2875.0),
    ]);
    dk.has_floor_heating = true;

    let mut main_room = Room::new("メインルーム", vec![
        Point2D::new(0.0, 2875.0), Point2D::new(12857.0, 2875.0),
        Point2D::new(12857.0, 8957.0), Point2D::new(0.0, 8957.0),
    ]);
    main_room.has_floor_heating = true;

    let utility = Room::new("ユーティリティ", vec![
        Point2D::new(0.0, 8957.0), Point2D::new(6523.0, 8957.0),
        Point2D::new(6523.0, 10680.0), Point2D::new(0.0, 10680.0),
    ]);

    let bedroom = Room::new("寝室", vec![
        Point2D::new(6523.0, 8957.0), Point2D::new(12857.0, 8957.0),
        Point2D::new(12857.0, 10680.0), Point2D::new(6523.0, 10680.0),
    ]);

    f1.rooms = vec![living, dk, main_room, utility, bedroom];
    bldg.add_floor(f1);
    bldg
}

fn main() {
    let bldg = build_yamanakako();

    // 新しい位置に描画（既存図面と重ならないように）
    let renderer = AcadRenderer::with_origin(Point2D::new(20000.0, 0.0));

    if !renderer.is_connected() {
        eprintln!("AutoCAD に接続できません。プラグインを確認してください。");
        std::process::exit(1);
    }

    println!("AutoCAD に描画中...");
    match renderer.render_building(&bldg) {
        Ok(report) => {
            println!("完了！ {report}");
            println!("AutoCAD で ZOOM E して確認してください。");
        }
        Err(e) => eprintln!("エラー: {e}"),
    }
}
