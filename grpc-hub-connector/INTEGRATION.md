# Integration Guide: Using grpc-hub-connector from GitHub

This guide shows how to use the `grpc-hub-connector` library in your own Rust projects by importing it directly from the GitHub repository.

## Quick Start

### 1. Add to Cargo.toml

Add this to your project's `Cargo.toml`:

```toml
[dependencies]
grpc-hub-connector = { git = "https://github.com/yourusername/grpc-hub", package = "grpc-hub-connector" }
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
anyhow = "1.0"
```

### 2. Basic Usage

```rust
use grpc_hub_connector::GrpcHubConnector;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create connector
    let connector = GrpcHubConnector::new(); // Uses 127.0.0.1:50099 by default
    
    // Or with custom hub address
    let connector = GrpcHubConnector::with_hub_connection(
        "hub.example.com".to_string(), 
        50099
    );
    
    // Discover a service
    let (host, port) = connector.discover_service("my-service").await?;
    println!("Service found at {}:{}", host, port);
    
    Ok(())
}
```

## Real-World Examples

### Example 1: Microservice with Service Discovery

```rust
use grpc_hub_connector::GrpcHubConnector;
use std::collections::HashMap;

struct MyMicroservice {
    connector: GrpcHubConnector,
    service_id: String,
    data: HashMap<String, serde_json::Value>,
}

impl MyMicroservice {
    fn new(hub_host: String, hub_port: u16) -> Self {
        Self {
            connector: GrpcHubConnector::with_hub_connection(hub_host, hub_port),
            service_id: uuid::Uuid::new_v4().to_string(),
            data: HashMap::new(),
        }
    }
    
    async fn process_request(&self, request: &str) -> Result<String, Box<dyn std::error::Error>> {
        // Report busy status
        self.connector.set_service_busy(&self.service_id).await?;
        
        // Do work
        let result = self.do_work(request).await?;
        
        // Report online status
        self.connector.set_service_online(&self.service_id).await?;
        
        Ok(result)
    }
    
    async fn do_work(&self, request: &str) -> Result<String, Box<dyn std::error::Error>> {
        // Simulate work
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        Ok(format!("Processed: {}", request))
    }
    
    async fn call_dependency(&self, service_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Discover and call another service
        let (host, port) = self.connector.discover_service(service_name).await?;
        println!("Calling {} at {}:{}", service_name, host, port);
        
        // Make your gRPC call here
        // let client = YourGrpcClient::connect(format!("http://{}:{}", host, port)).await?;
        // let response = client.your_method(request).await?;
        
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let service = MyMicroservice::new("127.0.0.1".to_string(), 50099);
    
    // Process requests
    for i in 1..=5 {
        let result = service.process_request(&format!("Request #{}", i)).await?;
        println!("Result: {}", result);
    }
    
    Ok(())
}
```

### Example 2: Load Testing Client

```rust
use grpc_hub_connector::GrpcHubConnector;
use std::time::Instant;

async fn load_test_service(
    service_name: &str, 
    method: &str, 
    iterations: u32,
    interval_ms: u64
) -> Result<(), Box<dyn std::error::Error>> {
    let connector = GrpcHubConnector::new();
    
    println!("🧪 Load testing {} with {} calls", service_name, iterations);
    
    let mut success_count = 0;
    let mut total_duration = std::time::Duration::new(0, 0);
    
    for i in 1..=iterations {
        let start = Instant::now();
        
        // Discover service
        match connector.discover_service(service_name).await {
            Ok((host, port)) => {
                // Simulate gRPC call
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                
                let duration = start.elapsed();
                total_duration += duration;
                success_count += 1;
                
                println!("✅ Call {}: {}:{} ({}ms)", i, host, port, duration.as_millis());
            }
            Err(e) => {
                println!("❌ Call {}: Failed - {}", i, e);
            }
        }
        
        if i < iterations {
            tokio::time::sleep(tokio::time::Duration::from_millis(interval_ms)).await;
        }
    }
    
    println!("📊 Results: {}/{} successful, avg: {}ms", 
        success_count, iterations, 
        total_duration.as_millis() / success_count as u128
    );
    
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    load_test_service("dividend-service", "GetDividendHistory", 10, 1000).await?;
    Ok(())
}
```

### Example 3: Service Health Monitor

```rust
use grpc_hub_connector::GrpcHubConnector;
use tokio::time::{sleep, Duration};

async fn monitor_services() -> Result<(), Box<dyn std::error::Error>> {
    let connector = GrpcHubConnector::new();
    
    loop {
        println!("🔍 Checking service health...");
        
        // List all services
        match connector.list_all_services().await {
            Ok(services) => {
                for service in services {
                    let is_online = connector.is_service_online(&service.service_name).await?;
                    let status = if is_online { "🟢" } else { "🔴" };
                    println!("{} {}: {} ({}:{})", 
                        status, service.service_name, service.status,
                        service.service_address, service.service_port
                    );
                }
            }
            Err(e) => {
                println!("❌ Failed to get services: {}", e);
            }
        }
        
        sleep(Duration::from_secs(30)).await;
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    monitor_services().await
}
```

## Configuration Options

### Custom Hub Connection

```rust
// Default (127.0.0.1:50099)
let connector = GrpcHubConnector::new();

// Custom host and port
let connector = GrpcHubConnector::with_hub_connection("hub.example.com".to_string(), 50099);

// From endpoint URL
let connector = GrpcHubConnector::with_hub_endpoint("http://hub.example.com:50099".to_string());
```

### Cache Configuration

```rust
let connector = GrpcHubConnector::new()
    .with_cache_duration(60); // 60 seconds cache

// Clear cache when needed
connector.clear_cache().await;

// Check cache status
let (has_cache, timestamp) = connector.get_cache_info().await;
```

## Error Handling

The library uses `anyhow::Result` for error handling:

```rust
use anyhow::Result;

async fn handle_service_discovery() -> Result<()> {
    let connector = GrpcHubConnector::new();
    
    match connector.discover_service("my-service").await {
        Ok((host, port)) => {
            println!("Service found at {}:{}", host, port);
        }
        Err(e) => {
            eprintln!("Service discovery failed: {}", e);
            // Handle error appropriately
        }
    }
    
    Ok(())
}
```

## Dependencies

The library requires these dependencies in your `Cargo.toml`:

```toml
[dependencies]
grpc-hub-connector = { git = "https://github.com/yourusername/grpc-hub", package = "grpc-hub-connector" }
tokio = { version = "1.0", features = ["full"] }
anyhow = "1.0"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
uuid = { version = "1.0", features = ["v4"] } # If you need to generate service IDs
```

## Publishing to crates.io (Optional)

If you want to publish the library to crates.io for easier distribution:

1. Update the version in `grpc-hub-connector/Cargo.toml`
2. Add your crates.io credentials: `cargo login`
3. Publish: `cargo publish --package grpc-hub-connector`

Then users can add it as:

```toml
[dependencies]
grpc-hub-connector = "0.1.0"
```

## Troubleshooting

### Common Issues

1. **Service not found**: Make sure the gRPC hub is running and the service is registered
2. **Connection refused**: Check that the hub host and port are correct
3. **Compilation errors**: Ensure you have the required dependencies

### Debug Logging

Enable debug logging to see what's happening:

```rust
use std::env;

// Enable debug logging
env::set_var("RUST_LOG", "debug");

// Your code here
```

## Support

For issues and questions:
- GitHub Issues: [https://github.com/yourusername/grpc-hub/issues](https://github.com/yourusername/grpc-hub/issues)
- Documentation: [https://docs.rs/grpc-hub-connector](https://docs.rs/grpc-hub-connector) (when published)
