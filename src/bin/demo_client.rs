use std::collections::HashMap;
use std::time::Duration;
use tokio::time::sleep;
use tonic::Request;

mod grpc_hub {
    tonic::include_proto!("grpc_hub");
}

use grpc_hub::grpc_hub_client::GrpcHubClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = GrpcHubClient::connect("http://127.0.0.1:50099").await?;
    
    println!("🚀 Connected to gRPC Hub");
    println!("📡 Registering multiple demo services...\n");
    
    // Register multiple demo services
    let services = vec![
        ("user-service", "1.2.0", "127.0.0.1", "8081", vec!["GetUser", "CreateUser", "UpdateUser", "DeleteUser", "ListUsers"]),
        ("order-service", "2.1.0", "127.0.0.1", "8082", vec!["CreateOrder", "GetOrder", "UpdateOrder", "CancelOrder", "ListOrders"]),
        ("payment-service", "1.5.0", "127.0.0.1", "8083", vec!["ProcessPayment", "RefundPayment", "GetPaymentStatus"]),
        ("notification-service", "1.0.0", "127.0.0.1", "8084", vec!["SendEmail", "SendSMS", "SendPush"]),
        ("analytics-service", "3.0.0", "127.0.0.1", "8085", vec!["TrackEvent", "GetMetrics", "GenerateReport"]),
    ];
    
    let mut registered_services = Vec::new();
    
    for (name, version, address, port, methods) in services {
        let mut metadata = HashMap::new();
        metadata.insert("environment".to_string(), "production".to_string());
        metadata.insert("team".to_string(), "backend".to_string());
        metadata.insert("region".to_string(), "us-west-2".to_string());
        
        let register_request = Request::new(grpc_hub::RegisterServiceRequest {
            service_name: name.to_string(),
            service_version: version.to_string(),
            service_address: address.to_string(),
            service_port: port.to_string(),
            methods: methods.iter().map(|s| s.to_string()).collect(),
            metadata,
        });
        
        let response = client.register_service(register_request).await?;
        let response = response.into_inner();
        
        if response.success {
            println!("✅ Registered {} v{} (ID: {})", name, version, response.service_id);
            registered_services.push(response.service_id);
        } else {
            println!("❌ Failed to register {}: {}", name, response.message);
        }
    }
    
    println!("\n📊 Current service registry:");
    
    // List all services
    let list_request = Request::new(grpc_hub::ListServicesRequest {
        filter: None,
    });
    
    let list_response = client.list_services(list_request).await?;
    let list_response = list_response.into_inner();
    
    for service in &list_response.services {
        println!("  🔧 {} v{} at {}:{}", 
                service.service_name, 
                service.service_version, 
                service.service_address, 
                service.service_port);
        println!("     Methods: {}", service.methods.join(", "));
        println!("     ID: {}", service.service_id);
        println!();
    }
    
    println!("💓 Simulating health checks for 30 seconds...");
    
    // Simulate health checks for 30 seconds
    for i in 1..=15 {
        sleep(Duration::from_secs(2)).await;
        
        for service_id in &registered_services {
            let health_request = Request::new(grpc_hub::HealthCheckRequest {
                service_id: service_id.clone(),
            });
            
            let health_response = client.health_check(health_request).await?;
            let health_response = health_response.into_inner();
            
            if i % 5 == 0 { // Show status every 10 seconds
                println!("  💚 Health check #{}: {}", i, health_response.message);
            }
        }
    }
    
    println!("\n🔍 Testing service filtering:");
    
    // Test filtering
    let filter_request = Request::new(grpc_hub::ListServicesRequest {
        filter: Some("service".to_string()),
    });
    
    let filter_response = client.list_services(filter_request).await?;
    let filter_response = filter_response.into_inner();
    
    println!("  Services matching 'service': {}", filter_response.services.len());
    
    // Test specific service lookup
    if let Some(service_id) = registered_services.first() {
        let get_request = Request::new(grpc_hub::GetServiceRequest {
            service_id: service_id.clone(),
        });
        
        let get_response = client.get_service(get_request).await?;
        let get_response = get_response.into_inner();
        
        if get_response.found {
            if let Some(service) = get_response.service {
                println!("\n🔍 Service details for {}:", service.service_name);
                println!("  Version: {}", service.service_version);
                println!("  Address: {}:{}", service.service_address, service.service_port);
                println!("  Methods: {}", service.methods.join(", "));
                println!("  Metadata: {:?}", service.metadata);
                println!("  Registered: {}", service.registered_at);
                println!("  Last Heartbeat: {}", service.last_heartbeat);
            }
        }
    }
    
    println!("\n🎉 Demo completed! Services are now registered and running.");
    println!("🌐 Check the web interface at http://localhost:8080");
    println!("💡 Services will continue running with periodic health checks...");
    println!("🛑 Press Ctrl+C to stop and unregister all services");
    
    // Keep services running with periodic health checks
    let mut health_check_count = 0;
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
        health_check_count += 1;
        
        println!("💓 Health check round #{} for {} services...", health_check_count, registered_services.len());
        
        for service_id in &registered_services {
            let health_request = Request::new(grpc_hub::HealthCheckRequest {
                service_id: service_id.clone(),
            });
            
            let health_response = client.health_check(health_request).await?;
            let health_response = health_response.into_inner();
            
            if !health_response.healthy {
                println!("  ⚠️  Service {} is unhealthy: {}", service_id, health_response.message);
            }
        }
        
        println!("  ✅ All services healthy - continuing monitoring...");
    }
}

