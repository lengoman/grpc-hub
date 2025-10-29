use grpc_hub_connector::GrpcHubConnector;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 **gRPC Hub Connector Example**");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    // Create a connector with default settings (127.0.0.1:50099)
    let connector = GrpcHubConnector::new();
    
    println!("📡 Connecting to gRPC hub...");
    
    // List all available services
    match connector.list_all_services().await {
        Ok(services) => {
            println!("📋 Available services ({}):", services.len());
            for service in services {
                println!("  - {}: {} ({}:{})", 
                    service.service_name, 
                    service.status,
                    service.service_address,
                    service.service_port
                );
            }
        }
        Err(e) => {
            println!("❌ Failed to list services: {}", e);
            return Ok(());
        }
    }
    
    // Try to discover a specific service
    let service_name = "dividend-service";
    println!("\n🔍 Discovering service: {}", service_name);
    
    match connector.discover_service(service_name).await {
        Ok((host, port)) => {
            println!("✅ Found {} at {}:{}", service_name, host, port);
            
            // Check if service is online
            match connector.is_service_online(service_name).await {
                Ok(is_online) => {
                    println!("📊 Service status: {}", if is_online { "online" } else { "offline" });
                }
                Err(e) => {
                    println!("⚠️  Could not check service status: {}", e);
                }
            }
        }
        Err(e) => {
            println!("❌ Service not found: {}", e);
        }
    }
    
    // Example of status reporting (if you have a service ID)
    let service_id = "example-service-123";
    println!("\n🔄 Reporting service status...");
    
    // Report busy status
    if let Err(e) = connector.set_service_busy(service_id).await {
        println!("⚠️  Could not report busy status: {}", e);
    } else {
        println!("🟠 Service reported as busy");
        
        // Simulate some work
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        
        // Report online status
        if let Err(e) = connector.set_service_online(service_id).await {
            println!("⚠️  Could not report online status: {}", e);
        } else {
            println!("🟢 Service reported as online");
        }
    }
    
    println!("\n✅ Example completed successfully!");
    Ok(())
}
