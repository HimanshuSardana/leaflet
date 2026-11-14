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
        data_dir: dir,
        log: "error".into(),
    };
    let pool = leaflet::db::open(&config).await.unwrap();
    AppState { config, pool }
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
async fn unknown_route_is_404() {
    let response = router(state().await)
        .oneshot(Request::get("/nope").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
