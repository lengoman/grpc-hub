use std::collections::HashMap;
use tonic::Request;

mod grpc_hub {
    tonic::include_proto!("grpc_hub");
}

use grpc_hub::grpc_hub_client::GrpcHubClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = GrpcHubClient::connect("http://127.0.0.1:50099").await?;
    
    println!("Connected to gRPC Hub");
    
    // Register a test service
    let mut metadata = HashMap::new();
    metadata.insert("environment".to_string(), "test".to_string());
    metadata.insert("team".to_string(), "backend".to_string());
    
    let register_request = Request::new(grpc_hub::RegisterServiceRequest {
        service_name: "test-service".to_string(),
        service_version: "1.0.0".to_string(),
        service_address: "127.0.0.1".to_string(),
        service_port: "9090".to_string(),
        methods: vec![
            "GetUser".to_string(),
            "CreateUser".to_string(),
            "UpdateUser".to_string(),
            "DeleteUser".to_string(),
        ],
        metadata,
    });
    
    let response = client.register_service(register_request).await?;
    let response = response.into_inner();
    
    println!("Service registration response:");
    println!("  Success: {}", response.success);
    println!("  Message: {}", response.message);
    println!("  Service ID: {}", response.service_id);
    
    if response.success {
        let service_id = response.service_id;
        
        // Send a health check
        let health_request = Request::new(grpc_hub::HealthCheckRequest {
            service_id: service_id.clone(),
        });
        
        let health_response = client.health_check(health_request).await?;
        let health_response = health_response.into_inner();
        
        println!("\nHealth check response:");
        println!("  Healthy: {}", health_response.healthy);
        println!("  Message: {}", health_response.message);
        
        // List all services
        let list_request = Request::new(grpc_hub::ListServicesRequest {
            filter: None,
        });
        
        let list_response = client.list_services(list_request).await?;
        let list_response = list_response.into_inner();
        
        println!("\nRegistered services:");
        for service in list_response.services {
            println!("  Service: {} v{}", service.service_name, service.service_version);
            println!("    ID: {}", service.service_id);
            println!("    Address: {}:{}", service.service_address, service.service_port);
            println!("    Methods: {:?}", service.methods);
            println!("    Metadata: {:?}", service.metadata);
            println!("    Registered: {}", service.registered_at);
            println!("    Last Heartbeat: {}", service.last_heartbeat);
            println!();
        }
        
        // Get specific service details
        let get_request = Request::new(grpc_hub::GetServiceRequest {
            service_id: service_id.clone(),
        });
        
        let get_response = client.get_service(get_request).await?;
        let get_response = get_response.into_inner();
        
        if get_response.found {
            if let Some(service) = get_response.service {
                println!("Service details:");
                println!("  Name: {}", service.service_name);
                println!("  Version: {}", service.service_version);
                println!("  Address: {}:{}", service.service_address, service.service_port);
                println!("  Methods: {:?}", service.methods);
            }
        }
        
        // Simulate periodic health checks
        println!("\nSending periodic health checks...");
        for i in 1..=3 {
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            
            let health_request = Request::new(grpc_hub::HealthCheckRequest {
                service_id: service_id.clone(),
            });
            
            let health_response = client.health_check(health_request).await?;
            let health_response = health_response.into_inner();
            
            println!("Health check #{}: {}", i, health_response.message);
        }
        
        // Unregister the service
        let unregister_request = Request::new(grpc_hub::UnregisterServiceRequest {
            service_id: service_id.clone(),
        });
        
        let unregister_response = client.unregister_service(unregister_request).await?;
        let unregister_response = unregister_response.into_inner();
        
        println!("\nUnregister response:");
        println!("  Success: {}", unregister_response.success);
        println!("  Message: {}", unregister_response.message);
    }
    
    Ok(())
}

