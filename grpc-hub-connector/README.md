# gRPC Hub Connector

A reusable Rust library for discovering and connecting to services through the gRPC hub. This connector provides a simple interface for services to register with the hub, discover other services, and make gRPC calls with intelligent load balancing.

## Features

- **Service Discovery**: Automatically discover and connect to services registered with the gRPC hub
- **Intelligent Load Balancing**: Round-robin distribution across multiple service instances
- **Service Status Management**: Report and track service status (online, busy, offline)
- **Caching**: Built-in caching for service addresses to improve performance
- **gRPC Communication**: Direct gRPC communication with the hub (no HTTP dependencies)
- **Error Handling**: Comprehensive error handling with `anyhow`

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
grpc-hub-connector = { git = "https://github.com/yourusername/grpc-hub", package = "grpc-hub-connector" }
```

Or if published to crates.io:

```toml
[dependencies]
grpc-hub-connector = "0.1.0"
```

## Quick Start

```rust
use grpc_hub_connector::GrpcHubConnector;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a connector with default settings (127.0.0.1:50099)
    let connector = GrpcHubConnector::new();
    
    // Or create with custom hub address
    let connector = GrpcHubConnector::with_hub_connection("192.168.1.100".to_string(), 50099);
    
    // Discover a service
    let (host, port) = connector.discover_service("my-service").await?;
    println!("Service found at {}:{}", host, port);
    
    // Report service status
    connector.set_service_busy("my-service-id").await?;
    // ... do work ...
    connector.set_service_online("my-service-id").await?;
    
    Ok(())
}
```

## API Reference

### Constructor Methods

- `GrpcHubConnector::new()` - Create with default settings (127.0.0.1:50099)
- `GrpcHubConnector::with_hub_connection(host, port)` - Create with custom hub address
- `GrpcHubConnector::with_hub_endpoint(endpoint)` - Create with full endpoint URL

### Service Discovery

- `discover_service(service_name)` - Discover a service by name
- `get_service_address(service_name)` - Get cached service address
- `list_all_services()` - List all registered services
- `is_service_online(service_name)` - Check if a service is online

### Status Management

- `set_service_busy(service_id)` - Report service as busy
- `set_service_online(service_id)` - Report service as online

### Configuration

- `with_cache_duration(seconds)` - Set cache duration
- `clear_cache()` - Clear service address cache
- `get_cache_info()` - Get cache information

## Examples

### Basic Service Discovery

```rust
use grpc_hub_connector::GrpcHubConnector;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let connector = GrpcHubConnector::new();
    
    // Discover a service
    match connector.discover_service("web-content-extract").await {
        Ok((host, port)) => {
            println!("Found service at {}:{}", host, port);
            // Use the service...
        }
        Err(e) => {
            eprintln!("Service not found: {}", e);
        }
    }
    
    Ok(())
}
```

### Service with Status Reporting

```rust
use grpc_hub_connector::GrpcHubConnector;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let connector = GrpcHubConnector::with_hub_connection("hub.example.com".to_string(), 50099);
    let service_id = "my-service-123".to_string();
    
    // Report busy status
    connector.set_service_busy(&service_id).await?;
    
    // Do some work
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    
    // Report online status
    connector.set_service_online(&service_id).await?;
    
    Ok(())
}
```

## Requirements

- Rust 1.70+
- Tokio runtime
- gRPC Hub server running

## License

MIT
