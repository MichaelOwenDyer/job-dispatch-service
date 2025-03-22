//! Worker registration and job assignment

use axum::extract::{Request, State};
use axum::http::StatusCode;
use axum::Json;
use axum::response::{IntoResponse, Response};
use chrono::{DateTime, Utc};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use tracing::{error, info};
use crate::AppState;
use crate::job::Job;

/// A worker that can process jobs.
#[derive(Debug, Serialize, Deserialize)]
pub struct Worker {
    /// The URL to which a job should be sent.
    pub callback_url: String,
    /// The time at which the worker was registered.
    pub registered_at: DateTime<Utc>,
}

impl Worker {
    /// Creates a new worker with the given callback URL.
    pub fn new(callback_url: impl Into<String>) -> Self {
        Self {
            callback_url: callback_url.into(),
            registered_at: Utc::now(),
        }
    }
}

/// An error that can occur when registering a worker.
#[derive(Debug, Serialize)]
pub enum CallbackHeaderError {
    /// The CPEE-CALLBACK header was missing from the request.
    Missing,
    /// The CPEE-CALLBACK header was not a valid string (HTTP headers can technically be any bytes).
    NotAString,
    /// The CPEE-CALLBACK header was not a valid URL.
    NotAUrl,
}

/// The response to a worker registration request.
#[derive(Debug, Serialize)]
pub enum RegisterWorkerResponse {
    /// No jobs were available and the worker was queued.
    Queued,
    /// A queued job was immediately available and returned.
    Job(Job),
    /// An error occurred when processing the request.
    Error(CallbackHeaderError),
}

/// Attempts to extract the callback URL from the request headers.
fn extract_callback_header(request: &Request) -> Result<Url, CallbackHeaderError> {
    request.headers()
        .get("cpee-callback").ok_or_else(|| {
            error!("Worker registration failed: CPEE-CALLBACK header was missing");
            CallbackHeaderError::Missing
        })
        .and_then(|header| header.to_str().map_err(|err| {
            error!("Worker registration failed: CPEE-CALLBACK header was not a valid string: {err}");
            CallbackHeaderError::NotAString
        }))
        .and_then(|header| Url::parse(header).map_err(|err| {
            error!("Worker registration failed: CPEE-CALLBACK header was not a valid URL: {err}");
            CallbackHeaderError::NotAUrl
        }))
}

/// POST /register-worker
/// Tells the server that a worker is ready to receive a job.
///
/// The worker must provide a CPEE-CALLBACK header with a valid URL in case there are no jobs
/// immediately available. If the header is missing, not a string, or not a valid URL,
/// the request is rejected with a 400 Bad Request status and an error message.
///
/// If a queued job is immediately available, it is returned with a 200 OK status.
/// If no jobs are immediately available, the worker is queued and a 202 Accepted status is returned
/// with the CPEE-CALLBACK header set to true. This indicates that the job will be sent to the worker
/// at a later time using the provided callback URL.
#[rustfmt::skip]
pub async fn register_worker(
    State(state): State<AppState>,
    request: Request
) -> Response {
    let callback_url = match extract_callback_header(&request) {
        Ok(callback_url) => callback_url,
        Err(err) => {
            return (StatusCode::BAD_REQUEST, Json(RegisterWorkerResponse::Error(err))).into_response();
        }
    };
    if let Some(job) = state.job_queue.lock().await.dequeue().await {
        let queue_time = Utc::now().signed_duration_since(job.submitted_at).num_seconds();
        info!("Worker registration received ({callback_url}). Assigning job... (was queued for {queue_time}s)");
        (StatusCode::OK, Json(RegisterWorkerResponse::Job(job))).into_response()
    } else {
        // Set the cpee-callback header to true to indicate that the job will be returned asynchronously
        info!("Worker registration received ({callback_url}). No jobs available, queuing...");
        state.worker_queue.lock().await.enqueue(Worker::new(callback_url)).await;
        (StatusCode::ACCEPTED, [("cpee-callback", "true")], Json(RegisterWorkerResponse::Queued)).into_response()
    }
}
