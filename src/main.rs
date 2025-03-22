mod job;
mod queue;
mod worker;

use crate::{job::Job, queue::Queue, worker::Worker};
use axum::{routing::{get, post}, Json, Router};
use clap::Parser;
use derive_more::{Display, FromStr};
use serde_json::json;
use std::{net::{Ipv6Addr, SocketAddr}, sync::Arc};
use tokio::{net::TcpListener, sync::Mutex};
use tower_http::{services::ServeDir, trace::TraceLayer};
use tracing::info;

/// The available queue implementations chosen via the command line.
#[derive(Debug, Clone, Copy, Display, FromStr)]
#[non_exhaustive]
enum QueueMode {
    /// An in-memory queue, backed by a `VecDeque`.
    InMemory,
    /// A queue that reads from and writes to a JSON file on every operation.
    JsonFile,
    /// A queue that writes to a JSON file on every operation, but caches the entire queue in memory.
    CachedJsonFile,
}

#[derive(Debug, Clone, Copy, clap::Parser)]
struct Args {
    /// The TCP port on which the server will listen.
    #[clap(short, long, default_value_t = 2567)]
    port: u16,
    /// The queue implementation to use for both the worker and job queues.
    /// Possible values are `InMemory`, `JsonFile`, and `CachedJsonFile`.
    #[clap(short, long, default_value_t = QueueMode::CachedJsonFile)]
    mode: QueueMode,
    /// The queue implementation to use for the job queue.
    /// If not specified, the mode will be used.
    #[clap(long)]
    job_queue_mode: Option<QueueMode>,
    /// The queue implementation to use for the worker queue.
    /// If not specified, the mode will be used.
    #[clap(long)]
    worker_queue_mode: Option<QueueMode>,
}

/// The application state.
/// The queues are wrapped in Arc<Mutex<_>> to allow synchronized multithreaded mutation.
#[derive(Debug, Clone)]
struct AppState {
    http_client: reqwest::Client,
    worker_queue: Arc<Mutex<Queue<Worker>>>,
    job_queue: Arc<Mutex<Queue<Job>>>,
}

#[tokio::main]
async fn main() {
    // Initialize the logger.
    tracing_subscriber::fmt::init();

    // Parse the command-line arguments.
    let args = Args::parse();
    let port = args.port;
    let job_queue = match args.job_queue_mode.unwrap_or(args.mode) {
        QueueMode::InMemory => queue::InMemoryQueue::new().into(),
        QueueMode::JsonFile => queue::JsonFileQueue::new("jobs.json").into(),
        QueueMode::CachedJsonFile => queue::CachedJsonFileQueue::new("jobs.json").await.into(),
    };
    let worker_queue = match args.worker_queue_mode.unwrap_or(args.mode) {
        QueueMode::InMemory => queue::InMemoryQueue::new().into(),
        QueueMode::JsonFile => queue::JsonFileQueue::new("workers.json").into(),
        QueueMode::CachedJsonFile => queue::CachedJsonFileQueue::new("workers.json").await.into(),
    };

    // Create the application state for the handlers to use.
    let state = AppState {
        http_client: reqwest::Client::new(),
        job_queue: Arc::new(Mutex::new(job_queue)),
        worker_queue: Arc::new(Mutex::new(worker_queue)),
    };

    // Generate the contents of the public/config.json file.
    let config = json!({
        "server_port": port,
    });

    // Create the application routes.
    let app = Router::new()
        .route("/register-worker", post(worker::register_worker))
        .route("/submit-job", post(job::submit_job))
        .with_state(state)
        .route(
            "/public/config.json",
            get(move || async move { Json(config.clone()) }),
        )
        .nest_service("/public", ServeDir::new("public"))
        .layer(TraceLayer::new_for_http());

    // Listen over TCP on the specified port.
    let addr = SocketAddr::from((Ipv6Addr::UNSPECIFIED, port));
    let listener = TcpListener::bind(addr).await.unwrap();
    info!("Queue service running on {addr}");
    axum::serve(listener, app).await.unwrap();
}
