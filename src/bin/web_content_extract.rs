use std::collections::HashMap;
use tonic::Request;

mod grpc_hub {
    tonic::include_proto!("grpc_hub");
}

use grpc_hub::grpc_hub_client::GrpcHubClient;
use grpc_hub::RegisterServiceRequest;

// Helper function to extract method names from the proto file
fn get_service_methods() -> Vec<String> {
    include_str!("../../proto/web_content_extract.proto")
        .lines()
        .filter(|line| line.contains("rpc"))
        .map(|line| {
            // Extract method name from "rpc MethodName(...) returns (...);"
            line.split_whitespace()
                .nth(1)
                .unwrap_or("")
                .to_string()
        })
        .collect()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🌐 Web Content Extract Service - Starting data extraction service");
    
    // Connect to the gRPC hub
    let mut hub_client = GrpcHubClient::connect("http://127.0.0.1:50099").await?;
    
    // Register this service with the hub
    let mut metadata = HashMap::new();
    metadata.insert("team".to_string(), "data".to_string());
    metadata.insert("environment".to_string(), "production".to_string());
    metadata.insert("purpose".to_string(), "content_extraction".to_string());
    
    // Automatically discover methods from the proto file
    let methods = get_service_methods();
    println!("📋 Discovered {} methods from proto file", methods.len());
    
    let register_request = Request::new(RegisterServiceRequest {
        service_name: "web-content-extract".to_string(),
        service_version: "1.0.0".to_string(),
        service_address: "127.0.0.1".to_string(),
        service_port: "8084".to_string(),
        methods: methods.clone(),
        metadata,
    });
    
    let register_response = hub_client.register_service(register_request).await?;
    let register_response = register_response.into_inner();
    println!("✅ Registered web-content-extract: {}", register_response.service_id);
    
    // Simulate the web content extract service
    println!("🔄 Web content extract service ready to process requests...");
    
    // Keep the service running
    println!("🔄 Web content extract service running... Press Ctrl+C to stop");
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
        println!("💓 Web content extract service heartbeat...");
    }
}

