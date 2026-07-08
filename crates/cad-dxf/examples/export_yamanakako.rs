/// 山中湖別荘 B-1 を DXF に出力（UTF-8 + Shift-JIS 両方）
use cad_core::*;
use cad_dxf::DxfExporter;
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

    let ws_id = {
        let w = f1.add_wall_mut(Point2D::new(0.0, 0.0), Point2D::new(12857.0, 0.0), 150.0);
        w.is_exterior = true;
        w.id
    };
    let wn_id = {
        let w = f1.add_wall_mut(
            Point2D::new(0.0, 10680.0),
            Point2D::new(12857.0, 10680.0),
            150.0,
        );
        w.is_exterior = true;
        w.id
    };
    let ww_id = {
        let w = f1.add_wall_mut(Point2D::new(0.0, 0.0), Point2D::new(0.0, 10680.0), 150.0);
        w.is_exterior = true;
        w.id
    };
    let we_id = {
        let w = f1.add_wall_mut(
            Point2D::new(12857.0, 0.0),
            Point2D::new(12857.0, 10680.0),
            150.0,
        );
        w.is_exterior = true;
        w.id
    };

    f1.add_wall(
        Point2D::new(6523.0, 0.0),
        Point2D::new(6523.0, 2875.0),
        80.0,
    );
    f1.add_wall(
        Point2D::new(0.0, 2875.0),
        Point2D::new(12857.0, 2875.0),
        80.0,
    );
    f1.add_wall(
        Point2D::new(0.0, 8957.0),
        Point2D::new(6523.0, 8957.0),
        80.0,
    );
    f1.add_wall(
        Point2D::new(6523.0, 8957.0),
        Point2D::new(12857.0, 8957.0),
        80.0,
    );

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
        // 重心: (6523,0),(12857,0),(12857,2875),(6523,2875) → (9690, 1437.5)
        Room::new("リビング", Point2D::new(9690.0, 1437.5)),
        // 重心: (0,0),(6523,0),(6523,2875),(0,2875) → (3261.5, 1437.5)
        Room::new("DK", Point2D::new(3261.5, 1437.5)),
        // 重心: (0,2875),(12857,2875),(12857,8957),(0,8957) → (6428.5, 5916)
        Room::new("メインルーム", Point2D::new(6428.5, 5916.0)),
        // 重心: (0,8957),(6523,8957),(6523,10680),(0,10680) → (3261.5, 9818.5)
        Room::new("ユーティリティ", Point2D::new(3261.5, 9818.5)),
        // 重心: (6523,8957),(12857,8957),(12857,10680),(6523,10680) → (9690, 9818.5)
        Room::new("寝室", Point2D::new(9690.0, 9818.5)),
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
