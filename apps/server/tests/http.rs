use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use leaflet::{AppState, config::Config, router};
use tower::ServiceExt;

async fn state() -> AppState {
    let dir = std::env::temp_dir().join(format!("leaflet-http-{}", uuid::Uuid::now_v7()));
    let config = Config {
        bind: "127.0.0.1:0".parse().unwrap(),
        database_url: format!("sqlite://{}?mode=rwc", dir.join("leaflet.db").display()),
        data_dir: dir.clone(),
        web_dir: dir.join("web"),
        log: "error".into(),
    };
    let pool = leaflet::db::open(&config).await.unwrap();
    let storage = leaflet::storage::Storage::new(config.blobs_dir()).unwrap();
    let assets = leaflet::assets::Assets::new(&config.web_dir);
    AppState {
        config,
        pool,
        storage,
        assets,
    }
}

async fn get(path: &str) -> (StatusCode, serde_json::Value) {
    let response = router(state().await)
        .oneshot(Request::get(path).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (status, serde_json::from_slice(&bytes).unwrap())
}

#[tokio::test]
async fn healthz_reports_ok() {
    let (status, body) = get("/healthz").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");
}

#[tokio::test]
async fn version_reports_api_v1() {
    let (status, body) = get("/api/v1/version").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["api"], "v1");
    assert_eq!(body["name"], "leaflet-server");
}

#[tokio::test]
async fn api_paths_never_fall_through_to_the_spa() {
    let response = router(state().await)
        .oneshot(Request::get("/api/v1/nope").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn built_assets_are_served_from_the_root() {
    let state = state().await;
    std::fs::create_dir_all(&state.config.web_dir).unwrap();
    std::fs::write(state.config.web_dir.join("index.html"), "<h1>leaflet</h1>").unwrap();
    let response = router(state.clone())
        .oneshot(Request::get("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    assert!(String::from_utf8_lossy(&body).contains("leaflet"));
    std::fs::remove_dir_all(&state.config.data_dir).ok();
}

#[tokio::test]
async fn unknown_route_is_404() {
    let response = router(state().await)
        .oneshot(Request::get("/nope").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
