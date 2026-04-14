/// 山中湖別荘 B-1 を DXF に出力（UTF-8 + Shift-JIS 両方）
use cad_core::*;
use cad_dxf::{DxfEncoding, DxfExporter};
use std::fs::File;
use std::io::BufWriter;

fn build_yamanakako() -> Building {
    let mut bldg = Building::new("山中湖別荘 B-1");

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

    let mut ws = Wall::new(Point2D::new(0.0, 0.0), Point2D::new(12857.0, 0.0), 150.0);
    ws.is_exterior = true;
    let ws_id = ws.id;
    let mut wn = Wall::new(
        Point2D::new(0.0, 10680.0),
        Point2D::new(12857.0, 10680.0),
        150.0,
    );
    wn.is_exterior = true;
    let wn_id = wn.id;
    let mut ww = Wall::new(Point2D::new(0.0, 0.0), Point2D::new(0.0, 10680.0), 150.0);
    ww.is_exterior = true;
    let ww_id = ww.id;
    let mut we = Wall::new(
        Point2D::new(12857.0, 0.0),
        Point2D::new(12857.0, 10680.0),
        150.0,
    );
    we.is_exterior = true;
    let we_id = we.id;

    let w_b = Wall::new(
        Point2D::new(6523.0, 0.0),
        Point2D::new(6523.0, 2875.0),
        80.0,
    );
    let w_f = Wall::new(
        Point2D::new(0.0, 2875.0),
        Point2D::new(12857.0, 2875.0),
        80.0,
    );
    let w_e = Wall::new(
        Point2D::new(0.0, 8957.0),
        Point2D::new(6523.0, 8957.0),
        80.0,
    );
    let w_e2 = Wall::new(
        Point2D::new(6523.0, 8957.0),
        Point2D::new(12857.0, 8957.0),
        80.0,
    );

    f1.walls = vec![ws, wn, ww, we, w_b, w_f, w_e, w_e2];

    f1.openings
        .push(Opening::window(ws_id, 3000.0, 3600.0, 2000.0, 400.0));
    f1.openings
        .push(Opening::window(ws_id, 9000.0, 2400.0, 1200.0, 800.0));
    f1.openings
        .push(Opening::window(wn_id, 3000.0, 1600.0, 1200.0, 800.0));
    f1.openings
        .push(Opening::window(wn_id, 9000.0, 1600.0, 1200.0, 800.0));
    f1.openings
        .push(Opening::door(ww_id, 5000.0, 900.0, 2100.0));
    f1.openings
        .push(Opening::window(we_id, 5000.0, 1600.0, 1200.0, 800.0));

    f1.rooms = vec![
        Room::new(
            "リビング",
            vec![
                Point2D::new(6523.0, 0.0),
                Point2D::new(12857.0, 0.0),
                Point2D::new(12857.0, 2875.0),
                Point2D::new(6523.0, 2875.0),
            ],
        ),
        Room::new(
            "DK",
            vec![
                Point2D::new(0.0, 0.0),
                Point2D::new(6523.0, 0.0),
                Point2D::new(6523.0, 2875.0),
                Point2D::new(0.0, 2875.0),
            ],
        ),
        Room::new(
            "メインルーム",
            vec![
                Point2D::new(0.0, 2875.0),
                Point2D::new(12857.0, 2875.0),
                Point2D::new(12857.0, 8957.0),
                Point2D::new(0.0, 8957.0),
            ],
        ),
        Room::new(
            "ユーティリティ",
            vec![
                Point2D::new(0.0, 8957.0),
                Point2D::new(6523.0, 8957.0),
                Point2D::new(6523.0, 10680.0),
                Point2D::new(0.0, 10680.0),
            ],
        ),
        Room::new(
            "寝室",
            vec![
                Point2D::new(6523.0, 8957.0),
                Point2D::new(12857.0, 8957.0),
                Point2D::new(12857.0, 10680.0),
                Point2D::new(6523.0, 10680.0),
            ],
        ),
    ];

    bldg.add_floor(f1);
    bldg
}

fn main() {
    let bldg = build_yamanakako();

    // UTF-8 版（AutoCAD 向け）
    {
        let file = File::create("/tmp/yamanakako-b1-utf8.dxf").unwrap();
        let mut buf = BufWriter::new(file);
        let exporter = DxfExporter::for_autocad();
        let report = exporter.export(&bldg, &mut buf).unwrap();
        println!("UTF-8:     /tmp/yamanakako-b1-utf8.dxf — {report}");
    }

    // Shift-JIS 版（Jw_cad 向け）
    {
        let file = File::create("/tmp/yamanakako-b1-sjis.dxf").unwrap();
        let mut buf = BufWriter::new(file);
        let exporter = DxfExporter::for_jwcad();
        let report = exporter.export(&bldg, &mut buf).unwrap();
        println!("Shift-JIS: /tmp/yamanakako-b1-sjis.dxf — {report}");
    }

    // ファイルサイズ確認
    let utf8_size = std::fs::metadata("/tmp/yamanakako-b1-utf8.dxf")
        .unwrap()
        .len();
    let sjis_size = std::fs::metadata("/tmp/yamanakako-b1-sjis.dxf")
        .unwrap()
        .len();
    println!("\nFile sizes: UTF-8={utf8_size} bytes, Shift-JIS={sjis_size} bytes");
}
