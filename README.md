# NetViz

> High-performance Network Visualization Application built with Rust

## Overview

NetViz is a web application that visualizes network data from PeeringDB. It provides a dashboard for network statistics, a searchable list of networks, and analytics charts.

It is built with **Rust** using the **Axum** framework for superior performance and type safety.

## Features

- **Dashboard**: Overview of network types, policies, and geographic scopes.
- **Search**: Fast filtering by ASN or Network Name.
- **Analytics**: correlation charts and distribution metrics.
- **Automated Data Fetching**: Automatically downloads and caches data from PeeringDB.

## Tech Stack

- **Backend**: Rust, Axum (Web Framework), Tokio (Async Runtime)
- **Frontend**: Tera (Templates), Bootstrap 5, Chart.js
- **Data**: Serde (JSON), Reqwest (HTTP Client)
- **Deployment**: Docker (Multi-stage build)

## Prerequisites

- [Rust & Cargo](https://rustup.rs/) (v1.92+)
- [Docker](https://www.docker.com/) (Optional, for containerized run)

## Quick Start

### Running Locally

1. **Clone the repository:**

   ```bash
   git clone https://github.com/crazyuploader/NetViz.git
   cd NetViz
   ```

2. **Run with Cargo:**

   ```bash
   cargo run
   ```

   The server will start at `http://0.0.0.0:8201`.
   On the first run, it will automatically fetch data from PeeringDB (this may take a minute).

3. **(Optional) Set API Key:**
   To avoid rate limits, set your PeeringDB API key:
   ```bash
   export PEERINGDB_API_KEY="your_key_here"
   cargo run
   ```

### Running with Docker

1. **Build and Run:**

   ```bash
   docker compose up --build
   ```

2. **Access:**
   Open [http://localhost:8201](http://localhost:8201) in your browser.

## Project Structure

- `src/main.rs`: Application entry point and web server routes.
- `src/fetcher.rs`: PeeringDB API data fetching logic.
- `src/models.rs`: Data structures (structs) for Networks and Stats.
- `src/data.rs`: JSON data loading and parsing.
- `templates/`: HTML templates (Tera).
- `data/`: Local data storage.

## Development

- **Check code:** `cargo check`
- **Run tests:** `cargo test`
- **Format code:** `cargo fmt`
- **Lint code:** `cargo clippy`

## License

[MIT](LICENSE)
