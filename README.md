# gRPC Hub

A gRPC hub server that acts as a central registry for other gRPC services. Services can register themselves with the hub, and the hub provides both gRPC and HTTP APIs to discover and monitor registered services.

## Features

- **Service Registration**: Services can register themselves with metadata, methods, and health status
- **Service Discovery**: Query registered services with filtering capabilities
- **Health Monitoring**: Track service health with heartbeat mechanisms
- **Web Interface**: Beautiful web UI to view all registered services
- **gRPC Reflection**: Full gRPC reflection support for service introspection
- **HTTP API**: RESTful API for web-based service discovery

## Quick Start

### 1. Build and Run the Hub Server

```bash
# Build the project
cargo build --release

# Run the hub server (default: gRPC on :50099, HTTP on :8080)
cargo run --release

# Or with custom ports
cargo run --release -- --grpc-port 50099 --http-port 8080
```

### 2. View the Web Interface

Open your browser and navigate to `http://localhost:8080` to see the web interface showing all registered services.

### 3. Test with Demo Clients

```bash
# Run the simple test client
cargo run --release --bin test_client

# Run the comprehensive demo client
cargo run --release --bin demo_client
```

## Complete Example: Web Content Extract and Dividend Service

This example demonstrates a complete workflow where the Dividend Service fetches data from the Web Content Extract Service through the gRPC Hub.

### Step 1: Start the gRPC Hub

```bash
# Terminal 1: Start the hub server
cargo run --release
```

You should see output like:
```
🚀 gRPC Hub Server starting...
📡 gRPC server listening on 0.0.0.0:50099
🌐 HTTP server listening on 0.0.0.0:8080
```

### Step 2: Start the Web Content Extract Service

```bash
# Terminal 2: Start the web content extract service
cargo run --release --bin web_content_extract_service
```

You should see:
```
🌐 Web Content Extract Service - Starting mock web scraping service
✅ Registered web-content-extract: <service-id>
🚀 Web Content Extract Service starting on 127.0.0.1:8085
```

### Step 3: Start the Dividend Service

```bash
# Terminal 3: Start the dividend service
cargo run --release --bin dividend_service
```

You should see:
```
💰 Dividend Service - Starting service that processes dividend data
✅ Registered dividend-service: <service-id>
🔄 Dividend service requesting data from web-content-extract...
✅ Successfully received data from web-content-extract:
   Response: {"dividend_amount":2.5,"payment_date":"2024-01-15",...}
📊 Processing dividend data...
💰 Calculated dividend: $2.75
📅 Payment date: 2024-01-15
🏷️  Stock symbol: AAPL
🔄 Dividend service running... Press Ctrl+C to stop
```

### Step 4: Monitor Services in the Web Interface

Open `http://localhost:8080` in your browser to see both services registered and their status.

### What Happened?

1. **Hub Registration**: Both services registered themselves with the gRPC Hub
2. **Service Discovery**: The Dividend Service discovered the Web Content Extract Service through the hub
3. **Inter-Service Communication**: The Dividend Service requested financial data from the Web Content Extract Service
4. **Data Processing**: The extracted data was processed and dividends were calculated

This demonstrates the hub's role as a service registry enabling service discovery and inter-service communication.

## Command Line Options

```bash
grpc-hub [OPTIONS]

Options:
  --grpc-host <GRPC_HOST>    gRPC server host [default: 0.0.0.0]
  --grpc-port <GRPC_PORT>    gRPC server port [default: 50051]
  --http-host <HTTP_HOST>    HTTP server host [default: 0.0.0.0]
  --http-port <HTTP_PORT>    HTTP server port [default: 8080]
  -h, --help                 Print help
```

## API Endpoints

### gRPC API

The hub exposes the following gRPC services:

- `RegisterService`: Register a new service with the hub
- `UnregisterService`: Remove a service from the hub
- `ListServices`: List all registered services (with optional filtering)
- `GetService`: Get details for a specific service
- `HealthCheck`: Update service health status

### HTTP API

- `GET /`: Web interface showing all registered services
- `GET /api/services`: JSON API returning all registered services

## Service Registration

Services register with the hub by providing:

- **Service Name**: Human-readable service identifier
- **Version**: Service version (e.g., "1.0.0")
- **Address & Port**: Network location of the service
- **Methods**: List of available gRPC methods
- **Metadata**: Key-value pairs for additional information

## Example Service Registration

```rust
use grpc_hub::grpc_hub_client::GrpcHubClient;
use grpc_hub::RegisterServiceRequest;
use std::collections::HashMap;
use tonic::Request;

let mut client = GrpcHubClient::connect("http://127.0.0.1:50051").await?;

let mut metadata = HashMap::new();
metadata.insert("environment".to_string(), "production".to_string());
metadata.insert("team".to_string(), "backend".to_string());

let request = Request::new(RegisterServiceRequest {
    service_name: "user-service".to_string(),
    service_version: "1.0.0".to_string(),
    service_address: "127.0.0.1".to_string(),
    service_port: "8080".to_string(),
    methods: vec!["GetUser".to_string(), "CreateUser".to_string()],
    metadata,
});

let response = client.register_service(request).await?;
```

## Web Interface

The web interface provides:

- **Real-time Service List**: Automatically refreshes every 5 seconds
- **Service Details**: View methods, metadata, registration time, and health status
- **Responsive Design**: Works on desktop and mobile devices
- **Service Filtering**: Filter services by name or version

## Architecture

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Service A     │    │   Service B     │    │   Service C     │
│   (gRPC)        │    │   (gRPC)        │    │   (gRPC)        │
└─────────┬───────┘    └─────────┬───────┘    └─────────┬───────┘
          │                      │                      │
          │                      │                      │
          └──────────────────────┼──────────────────────┘
                                 │
                    ┌─────────────▼─────────────┐
                    │      gRPC Hub Server      │
                    │                          │
                    │  ┌─────────────────────┐ │
                    │  │   Service Registry  │ │
                    │  └─────────────────────┘ │
                    │                          │
                    │  ┌─────────────────────┐ │
                    │  │    HTTP Server      │ │
                    │  │   (Web Interface)   │ │
                    │  └─────────────────────┘ │
                    └─────────────┬─────────────┘
                                  │
                    ┌─────────────▼─────────────┐
                    │      Web Browser         │
                    │   (Service Discovery)     │
                    └───────────────────────────┘
```

## Development

### Building

```bash
cargo build
```

### Running Tests

```bash
cargo test
```

### Running Examples

```bash
# Start the hub server
cargo run --release

# In another terminal, run the demo client
cargo run --release --bin demo_client
```

## Dependencies

- **tonic**: gRPC framework for Rust
- **tokio**: Async runtime
- **hyper**: HTTP server
- **serde**: Serialization
- **clap**: Command-line argument parsing
- **uuid**: Unique identifier generation
- **chrono**: Date and time handling

## License

This project is licensed under the MIT License.

