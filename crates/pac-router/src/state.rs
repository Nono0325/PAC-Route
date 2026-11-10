use std::sync::Arc;
use pac_discovery::NodeRegistry;
use pac_privacy::SemanticSketchPipeline;
use pac_scheduler::DuelingDdqnScheduler;

#[derive(Clone)]
pub struct AppState {
    pub registry: NodeRegistry,
    pub scheduler: Arc<DuelingDdqnScheduler>,
    pub sketch_pipeline: Arc<SemanticSketchPipeline>,
    #[allow(dead_code)]
    pub http_client: reqwest::Client,
}

impl AppState {
    pub fn new(registry: NodeRegistry) -> Self {
        Self {
            registry,
            scheduler: Arc::new(DuelingDdqnScheduler::new()),
            sketch_pipeline: Arc::new(SemanticSketchPipeline::default()),
            http_client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_default(),
        }
    }
}
