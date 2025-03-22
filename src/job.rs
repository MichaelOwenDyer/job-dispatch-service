//! Job submission and processing.

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{error, info};
use uuid::Uuid;
use crate::AppState;
use crate::worker::Worker;

/// A job to be processed by a worker.
#[derive(Debug, Serialize, Deserialize)]
pub struct Job {
    /// The unique identifier of the job.
    pub id: Uuid,
    /// The data to be processed. This can be any JSON value.
    pub data: Value,
    /// The time at which the job was submitted.
    pub submitted_at: DateTime<Utc>,
}

impl Job {
    /// Creates a new job with the given data.
    pub fn new(data: Value) -> Self {
        Self {
            id: Uuid::new_v4(),
            data,
            submitted_at: Utc::now(),
        }
    }
}

/// The response to a job submission request.
/// If a worker was immediately available, the response is Assigned.
/// If no workers were available, the response is Queued with the job's position in the queue.
#[derive(Debug, Serialize)]
pub enum SubmitJobResponse {
    /// A worker was assigned the job, and it is being processed.
    Assigned,
    /// No workers were available, and the job has been queued.
    /// The job's position in the queue is provided.
    Queued { position: usize },
}

/// An asynchronous response sent to a worker.
/// The only variant is Job, which contains the job to be processed.
/// The primary purpose of this enum is for API consistency with
/// [`RegisterWorkerResponse`](crate::worker::RegisterWorkerResponse),
/// which also wraps returned Jobs in a "Job" object.
#[derive(Debug, Serialize)]
pub enum AsynchronousWorkerResponse<'a> {
    Job(&'a Job)
}

/// POST /submit-job
/// Submits a job to be processed by a worker.
/// The job is sent to the workers in the worker queue in the order they are dequeued.
/// The first worker to return a 2xx status code is assigned the job, and this endpoint
/// responds with 200 Ok and "Assigned".
/// If no workers are available, the job is queued and this endpoint responds with
/// 202 Accepted and "Queued" along with the job's position in the queue.
#[rustfmt::skip]
pub async fn submit_job(
    State(state): State<AppState>,
    Json(data): Json<Value>
) -> (StatusCode, Json<SubmitJobResponse>) {
    let job = Job::new(data);
    loop {
        let Worker {
            callback_url,
            registered_at
        } = match state.worker_queue.lock().await.dequeue().await {
            Some(worker) => worker,
            None => break,
        };
        let queue_time = Utc::now().signed_duration_since(registered_at).num_seconds();
        match state.http_client.put(&callback_url).json(&AsynchronousWorkerResponse::Job(&job)).send().await {
            Err(err) => {
                // Something went wrong while sending the request (redirect loop, timeout, etc.)
                error!("Failed to send job to worker at {callback_url}: '{err}', discarding... (was queued for {queue_time}s)");
                continue;
            },
            Ok(response) if !response.status().is_success() => {
                let status = response.status();
                error!("Worker at {callback_url} responded to job assignment with non-2xx code ({status}), discarding... (was queued for {queue_time}s)");
                continue;
            },
            Ok(_) => {
                info!("Job submission received. Assigning to worker at {callback_url} (was queued for {queue_time}s)");
                return (StatusCode::OK, Json(SubmitJobResponse::Assigned));
            },
        };
    }
    info!("Job submission received. No workers available, queueing...");
    let queue_size = state.job_queue.lock().await.enqueue(job).await;
    (StatusCode::ACCEPTED, Json(SubmitJobResponse::Queued { position: queue_size }))
}
