// Example of how to use the GrpcHubConnector in other services
// This file demonstrates the usage patterns and can be used as a template

use std::collections::HashMap;
use tonic::{transport::Server, Request, Response, Status};

// Include the grpc_hub_connector
mod grpc_hub_connector {
    include!("grpc_hub_connector.rs");
}

// Example: A service that needs to call another service
#[derive(Debug, Clone)]
struct ExampleService {
    hub_connector: grpc_hub_connector::GrpcHubConnector,
}

impl ExampleService {
    fn new() -> Self {
        Self {
            // Use default settings (connects to http://127.0.0.1:50099)
            hub_connector: grpc_hub_connector::GrpcHubConnector::new(),
        }
    }

    fn new_with_custom_hub(hub_endpoint: String) -> Self {
        Self {
            // Use custom hub endpoint
            hub_connector: grpc_hub_connector::GrpcHubConnector::with_hub_endpoint(hub_endpoint),
        }
    }

    fn new_with_custom_cache(hub_endpoint: String, cache_duration_seconds: u64) -> Self {
        Self {
            // Use custom hub endpoint and cache duration
            hub_connector: grpc_hub_connector::GrpcHubConnector::with_hub_endpoint(hub_endpoint)
                .with_cache_duration(cache_duration_seconds),
        }
    }

    // Example: Call another service
    async fn call_another_service(&self, service_name: &str) -> Result<String, Box<dyn std::error::Error>> {
        // Get the service address and port
        let (address, port) = self.hub_connector.get_service_address(service_name).await?;
        
        println!("🔍 [DEBUG] Connecting to {} at {}:{}", service_name, address, port);
        
        // Here you would create your gRPC client and make the call
        // let mut client = YourServiceClient::connect(format!("http://{}:{}", address, port)).await?;
        // let response = client.your_method(request).await?;
        
        Ok(format!("Successfully connected to {} at {}:{}", service_name, address, port))
    }

    // Example: Check if a service is online
    async fn check_service_health(&self, service_name: &str) -> Result<bool, Box<dyn std::error::Error>> {
        let is_online = self.hub_connector.is_service_online(service_name).await?;
        println!("🔍 [DEBUG] Service '{}' is {}", service_name, if is_online { "online" } else { "offline" });
        Ok(is_online)
    }

    // Example: List all available services
    async fn list_available_services(&self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let services = self.hub_connector.list_all_services().await?;
        let service_names: Vec<String> = services.iter()
            .map(|s| s.service_name.clone())
            .collect();
        
        println!("🔍 [DEBUG] Available services: {:?}", service_names);
        Ok(service_names)
    }

    // Example: Clear cache and force fresh discovery
    async fn refresh_service_discovery(&self) {
        self.hub_connector.clear_cache().await;
        println!("🔍 [DEBUG] Service discovery cache cleared");
    }

    // Example: Get cache statistics
    async fn get_cache_stats(&self) -> (bool, u64) {
        self.hub_connector.get_cache_info().await
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Example Service using GrpcHubConnector");
    
    // Create service with default settings
    let service = ExampleService::new();
    
    // Example usage
    match service.call_another_service("web-content-extract").await {
        Ok(result) => println!("✅ {}", result),
        Err(e) => println!("❌ Error: {}", e),
    }
    
    // Check service health
    match service.check_service_health("web-content-extract").await {
        Ok(is_online) => println!("📊 Service is {}", if is_online { "online" } else { "offline" }),
        Err(e) => println!("❌ Error checking health: {}", e),
    }
    
    // List all services
    match service.list_available_services().await {
        Ok(services) => println!("📋 Available services: {:?}", services),
        Err(e) => println!("❌ Error listing services: {}", e),
    }
    
    // Get cache stats
    let (has_cached, last_update) = service.get_cache_stats().await;
    println!("💾 Cache stats: has_cached={}, last_update={}", has_cached, last_update);
    
    Ok(())
}

// Example: Integration in a gRPC service implementation
mod example_grpc_service {
    use super::*;
    
    // This would be your actual gRPC service implementation
    pub struct MyGrpcService {
        hub_connector: grpc_hub_connector::GrpcHubConnector,
    }
    
    impl MyGrpcService {
        pub fn new() -> Self {
            Self {
                hub_connector: grpc_hub_connector::GrpcHubConnector::new(),
            }
        }
        
        // Example gRPC method that calls another service
        pub async fn my_grpc_method(&self, request: Request<()>) -> Result<Response<String>, Status> {
            // Use the connector to discover and call another service
            match self.hub_connector.get_service_address("target-service").await {
                Ok((address, port)) => {
                    // Make your gRPC call here
                    println!("Calling target-service at {}:{}", address, port);
                    Ok(Response::new("Success".to_string()))
                }
                Err(e) => {
                    Err(Status::unavailable(format!("Target service unavailable: {}", e)))
                }
            }
        }
    }
}
