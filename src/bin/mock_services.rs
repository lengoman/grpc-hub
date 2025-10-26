use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tonic::{transport::Server, Request, Response, Status};
use uuid::Uuid;
use chrono::Utc;

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

// Mock User Service Implementation
#[derive(Debug)]
struct UserServiceServer {
    users: Arc<RwLock<HashMap<String, user_service::User>>>,
}

impl UserServiceServer {
    fn new() -> Self {
        let mut users = HashMap::new();
        // Add some initial users
        users.insert("1".to_string(), user_service::User {
            user_id: "1".to_string(),
            name: "Alice Smith".to_string(),
            email: "alice@example.com".to_string(),
            created_at: Utc::now().to_rfc3339(),
        });
        users.insert("2".to_string(), user_service::User {
            user_id: "2".to_string(),
            name: "Bob Johnson".to_string(),
            email: "bob@example.com".to_string(),
            created_at: Utc::now().to_rfc3339(),
        });
        
        Self {
            users: Arc::new(RwLock::new(users)),
        }
    }
}

impl Default for UserServiceServer {
    fn default() -> Self {
        Self::new()
    }
}

#[tonic::async_trait]
impl user_service::user_service_server::UserService for UserServiceServer {
    async fn get_user(&self, request: Request<user_service::GetUserRequest>) -> Result<Response<user_service::GetUserResponse>, Status> {
        let req = request.into_inner();
        println!("📞 UserService.GetUser called with ID: {}", req.user_id);
        
        let users = self.users.read().await;
        match users.get(&req.user_id) {
            Some(user) => Ok(Response::new(user_service::GetUserResponse {
                user_id: user.user_id.clone(),
                name: user.name.clone(),
                email: user.email.clone(),
                created_at: user.created_at.clone(),
            })),
            None => Err(Status::not_found(format!("User with ID {} not found", req.user_id))),
        }
    }
    
    async fn create_user(&self, request: Request<user_service::CreateUserRequest>) -> Result<Response<user_service::CreateUserResponse>, Status> {
        let req = request.into_inner();
        println!("📞 UserService.CreateUser called with name: {}", req.name);
        
        let user_id = Uuid::new_v4().to_string();
        let new_user = user_service::User {
            user_id: user_id.clone(),
            name: req.name,
            email: req.email,
            created_at: Utc::now().to_rfc3339(),
        };
        
        {
            let mut users = self.users.write().await;
            users.insert(user_id.clone(), new_user);
        }
        
        Ok(Response::new(user_service::CreateUserResponse {
            user_id,
            success: true,
            message: "User created successfully".to_string(),
        }))
    }
    
    async fn update_user(&self, request: Request<user_service::UpdateUserRequest>) -> Result<Response<user_service::UpdateUserResponse>, Status> {
        let req = request.into_inner();
        println!("📞 UserService.UpdateUser called for ID: {}", req.user_id);
        
        let mut users = self.users.write().await;
        match users.get_mut(&req.user_id) {
            Some(user) => {
                user.name = req.name;
                user.email = req.email;
                Ok(Response::new(user_service::UpdateUserResponse {
                    success: true,
                    message: "User updated successfully".to_string(),
                }))
            },
            None => Err(Status::not_found(format!("User with ID {} not found", req.user_id))),
        }
    }
    
    async fn delete_user(&self, request: Request<user_service::DeleteUserRequest>) -> Result<Response<user_service::DeleteUserResponse>, Status> {
        let req = request.into_inner();
        println!("📞 UserService.DeleteUser called for ID: {}", req.user_id);
        
        let mut users = self.users.write().await;
        match users.remove(&req.user_id) {
            Some(_) => Ok(Response::new(user_service::DeleteUserResponse {
                success: true,
                message: "User deleted successfully".to_string(),
            })),
            None => Err(Status::not_found(format!("User with ID {} not found", req.user_id))),
        }
    }
    
    async fn list_users(&self, request: Request<user_service::ListUsersRequest>) -> Result<Response<user_service::ListUsersResponse>, Status> {
        let _req = request.into_inner();
        println!("📞 UserService.ListUsers called");
        
        let users = self.users.read().await;
        let user_list: Vec<user_service::User> = users.values().cloned().collect();
        let total = user_list.len();
        
        Ok(Response::new(user_service::ListUsersResponse {
            users: user_list,
            total: total as i32,
        }))
    }
}

// Mock Order Service Implementation
#[derive(Debug)]
struct OrderServiceServer {
    orders: Arc<RwLock<HashMap<String, order_service::Order>>>,
}

impl OrderServiceServer {
    fn new() -> Self {
        Self {
            orders: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for OrderServiceServer {
    fn default() -> Self {
        Self::new()
    }
}

#[tonic::async_trait]
impl order_service::order_service_server::OrderService for OrderServiceServer {
    async fn create_order(&self, request: Request<order_service::CreateOrderRequest>) -> Result<Response<order_service::CreateOrderResponse>, Status> {
        let req = request.into_inner();
        println!("📞 OrderService.CreateOrder called for user: {}", req.user_id);
        
        let order_id = Uuid::new_v4().to_string();
        let total: f64 = req.items.iter().map(|item| item.price * item.quantity as f64).sum();
        
        let new_order = order_service::Order {
            order_id: order_id.clone(),
            user_id: req.user_id,
            status: "pending".to_string(),
            total,
            created_at: Utc::now().to_rfc3339(),
        };
        
        {
            let mut orders = self.orders.write().await;
            orders.insert(order_id.clone(), new_order);
        }
        
        Ok(Response::new(order_service::CreateOrderResponse {
            order_id,
            success: true,
            message: "Order created successfully".to_string(),
        }))
    }
    
    async fn get_order(&self, request: Request<order_service::GetOrderRequest>) -> Result<Response<order_service::GetOrderResponse>, Status> {
        let req = request.into_inner();
        println!("📞 OrderService.GetOrder called for ID: {}", req.order_id);
        
        let orders = self.orders.read().await;
        match orders.get(&req.order_id) {
            Some(order) => Ok(Response::new(order_service::GetOrderResponse {
                order_id: order.order_id.clone(),
                user_id: order.user_id.clone(),
                status: order.status.clone(),
                total: order.total,
                created_at: order.created_at.clone(),
            })),
            None => Err(Status::not_found(format!("Order with ID {} not found", req.order_id))),
        }
    }
    
    async fn update_order(&self, request: Request<order_service::UpdateOrderRequest>) -> Result<Response<order_service::UpdateOrderResponse>, Status> {
        let req = request.into_inner();
        println!("📞 OrderService.UpdateOrder called for ID: {}", req.order_id);
        
        let mut orders = self.orders.write().await;
        match orders.get_mut(&req.order_id) {
            Some(order) => {
                order.status = req.status;
                Ok(Response::new(order_service::UpdateOrderResponse {
                    success: true,
                    message: "Order updated successfully".to_string(),
                }))
            },
            None => Err(Status::not_found(format!("Order with ID {} not found", req.order_id))),
        }
    }
    
    async fn cancel_order(&self, request: Request<order_service::CancelOrderRequest>) -> Result<Response<order_service::CancelOrderResponse>, Status> {
        let req = request.into_inner();
        println!("📞 OrderService.CancelOrder called for ID: {}", req.order_id);
        
        let mut orders = self.orders.write().await;
        match orders.get_mut(&req.order_id) {
            Some(order) => {
                order.status = "cancelled".to_string();
                Ok(Response::new(order_service::CancelOrderResponse {
                    success: true,
                    message: "Order cancelled successfully".to_string(),
                }))
            },
            None => Err(Status::not_found(format!("Order with ID {} not found", req.order_id))),
        }
    }
    
    async fn list_orders(&self, request: Request<order_service::ListOrdersRequest>) -> Result<Response<order_service::ListOrdersResponse>, Status> {
        let req = request.into_inner();
        println!("📞 OrderService.ListOrders called for user: {}", req.user_id);
        
        let orders = self.orders.read().await;
        let user_orders: Vec<order_service::Order> = orders.values()
            .filter(|order| order.user_id == req.user_id)
            .cloned()
            .collect();
        let total = user_orders.len();
        
        Ok(Response::new(order_service::ListOrdersResponse {
            orders: user_orders,
            total: total as i32,
        }))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎭 Mock Services Server - Starting mock gRPC services");
    
    // Start mock services in background tasks
    let user_service_task = tokio::spawn(async {
        let addr = "127.0.0.1:8081".parse().unwrap();
        let user_service = UserServiceServer::default();
        
        println!("🚀 User Service starting on {}", addr);
        Server::builder()
            .add_service(user_service::user_service_server::UserServiceServer::new(user_service))
            .serve(addr)
            .await
            .unwrap();
    });
    
    let order_service_task = tokio::spawn(async {
        let addr = "127.0.0.1:8082".parse().unwrap();
        let order_service = OrderServiceServer::default();
        
        println!("🚀 Order Service starting on {}", addr);
        Server::builder()
            .add_service(order_service::order_service_server::OrderServiceServer::new(order_service))
            .serve(addr)
            .await
            .unwrap();
    });
    
    // Register services with the hub
    let mut hub_client = GrpcHubClient::connect("http://127.0.0.1:50099").await?;
    
    // Register User Service
    let mut metadata = HashMap::new();
    metadata.insert("environment".to_string(), "production".to_string());
    metadata.insert("team".to_string(), "backend".to_string());
    metadata.insert("service_type".to_string(), "user_management".to_string());
    
    let user_register_request = Request::new(grpc_hub::RegisterServiceRequest {
        service_name: "user-service".to_string(),
        service_version: "2.1.0".to_string(),
        service_address: "127.0.0.1".to_string(),
        service_port: "8081".to_string(),
        methods: vec![
            "GetUser".to_string(),
            "CreateUser".to_string(),
            "UpdateUser".to_string(),
            "DeleteUser".to_string(),
            "ListUsers".to_string(),
        ],
        metadata,
    });
    
    let user_response = hub_client.register_service(user_register_request).await?;
    let user_response = user_response.into_inner();
    println!("✅ Registered user-service: {}", user_response.service_id);
    
    // Register Order Service
    let mut order_metadata = HashMap::new();
    order_metadata.insert("environment".to_string(), "production".to_string());
    order_metadata.insert("team".to_string(), "backend".to_string());
    order_metadata.insert("service_type".to_string(), "order_management".to_string());
    
    let order_register_request = Request::new(grpc_hub::RegisterServiceRequest {
        service_name: "order-service".to_string(),
        service_version: "1.5.0".to_string(),
        service_address: "127.0.0.1".to_string(),
        service_port: "8082".to_string(),
        methods: vec![
            "CreateOrder".to_string(),
            "GetOrder".to_string(),
            "UpdateOrder".to_string(),
            "CancelOrder".to_string(),
            "ListOrders".to_string(),
        ],
        metadata: order_metadata,
    });
    
    let order_response = hub_client.register_service(order_register_request).await?;
    let order_response = order_response.into_inner();
    println!("✅ Registered order-service: {}", order_response.service_id);
    
    println!("\n🌐 Mock services are now running and registered!");
    println!("📞 You can now call these services from the UI or gRPC clients");
    println!("🛑 Press Ctrl+C to stop all services");
    
    // Keep services running
    tokio::select! {
        _ = user_service_task => {},
        _ = order_service_task => {},
    }
    
    Ok(())
}
