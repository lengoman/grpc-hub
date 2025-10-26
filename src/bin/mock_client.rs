use std::collections::HashMap;
use std::time::Duration;
use tokio::time::sleep;
use tonic::Request;

mod grpc_hub {
    tonic::include_proto!("grpc_hub");
}

use grpc_hub::grpc_hub_client::GrpcHubClient;
use grpc_hub::{ServiceCallRequest, SubscribeRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = GrpcHubClient::connect("http://127.0.0.1:50099").await?;
    
    println!("🎭 Mock Client - Registering sample services for UI testing");
    println!("🌐 Open http://localhost:8080 to see the services in the web interface\n");
    
    // Register multiple mock services with different characteristics
    let mock_services = vec![
        ("user-service", "2.1.0", "127.0.0.1", "8081", vec!["GetUser", "CreateUser", "UpdateUser", "DeleteUser", "ListUsers", "SearchUsers"]),
        ("order-service", "1.5.0", "127.0.0.1", "8082", vec!["CreateOrder", "GetOrder", "UpdateOrder", "CancelOrder", "ListOrders", "GetOrderHistory"]),
        ("payment-service", "3.0.0", "127.0.0.1", "8083", vec!["ProcessPayment", "RefundPayment", "GetPaymentStatus", "ListTransactions", "ValidateCard"]),
        ("notification-service", "1.2.0", "127.0.0.1", "8084", vec!["SendEmail", "SendSMS", "SendPush", "SendWebhook", "ScheduleNotification"]),
        ("analytics-service", "2.3.0", "127.0.0.1", "8085", vec!["TrackEvent", "GetMetrics", "GenerateReport", "GetDashboard", "ExportData"]),
        ("web-content-extract", "2.0.0", "127.0.0.1", "8085", vec!["ExtractFinancialData", "ExtractTextContent", "ExtractStructuredData"]),
        ("dividend-consumer", "1.0.0", "127.0.0.1", "8086", vec!["CalculateDividends", "GetDividendHistory", "ProcessDividendData"]),
        ("inventory-service", "1.0.0", "127.0.0.1", "8087", vec!["GetStock", "UpdateStock", "ReserveItem", "ReleaseItem", "GetInventory"]),
        ("shipping-service", "1.8.0", "127.0.0.1", "8088", vec!["CalculateShipping", "CreateShipment", "TrackPackage", "UpdateStatus", "GetRates"]),
        ("auth-service", "2.0.0", "127.0.0.1", "8089", vec!["Login", "Logout", "Register", "RefreshToken", "ValidateToken", "ResetPassword"]),
    ];
    
    let mut registered_services = Vec::new();
    
    for (name, version, address, port, methods) in mock_services {
        let mut metadata = HashMap::new();
        metadata.insert("environment".to_string(), "production".to_string());
        metadata.insert("team".to_string(), "backend".to_string());
        metadata.insert("region".to_string(), "us-west-2".to_string());
        metadata.insert("deployment".to_string(), "kubernetes".to_string());
        metadata.insert("monitoring".to_string(), "enabled".to_string());
        
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
    
    println!("\n📊 All services registered successfully!");
    println!("🌐 View them at: http://localhost:8080");
    println!("💡 Services will run with periodic health checks...");
    
    // Demonstrate service-to-service communication
    println!("\n🌉 Demonstrating service-to-service communication through the hub...");
    
    // Example 1: Order service calling user service
    println!("\n📋 Example 1: Order service → User service");
    let order_to_user_request = Request::new(ServiceCallRequest {
        target_service: "user-service".to_string(),
        method: "GetUser".to_string(),
        request_data: serde_json::json!({
            "user_id": "12345"
        }).to_string(),
        caller_service: "order-service".to_string(),
        headers: HashMap::new(),
    });
    
    match client.call_service(order_to_user_request).await {
        Ok(response) => {
            let response = response.into_inner();
            if response.success {
                println!("✅ Order service successfully called user service:");
                println!("   Response: {}", response.response_data);
            } else {
                println!("❌ Service call failed: {}", response.error_message);
            }
        }
        Err(e) => println!("❌ Service call error: {}", e),
    }
    
    // Example 2: Payment service calling order service
    println!("\n💳 Example 2: Payment service → Order service");
    let payment_to_order_request = Request::new(ServiceCallRequest {
        target_service: "order-service".to_string(),
        method: "GetOrder".to_string(),
        request_data: serde_json::json!({
            "order_id": "order-67890"
        }).to_string(),
        caller_service: "payment-service".to_string(),
        headers: HashMap::new(),
    });
    
    match client.call_service(payment_to_order_request).await {
        Ok(response) => {
            let response = response.into_inner();
            if response.success {
                println!("✅ Payment service successfully called order service:");
                println!("   Response: {}", response.response_data);
            } else {
                println!("❌ Service call failed: {}", response.error_message);
            }
        }
        Err(e) => println!("❌ Service call error: {}", e),
    }
    
    // Example 3: Analytics service calling multiple services
    println!("\n📊 Example 3: Analytics service → Multiple services");
    
    // Call user service
    let analytics_to_user_request = Request::new(ServiceCallRequest {
        target_service: "user-service".to_string(),
        method: "ListUsers".to_string(),
        request_data: serde_json::json!({
            "page": 1,
            "limit": 10
        }).to_string(),
        caller_service: "analytics-service".to_string(),
        headers: HashMap::new(),
    });
    
    match client.call_service(analytics_to_user_request).await {
        Ok(response) => {
            let response = response.into_inner();
            if response.success {
                println!("✅ Analytics → User service: {}", response.response_data);
            } else {
                println!("❌ Analytics → User failed: {}", response.error_message);
            }
        }
        Err(e) => println!("❌ Analytics → User error: {}", e),
    }
    
    // Call order service
    let analytics_to_order_request = Request::new(ServiceCallRequest {
        target_service: "order-service".to_string(),
        method: "ListOrders".to_string(),
        request_data: serde_json::json!({
            "user_id": "12345",
            "page": 1,
            "limit": 5
        }).to_string(),
        caller_service: "analytics-service".to_string(),
        headers: HashMap::new(),
    });
    
    match client.call_service(analytics_to_order_request).await {
        Ok(response) => {
            let response = response.into_inner();
            if response.success {
                println!("✅ Analytics → Order service: {}", response.response_data);
            } else {
                println!("❌ Analytics → Order failed: {}", response.error_message);
            }
        }
        Err(e) => println!("❌ Analytics → Order error: {}", e),
    }
    
    // Example 3: Web Content Extract → Dividend Consumer flow
    println!("\n🌐 Example 3: Web Content Extract → Dividend Consumer flow");
    let web_extract_request = Request::new(ServiceCallRequest {
        target_service: "web-content-extract".to_string(),
        method: "ExtractFinancialData".to_string(),
        request_data: serde_json::json!({
            "url": "https://financial-data.com/dividend-info",
            "fields": ["dividend_amount", "payment_date", "stock_symbol"],
            "extraction_type": "financial_data"
        }).to_string(),
        caller_service: "dividend-consumer".to_string(),
        headers: HashMap::new(),
    });
    
    match client.call_service(web_extract_request).await {
        Ok(response) => {
            let response = response.into_inner();
            if response.success {
                println!("✅ Dividend consumer → Web content extract: {}", response.response_data);
                
                // Now call dividend service to process the data
                let dividend_request = Request::new(ServiceCallRequest {
                    target_service: "dividend-consumer".to_string(),
                    method: "CalculateDividends".to_string(),
                    request_data: serde_json::json!({
                        "amount": 2.50,
                        "currency": "USD",
                        "user_id": "user123"
                    }).to_string(),
                    caller_service: "web-content-extract".to_string(),
                    headers: HashMap::new(),
                });
                
                match client.call_service(dividend_request).await {
                    Ok(response) => {
                        let response = response.into_inner();
                        if response.success {
                            println!("✅ Web content extract → Dividend consumer: {}", response.response_data);
                        } else {
                            println!("❌ Dividend service call failed: {}", response.error_message);
                        }
                    }
                    Err(e) => println!("❌ Dividend service call error: {}", e),
                }
            } else {
                println!("❌ Web content extract call failed: {}", response.error_message);
            }
        }
        Err(e) => println!("❌ Web content extract call error: {}", e),
    }
    
    println!("\n🎉 Service-to-service communication examples completed!");
    println!("🌐 You can now test these interactions in the UI at: http://localhost:8080");
    println!("💡 Services will continue running with periodic health checks...");
    println!("🛑 Press Ctrl+C to stop\n");
    
    // Keep services running with periodic health checks
    let mut health_check_count = 0;
    loop {
        sleep(Duration::from_secs(15)).await;
        health_check_count += 1;
        
        println!("💓 Health check round #{} for {} services...", health_check_count, registered_services.len());
        
        let mut healthy_count = 0;
        for service_id in &registered_services {
            let health_request = Request::new(grpc_hub::HealthCheckRequest {
                service_id: service_id.clone(),
            });
            
            let health_response = client.health_check(health_request).await?;
            let health_response = health_response.into_inner();
            
            if health_response.healthy {
                healthy_count += 1;
            } else {
                println!("  ⚠️  Service {} is unhealthy: {}", service_id, health_response.message);
            }
        }
        
        println!("  ✅ {}/{} services healthy - continuing monitoring...", healthy_count, registered_services.len());
    }
}
