use tonic::Request;

mod grpc_hub {
    tonic::include_proto!("grpc_hub");
}

use grpc_hub::grpc_hub_client::GrpcHubClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = GrpcHubClient::connect("http://127.0.0.1:50099").await?;
    
    println!("🧪 Service Tester - Testing gRPC service calls");
    println!("🌐 Make sure the hub is running and services are registered\n");
    
    // List all registered services
    let list_request = Request::new(grpc_hub::ListServicesRequest {
        filter: None,
    });
    
    let list_response = client.list_services(list_request).await?;
    let services = list_response.into_inner().services;
    
    println!("📋 Found {} registered services:", services.len());
    for service in &services {
        println!("  🔧 {} v{} at {}:{}", 
            service.service_name, 
            service.service_version, 
            service.service_address, 
            service.service_port
        );
        println!("     Methods: {}", service.methods.join(", "));
        println!("     ID: {}", service.service_id);
        println!();
    }
    
    // Test service details for each service
    for service in &services {
        println!("🔍 Testing service: {}", service.service_name);
        
        // Get detailed service info
        let get_request = Request::new(grpc_hub::GetServiceRequest {
            service_id: service.service_id.clone(),
        });
        
        let get_response = client.get_service(get_request).await?;
        let service_details = get_response.into_inner();
        
        if service_details.found {
            let service_info = service_details.service.unwrap();
            println!("  ✅ Service found:");
            println!("     Name: {}", service_info.service_name);
            println!("     Version: {}", service_info.service_version);
            println!("     Address: {}:{}", service_info.service_address, service_info.service_port);
            println!("     Methods: {}", service_info.methods.join(", "));
            println!("     Registered: {}", service_info.registered_at);
            println!("     Last Heartbeat: {}", service_info.last_heartbeat);
            println!("     Metadata: {:?}", service_info.metadata);
        } else {
            println!("  ❌ Service not found");
        }
        
        // Test health check
        let health_request = Request::new(grpc_hub::HealthCheckRequest {
            service_id: service.service_id.clone(),
        });
        
        let health_response = client.health_check(health_request).await?;
        let health_result = health_response.into_inner();
        
        if health_result.healthy {
            println!("  💚 Health check: {}", health_result.message);
        } else {
            println!("  ❌ Health check failed: {}", health_result.message);
        }
        
        println!();
    }
    
    // Demonstrate service filtering
    println!("🔍 Testing service filtering:");
    
    // Filter by name containing "user"
    let filter_request = Request::new(grpc_hub::ListServicesRequest {
        filter: Some("user".to_string()),
    });
    
    let filter_response = client.list_services(filter_request).await?;
    let filtered_services = filter_response.into_inner().services;
    
    println!("  Services matching 'user': {}", filtered_services.len());
    for service in &filtered_services {
        println!("    - {} v{}", service.service_name, service.service_version);
    }
    
    println!("\n🎉 Service testing completed!");
    println!("💡 To test actual gRPC calls, you would need to:");
    println!("   1. Implement the service proto files");
    println!("   2. Create gRPC clients for each service");
    println!("   3. Call the service methods directly");
    println!("   4. Or use tools like grpcurl or Postman with gRPC support");
    
    Ok(())
}
