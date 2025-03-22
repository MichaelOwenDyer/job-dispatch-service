# Job Dispatcher Service

## Introduction

`job-dispatcher-service` is a Rust-based application designed to asynchronously assign jobs (arbitrary JSON data) 
to workers (CPEE process engine instances) for processing. Waiting jobs and workers are stored in FIFO queues.
There are multiple queue implementations available, including in-memory and file-based options.

## Scenario

The prototypical domain for the Job Dispatcher Service is the drink ordering system.
In this scenario, customers can place orders for drinks, which are then prepared by an automated robot bartender.

1. A CPEE process engine instance is started for each robot bartender. The instances call the service to request a job, and wait for a job to be assigned.
2. The customer visits the web interface and types in their name, logo, and drink order, then clicks the "Order" button.
3. The service receives the job and assigns it to the next available worker.
4. The worker receives the job and prepares the drink according to the customer's specifications.
5. The customer receives the drink with their name and logo on it and is happy.
6. The worker is ready for the next job. Any jobs which were placed while the worker was busy are queued up and processed in order.

## Features

- Multiple queue implementations: InMemory, JsonFile, and CachedJsonFile.
- Asynchronous processing using Tokio.
- Web service capabilities with Axum.
- Command-line interface using Clap.
- Tracing and logging with Tracing and Tracing Subscriber.

## Installation

To build and run the Job Dispatcher Service, ensure you have Rust and Cargo installed. Then, clone the repository and build the project:

```bash
# Clone the repository
git clone https://github.com/MichaelOwenDyer/job-dispatch-service
cd queue-service

# Build the project
cargo build --release
```

## Usage

Run the application with the following command:

```bash
cargo run --release -- --port <port> --mode <mode>
```

The application can be customized with the following flags:

- `--port`: The TCP port on which the server will listen (default: 2567).
- `--mode`: The queue implementation to use for the queues. Possible values are:
    - `InMemory`: Non-persistent.
    - `JsonFile`: Queues are written into the json files `workers.json` and `jobs.json`.
    - `CachedJsonFile`: Like `JsonFile` but the content of the file is cached in memory.

Optionally, you may use the options `--worker-queue-mode` and `--job-queue-mode` to choose a different queue implementation for
the worker queue and the job queue, respectively.

Example:

```bash
cargo run --release -- --port 8080 --mode InMemory
```

Tip: Remove the `--release` flag for faster build times at the expense of less optimization.
