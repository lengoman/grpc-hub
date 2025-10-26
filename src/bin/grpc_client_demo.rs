use tonic::Request;

mod grpc_hub {
    tonic::include_proto!("grpc_hub");
}

use grpc_hub::grpc_hub_client::GrpcHubClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = GrpcHubClient::connect("http://127.0.0.1:50099").await?;
    
    println!("🔗 gRPC Client Demo - Demonstrating service discovery and calls");
    println!("🌐 This shows how a client would discover and call services\n");
    
    // Step 1: Discover services
    println!("📡 Step 1: Discovering available services...");
    let list_request = Request::new(grpc_hub::ListServicesRequest {
        filter: None,
    });
    
    let list_response = client.list_services(list_request).await?;
    let services = list_response.into_inner().services;
    
    println!("Found {} services:", services.len());
    for service in &services {
        println!("  🔧 {} v{} at {}:{}", 
            service.service_name, 
            service.service_version, 
            service.service_address, 
            service.service_port
        );
        println!("     Available methods: {}", service.methods.join(", "));
    }
    
    // Step 2: Get detailed info for a specific service
    if let Some(user_service) = services.iter().find(|s| s.service_name == "user-service") {
        println!("\n📋 Step 2: Getting detailed info for user-service...");
        
        let get_request = Request::new(grpc_hub::GetServiceRequest {
            service_id: user_service.service_id.clone(),
        });
        
        let get_response = client.get_service(get_request).await?;
        let service_details = get_response.into_inner();
        
        if service_details.found {
            let service_info = service_details.service.unwrap();
            println!("  ✅ Service Details:");
            println!("     Name: {}", service_info.service_name);
            println!("     Version: {}", service_info.service_version);
            println!("     Address: {}:{}", service_info.service_address, service_info.service_port);
            println!("     Methods: {}", service_info.methods.join(", "));
            println!("     Metadata: {:?}", service_info.metadata);
            println!("     Registered: {}", service_info.registered_at);
            println!("     Last Heartbeat: {}", service_info.last_heartbeat);
        }
    }
    
    // Step 3: Demonstrate how to call a service
    println!("\n📞 Step 3: How to call a service (pseudo-code):");
    println!("   // In a real application, you would:");
    println!("   // 1. Create a gRPC client for the specific service");
    println!("   // 2. Connect to the service's address and port");
    println!("   // 3. Call the service methods");
    println!();
    println!("   // Example for user-service:");
    println!("   // let mut user_client = UserServiceClient::connect(\"http://127.0.0.1:8081\").await?;");
    println!("   // let response = user_client.get_user(GetUserRequest {{ user_id: \"123\" }}).await?;");
    println!();
    
    // Step 4: Show service health
    println!("💓 Step 4: Checking service health...");
    for service in &services {
        let health_request = Request::new(grpc_hub::HealthCheckRequest {
            service_id: service.service_id.clone(),
        });
        
        let health_response = client.health_check(health_request).await?;
        let health_result = health_response.into_inner();
        
        if health_result.healthy {
            println!("  💚 {}: {}", service.service_name, health_result.message);
        } else {
            println!("  ❌ {}: {}", service.service_name, health_result.message);
        }
    }
    
    // Step 5: Demonstrate service filtering
    println!("\n🔍 Step 5: Service filtering examples:");
    
    // Filter by name
    let user_filter_request = Request::new(grpc_hub::ListServicesRequest {
        filter: Some("user".to_string()),
    });
    
    let user_filter_response = client.list_services(user_filter_request).await?;
    let user_services = user_filter_response.into_inner().services;
    println!("  Services matching 'user': {}", user_services.len());
    
    // Filter by version
    let order_filter_request = Request::new(grpc_hub::ListServicesRequest {
        filter: Some("order".to_string()),
    });
    
    let order_filter_response = client.list_services(order_filter_request).await?;
    let order_services = order_filter_response.into_inner().services;
    println!("  Services matching 'order': {}", order_services.len());
    
    println!("\n🎉 Demo completed!");
    println!("💡 This demonstrates the service discovery pattern:");
    println!("   1. Client connects to the hub");
    println!("   2. Client discovers available services");
    println!("   3. Client gets service details and health status");
    println!("   4. Client can then connect directly to services");
    println!("   5. Client can filter services by name, version, etc.");
    
    Ok(())
}
