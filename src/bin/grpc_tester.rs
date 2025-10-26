use tonic::Request;

mod grpc_hub {
    tonic::include_proto!("grpc_hub");
}

mod user_service {
    tonic::include_proto!("user_service");
}

mod order_service {
    tonic::include_proto!("order_service");
}

use grpc_hub::grpc_hub_client::GrpcHubClient;
use user_service::user_service_client::UserServiceClient;
use order_service::order_service_client::OrderServiceClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 gRPC Service Tester - Testing actual service calls");
    
    // Connect to hub to discover services
    let mut hub_client = GrpcHubClient::connect("http://127.0.0.1:50099").await?;
    
    // Get user service details
    let list_request = Request::new(grpc_hub::ListServicesRequest {
        filter: Some("user".to_string()),
    });
    
    let list_response = hub_client.list_services(list_request).await?;
    let services = list_response.into_inner().services;
    
    if let Some(user_service) = services.first() {
        println!("🔍 Found user service: {} at {}:{}", 
            user_service.service_name, 
            user_service.service_address, 
            user_service.service_port
        );
        
        // Connect directly to the user service
        let user_service_url = format!("http://{}:{}", user_service.service_address, user_service.service_port);
        let mut user_client = UserServiceClient::connect(user_service_url).await?;
        
        println!("\n📞 Testing User Service calls:");
        
        // Test GetUser
        println!("  🔸 Testing GetUser...");
        let get_user_request = Request::new(user_service::GetUserRequest {
            user_id: "123".to_string(),
        });
        let get_user_response = user_client.get_user(get_user_request).await?;
        let user = get_user_response.into_inner();
        println!("     ✅ Response: ID={}, Name={}, Email={}", user.user_id, user.name, user.email);
        
        // Test CreateUser
        println!("  🔸 Testing CreateUser...");
        let create_user_request = Request::new(user_service::CreateUserRequest {
            name: "Test User".to_string(),
            email: "test@example.com".to_string(),
        });
        let create_user_response = user_client.create_user(create_user_request).await?;
        let create_result = create_user_response.into_inner();
        println!("     ✅ Response: Success={}, Message={}, UserID={}", 
            create_result.success, create_result.message, create_result.user_id);
        
        // Test ListUsers
        println!("  🔸 Testing ListUsers...");
        let list_users_request = Request::new(user_service::ListUsersRequest {
            page: 1,
            limit: 10,
        });
        let list_users_response = user_client.list_users(list_users_request).await?;
        let users = list_users_response.into_inner();
        println!("     ✅ Response: Found {} users", users.total);
        for user in &users.users {
            println!("       - {} ({})", user.name, user.email);
        }
    }
    
    // Test order service
    let order_list_request = Request::new(grpc_hub::ListServicesRequest {
        filter: Some("order".to_string()),
    });
    
    let order_list_response = hub_client.list_services(order_list_request).await?;
    let order_services = order_list_response.into_inner().services;
    
    if let Some(order_service) = order_services.first() {
        println!("\n🔍 Found order service: {} at {}:{}", 
            order_service.service_name, 
            order_service.service_address, 
            order_service.service_port
        );
        
        // Connect directly to the order service
        let order_service_url = format!("http://{}:{}", order_service.service_address, order_service.service_port);
        let mut order_client = OrderServiceClient::connect(order_service_url).await?;
        
        println!("\n📞 Testing Order Service calls:");
        
        // Test CreateOrder
        println!("  🔸 Testing CreateOrder...");
        let create_order_request = Request::new(order_service::CreateOrderRequest {
            user_id: "123".to_string(),
            items: vec![
                order_service::OrderItem {
                    product_id: "prod-1".to_string(),
                    quantity: 2,
                    price: 29.99,
                },
                order_service::OrderItem {
                    product_id: "prod-2".to_string(),
                    quantity: 1,
                    price: 19.99,
                },
            ],
        });
        let create_order_response = order_client.create_order(create_order_request).await?;
        let order_result = create_order_response.into_inner();
        println!("     ✅ Response: Success={}, Message={}, OrderID={}", 
            order_result.success, order_result.message, order_result.order_id);
        
        // Test GetOrder
        println!("  🔸 Testing GetOrder...");
        let get_order_request = Request::new(order_service::GetOrderRequest {
            order_id: order_result.order_id.clone(),
        });
        let get_order_response = order_client.get_order(get_order_request).await?;
        let order = get_order_response.into_inner();
        println!("     ✅ Response: OrderID={}, UserID={}, Status={}, Total=${}", 
            order.order_id, order.user_id, order.status, order.total);
    }
    
    println!("\n🎉 All service calls completed successfully!");
    println!("💡 This demonstrates how clients can:");
    println!("   1. Discover services through the hub");
    println!("   2. Connect directly to services");
    println!("   3. Call service methods with real data");
    println!("   4. Get actual responses from the services");
    
    Ok(())
}
