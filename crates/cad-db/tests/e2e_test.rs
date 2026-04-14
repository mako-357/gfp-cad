use cad_core::*;
use cad_db::CadDbClient;

fn config() -> cad_db::DbConfig {
    cad_db::DbConfig {
        url: "http://localhost:12000".into(),
        namespace: "gfp".into(),
        database: "cad".into(),
        username: "admin".into(),
        password: "admin-local-dev".into(),
    }
}

#[tokio::test]
async fn test_full_workflow() {
    let db = CadDbClient::connect(&config()).await.expect("DB connect");
    db.init_schema().await.expect("Schema init");

    // 1. ユーザー作成
    let user = db
        .upsert_user_by_auth("google-oauth2", "e2e-test-001", "e2e@example.com", Some("E2E Tester"), None)
        .await
        .expect("Create user");
    assert_eq!(user.email, "e2e@example.com");
    assert!(user.id.is_some());
    println!("User: {:?}", user.id);

    // 2. 再ログイン
    let user2 = db
        .upsert_user_by_auth("google-oauth2", "e2e-test-001", "e2e@example.com", Some("E2E Tester"), None)
        .await
        .expect("Re-login");
    assert_eq!(user2.email, "e2e@example.com");

    // 3. ワークスペース一覧
    let workspaces = db.list_workspaces(&user).await.expect("List workspaces");
    println!("Workspaces: {}", workspaces.len());
    assert!(!workspaces.is_empty(), "Default workspace should exist");
    let ws_id = workspaces[0].id.as_ref().expect("Workspace ID");

    // 4. プロジェクト作成
    let project = db.create_project(ws_id, "E2E Test Project", &user).await.expect("Create project");
    let proj_id = project.id.as_ref().expect("Project ID");

    // 5. Building 保存
    let mut bldg = Building::new("E2E House");
    bldg.grid.x_axes = vec![GridAxis::new("A", 0.0), GridAxis::new("B", 6000.0)];
    let mut floor = Floor::new("1F", 200.0, 3000.0);
    floor.walls.push(Wall::new(Point2D::new(0.0, 0.0), Point2D::new(6000.0, 0.0), 150.0));
    floor.rooms.push(Room::new("リビング", vec![
        Point2D::new(0.0, 0.0), Point2D::new(6000.0, 0.0),
        Point2D::new(6000.0, 5000.0), Point2D::new(0.0, 5000.0),
    ]));
    bldg.add_floor(floor);

    let record = db.save_building(proj_id, &bldg, &user).await.expect("Save building");
    let bldg_id = record.id.as_ref().expect("Building ID");
    println!("Saved: {:?}", bldg_id);

    // 6. Building 一覧
    let buildings = db.list_buildings(proj_id).await.expect("List buildings");
    assert!(!buildings.is_empty());

    // 7. Building 読み込み
    let loaded = db.load_building(bldg_id).await.expect("Load building");
    assert_eq!(loaded.name, "E2E House");
    assert_eq!(loaded.floors.len(), 1);
    assert_eq!(loaded.floors[0].walls.len(), 1);
    assert_eq!(loaded.floors[0].rooms[0].name, "リビング");
    let area = loaded.total_floor_area();
    assert!((area - 30.0).abs() < 0.1, "Area should be 30 sqm, got {area}");
    println!("Loaded: {} — {:.1} sqm", loaded.name, area);

    // クリーンアップ
    db.db.query("DELETE buildings; DELETE projects; DELETE workspace_member; DELETE workspaces; DELETE auth_identities; DELETE users")
        .await.ok();
    println!("=== E2E PASSED ===");
}
