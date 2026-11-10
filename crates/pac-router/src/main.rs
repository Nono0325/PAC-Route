use axum::{
    routing::{get, post},
    Router,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::info;

mod handlers;
mod state;

use handlers::{chat_completions_handler, cluster_nodes_handler, health_handler, list_models_handler};
use pac_discovery::{LanDiscoveryService, NodeRegistry};
use state::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. 初始化日誌輸出
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "pac_router=info,pac_discovery=info,pac_scheduler=info".into()),
        )
        .init();

    info!("===========================================================");
    info!("🚀 PAC-Route: 個人異質 AI 叢集之隱私約束調度與兩平面架構");
    info!("   版本: v{}", pac_core::PAC_ROUTE_VERSION);
    info!("   作者: 陳有臨 (You-Lin Chen) / 國立虎尾科技大學 資工所");
    info!("===========================================================");

    // 2. 初始化叢集註冊表並預載示範異質節點 (RTX 4090, RTX 3060, Raspberry Pi 5, Cloud Proxy)
    let registry = NodeRegistry::new();
    registry.seed_sample_cluster().await;
    info!("✅ 預設異質個人叢集節點已載入完畢");

    // 3. 啟動背景 mDNS / 局域網自動廣播監聽任務
    let discovery_service = Arc::new(LanDiscoveryService::new(registry.clone(), 5353));
    tokio::spawn(async move {
        discovery_service.start_listener().await;
    });

    // 4. 定期維護示範節點心跳與清理離線節點
    let heartbeat_registry = registry.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(5));
        loop {
            interval.tick().await;
            heartbeat_registry.update_heartbeat("node_workstation_rtx4090", 0, 18432, 48000).await;
            heartbeat_registry.update_heartbeat("node_edge_rtx3060", 1, 8192, 22000).await;
            heartbeat_registry.update_heartbeat("node_sbc_rpi5", 0, 0, 5120).await;
            heartbeat_registry.update_heartbeat("node_cloud_proxy", 0, 80000, 512000).await;
        }
    });

    let cleaner_registry = registry.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(15));
        loop {
            interval.tick().await;
            cleaner_registry.clean_stale_nodes(60).await;
        }
    });

    let app_state = AppState::new(registry);

    // 5. 構建 Axum 路由，提供完全相容 OpenAI 與本地 AI 工具之端點
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/health", get(health_handler))
        .route("/cluster/nodes", get(cluster_nodes_handler))
        .route("/v1/models", get(list_models_handler))
        .route("/v1/chat/completions", post(chat_completions_handler))
        // Ollama 相容別名
        .route("/api/chat", post(chat_completions_handler))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(app_state);

    let port = pac_core::DEFAULT_ROUTER_PORT;
    let bind_addr: SocketAddr = format!("0.0.0.0:{}", port).parse()?;
    info!("🌐 PAC-Route 閘道器已啟動於 http://{}", bind_addr);
    info!("   OpenAI 端點: http://{}/v1/chat/completions", bind_addr);
    info!("   叢集節點端點: http://{}/cluster/nodes", bind_addr);

    let listener = tokio::net::TcpListener::bind(bind_addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

