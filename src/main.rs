use std::sync::Arc;
use axum::{
    Router,
    extract::{Json, State},
    response::{Html, IntoResponse},
    routing::{get, post},
};
use rig::{
    completion::Prompt,
    providers::openai::{self, GPT_4},
};
use rig_arxiv_agent::arxiv::tools::{self, ArxivSearchTool, Paper};
use serde::Deserialize;
use tower_http::cors::{Any, CorsLayer};
use std::net::SocketAddr;
use tokio::net::TcpListener;


// Request structure for search endpoint
#[derive(Deserialize)]
struct SearchRequest {
    query: String,
}

// Custom error type for the application
struct AppError(anyhow::Error);

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Something went wrong: {}", self.0),
        )
            .into_response()
    }
}

impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}

// State structure to hold shared data
struct AppState {
    openai_client: openai::Client,
}

// Handler for serving the static index.html
async fn serve_index() -> impl IntoResponse {
    Html(include_str!("../static/index.html"))
}

// Handler for the search endpoint
async fn search_papers(
    State(state): State<Arc<AppState>>,
    Json(request): Json<SearchRequest>,
) -> Result<impl IntoResponse, AppError> {
    let paper_agent = state.openai_client
        .agent(GPT_4)
        .preamble(
            "You are a helpful research assistant that can search and analyze academic papers from arXiv. \
             When asked about a research topic, use the search_arxiv tool to find relevant papers and \
             return only the raw JSON response from the tool."
        )
        .tool(ArxivSearchTool)
        .build();

    let response = paper_agent.prompt(&request.query).await?;

    let papers: Vec<Paper> = serde_json::from_str(&response)?;

    // Format the papers into HTML table
    let html = tools::format_papers_as_html(&papers)?;
    Ok(Html(html))
}

#[tokio::main]
async fn main() {
    // 初始化日志
    tracing_subscriber::fmt::init();

    // Initialize OpenAI client from env
    let openai_client = openai::Client::from_env();

    // Create shared state
    let state = Arc::new(AppState {
        openai_client,
    });

    // Set up CORS
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([axum::http::Method::GET, axum::http::Method::POST])
        .allow_headers(Any);

    // Create router
    let router = Router::new()
        .route("/", get(serve_index))
        .route("/api/search", post(search_papers))
        .layer(cors)
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3333));
    let tcp = TcpListener::bind(&addr).await.unwrap();
    tracing::info!("listening on {}", addr);

    axum::serve(tcp, router.into_make_service())
        .await
        .expect("unable to start server");
}
