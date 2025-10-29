use grpc_hub_connector::GrpcHubConnector;
use std::collections::HashMap;
use serde_json::Value;

/// Example service that uses the gRPC Hub Connector
struct MyService {
    connector: GrpcHubConnector,
    service_id: String,
    data: HashMap<String, Value>,
}

impl MyService {
    fn new(hub_host: String, hub_port: u16) -> Self {
        Self {
            connector: GrpcHubConnector::with_hub_connection(hub_host, hub_port),
            service_id: uuid::Uuid::new_v4().to_string(),
            data: HashMap::new(),
        }
    }
    
    async fn register_with_hub(&self, service_name: &str, service_port: u16) -> Result<(), Box<dyn std::error::Error>> {
        println!("📝 Registering service '{}' with hub...", service_name);
        
        // In a real implementation, you would register with the hub here
        // For this example, we'll just simulate it
        println!("✅ Service '{}' registered with ID: {}", service_name, self.service_id);
        Ok(())
    }
    
    async fn process_request(&self, request: &str) -> Result<String, Box<dyn std::error::Error>> {
        println!("🔄 Processing request: {}", request);
        
        // Report busy status
        self.connector.set_service_busy(&self.service_id).await?;
        
        // Simulate processing work
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        
        // Call another service through the hub
        self.call_another_service().await?;
        
        // Report online status
        self.connector.set_service_online(&self.service_id).await?;
        
        Ok(format!("Processed: {}", request))
    }
    
    async fn call_another_service(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("📞 Calling another service through hub...");
        
        // Discover a service
        match self.connector.discover_service("web-content-extract").await {
            Ok((host, port)) => {
                println!("✅ Found service at {}:{}", host, port);
                // In a real implementation, you would make a gRPC call here
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                println!("✅ Service call completed");
            }
            Err(e) => {
                println!("⚠️  Service not available: {}", e);
            }
        }
        
        Ok(())
    }
    
    async fn get_service_info(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("📊 Getting service information...");
        
        // List all services
        let services = self.connector.list_all_services().await?;
        println!("📋 Total services registered: {}", services.len());
        
        // Get cache info
        let (has_cache, timestamp) = self.connector.get_cache_info().await;
        println!("💾 Cache status: {} (timestamp: {})", 
            if has_cache { "cached" } else { "empty" }, timestamp);
        
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 **Service Example with gRPC Hub Connector**");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    // Create service with custom hub connection
    let service = MyService::new("127.0.0.1".to_string(), 50099);
    
    // Register with hub
    service.register_with_hub("my-example-service", 8080).await?;
    
    // Get service information
    service.get_service_info().await?;
    
    // Process some requests
    for i in 1..=3 {
        println!("\n--- Request {} ---", i);
        let result = service.process_request(&format!("Request #{}", i)).await?;
        println!("Result: {}", result);
        
        // Wait between requests
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    }
    
    println!("\n✅ Service example completed successfully!");
    Ok(())
}
