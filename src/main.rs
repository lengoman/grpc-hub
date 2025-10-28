use clap::Parser;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_stream::wrappers::ReceiverStream;
use futures_util::StreamExt;
use tonic::{transport::Server, Request, Response, Status};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use tokio::process::Command;
use http_body_util::BodyExt;


mod grpc_hub {
    tonic::include_proto!("grpc_hub");
}

mod grpc_hub_connector;


#[derive(Parser, Debug)]
#[command(name = "grpc-hub")]
#[command(about = "gRPC Hub - Central registry and router for gRPC services")]
struct Args {
    /// gRPC server port
    #[arg(long, default_value = "50099")]
    grpc_port: u16,
    
    /// HTTP server port
    #[arg(long, default_value = "8080")]
    http_port: u16,
    
    /// HTTP server host
    #[arg(long, default_value = "0.0.0.0")]
    http_host: String,
    
    /// gRPC server host
    #[arg(long, default_value = "0.0.0.0")]
    grpc_host: String,
}

// grpcurl-based gRPC calling functions
async fn call_grpc_method(
    host: &str,
    port: u16,
    service: &str,
    method: &str,
    input: serde_json::Value,
) -> Result<serde_json::Value, anyhow::Error> {
    let address = format!("{}:{}", host, port);
    let full_method = format!("{}/{}", service, method);
    
    println!("🔍 [DEBUG] Hub: Starting gRPC call to {} at {}", full_method, address);
    
    // Convert input to JSON string
    let input_json = serde_json::to_string(&input)?;
    println!("🔍 [DEBUG] Hub: Input JSON: {}", input_json);
    
    // Call grpcurl with timeout
    println!("🔍 [DEBUG] Hub: Executing grpcurl command with timeout");
    let output = tokio::time::timeout(
        std::time::Duration::from_secs(10), // 10 second timeout
        tokio::process::Command::new("grpcurl")
            .arg("-plaintext")
            .arg("-d")
            .arg(&input_json)
            .arg(&address)
            .arg(&full_method)
            .output()
    ).await??;
    
    println!("🔍 [DEBUG] Hub: grpcurl command completed with status: {}", output.status);
    
    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        println!("❌ [DEBUG] Hub: gRPC call failed: {}", error);
        return Err(anyhow::anyhow!("gRPC call failed: {}", error));
    }
    
    let result = String::from_utf8_lossy(&output.stdout).to_string();
    println!("🔍 [DEBUG] Hub: gRPC call successful, result: {}", result);
    
    // Try to parse as JSON, if it fails return as string
    match serde_json::from_str::<serde_json::Value>(&result) {
        Ok(json) => Ok(json),
        Err(_) => Ok(serde_json::Value::String(result)),
    }
}

use grpc_hub::grpc_hub_server::{GrpcHub, GrpcHubServer};
use grpc_hub::*;

#[derive(Parser)]
#[command(name = "grpc-hub")]
#[command(about = "A gRPC hub server that acts as a registry for other gRPC services")]
struct Cli {
    #[arg(long, default_value = "0.0.0.0")]
    grpc_host: String,
    
    #[arg(long, default_value = "50099")]
    grpc_port: u16,
    
    #[arg(long, default_value = "0.0.0.0")]
    http_host: String,
    
    #[arg(long, default_value = "8080")]
    http_port: u16,
}

#[derive(Debug, Clone)]
struct ServiceInfo {
    service_id: String,
    service_name: String,
    service_version: String,
    service_address: String,
    service_port: String,
    methods: Vec<String>,
    metadata: HashMap<String, String>,
    registered_at: DateTime<Utc>,
    last_heartbeat: DateTime<Utc>,
    status: String, // "online", "offline", or "busy"
}

impl From<ServiceInfo> for grpc_hub::ServiceInfo {
    fn from(info: ServiceInfo) -> Self {
        grpc_hub::ServiceInfo {
            service_id: info.service_id,
            service_name: info.service_name,
            service_version: info.service_version,
            service_address: info.service_address,
            service_port: info.service_port,
            methods: info.methods,
            metadata: info.metadata,
            registered_at: info.registered_at.to_rfc3339(),
            last_heartbeat: info.last_heartbeat.to_rfc3339(),
            status: info.status, // Use actual status from the service
        }
    }
}

#[derive(Debug, Clone)]
struct GrpcHubService {
    services: Arc<RwLock<HashMap<String, ServiceInfo>>>,
    event_senders: Arc<RwLock<Vec<tokio::sync::broadcast::Sender<SSEEvent>>>>,
}

#[derive(Debug, Clone)]
struct SSEEvent {
    event_type: String,
    data: String,
}

impl Default for GrpcHubService {
    fn default() -> Self {
        Self {
            services: Arc::new(RwLock::new(HashMap::new())),
            event_senders: Arc::new(RwLock::new(Vec::new())),
        }
    }
}

impl GrpcHubService {
    async fn broadcast_event(&self, event: SSEEvent) {
        let senders = self.event_senders.read().await;
        println!("📡 Broadcasting event '{}' to {} subscribers", event.event_type, senders.len());
        for sender in senders.iter() {
            if let Err(e) = sender.send(event.clone()) {
                println!("⚠️  Failed to send event: {}", e);
            }
        }
    }
    
    async fn add_event_sender(&self, sender: tokio::sync::broadcast::Sender<SSEEvent>) {
        let mut senders = self.event_senders.write().await;
        senders.push(sender);
    }

    async fn set_service_busy(&self, service_id: &str) {
        println!("🔍 [DEBUG] set_service_busy: Attempting to set service {} to busy", service_id);
        let mut services = self.services.write().await;
        if let Some(service) = services.get_mut(service_id) {
            println!("🔍 [DEBUG] set_service_busy: Found service {}, current status: {}", service.service_name, service.status);
            if service.status == "online" {
                service.status = "busy".to_string();
                println!("🔄 Service {} is now busy", service.service_name);
                
                // Broadcast status change
                self.broadcast_event(SSEEvent {
                    event_type: "status_change".to_string(),
                    data: serde_json::json!({
                        "service_id": service_id,
                        "service_name": service.service_name,
                        "status": "busy"
                    }).to_string(),
                }).await;
            } else {
                println!("🔍 [DEBUG] set_service_busy: Service {} is not online (status: {}), not setting to busy", service.service_name, service.status);
            }
        } else {
            println!("❌ [DEBUG] set_service_busy: Service {} not found in services map", service_id);
        }
    }

    async fn set_service_online(&self, service_id: &str) {
        let mut services = self.services.write().await;
        if let Some(service) = services.get_mut(service_id) {
            if service.status == "busy" {
                service.status = "online".to_string();
                println!("✅ Service {} is now online", service.service_name);
                
                // Broadcast status change
                self.broadcast_event(SSEEvent {
                    event_type: "status_change".to_string(),
                    data: serde_json::json!({
                        "service_id": service_id,
                        "service_name": service.service_name,
                        "status": "online"
                    }).to_string(),
                }).await;
            }
        }
    }

    async fn get_service_by_address(&self, address: &str, port: u16) -> Option<String> {
        let services = self.services.read().await;
        println!("🔍 [DEBUG] get_service_by_address: Looking for {}:{}", address, port);
        println!("🔍 [DEBUG] get_service_by_address: Available services:");
        for service in services.values() {
            println!("  - {}:{} (ID: {})", service.service_address, service.service_port, service.service_id);
        }
        let result = services.values()
            .find(|s| s.service_address == address && s.service_port == port.to_string())
            .map(|s| s.service_id.clone());
        println!("🔍 [DEBUG] get_service_by_address: Result: {:?}", result);
        result
    }

    /// Get the best available service by name (prioritizes online, non-busy services)
    async fn get_best_service_by_name(&self, service_name: &str) -> Option<(String, String, u16)> {
        let services = self.services.read().await;
        
        println!("🔍 [DEBUG] get_best_service_by_name: Looking for service '{}'", service_name);
        
        // Find all services with the matching name
        let matching_services: Vec<_> = services.values()
            .filter(|service| service.service_name == service_name)
            .collect();
        
        if matching_services.is_empty() {
            println!("❌ [DEBUG] get_best_service_by_name: No services found with name '{}'", service_name);
            return None;
        }
        
        println!("🔍 [DEBUG] get_best_service_by_name: Found {} services with name '{}'", matching_services.len(), service_name);
        
        // Prioritize services that are online and not busy
        let online_services: Vec<_> = matching_services.iter()
            .filter(|service| service.status == "online")
            .collect();
        
        let selected_service = if !online_services.is_empty() {
            println!("✅ [DEBUG] get_best_service_by_name: Found {} online services, selecting first", online_services.len());
            online_services[0]
        } else {
            println!("⚠️  [DEBUG] get_best_service_by_name: No online services, selecting first available");
            matching_services[0]
        };
        
        let port = selected_service.service_port.parse::<u16>().ok()?;
        println!("🎯 [DEBUG] get_best_service_by_name: Selected service at {}:{} (status: {})", 
                 selected_service.service_address, port, selected_service.status);
        
        Some((
            selected_service.service_id.clone(),
            selected_service.service_address.clone(),
            port
        ))
    }

    /// Mark a service as offline instantly when a connection fails
    async fn mark_service_offline(&self, service_id: &str, reason: &str) {
        let service_name = {
            let mut services = self.services.write().await;
            if let Some(service) = services.get_mut(service_id) {
                let was_online = service.status == "online" || service.status == "busy";
                service.status = "offline".to_string();
                let service_name = service.service_name.clone();
                
                println!("🔴 Service {} (ID: {}) marked offline: {}", service_name, service_id, reason);
                println!("   Status updated to: {}", service.status);
                
                if was_online {
                    Some(service_name)
                } else {
                    None
                }
            } else {
                None
            }
        }; // Lock is dropped here after status is updated
        
        // Broadcast status change after releasing the lock
        if let Some(name) = service_name {
            self.broadcast_event(SSEEvent {
                event_type: "status_change".to_string(),
                data: serde_json::json!({
                    "service_id": service_id,
                    "service_name": name,
                    "status": "offline",
                    "reason": reason
                }).to_string(),
            }).await;
        }
    }

    /// Perform active health check on a service
    async fn health_check_service(&self, service_id: &str, service_address: &str, service_port: u16) -> bool {
        // Try to connect to the service's gRPC endpoint
        let address = format!("{}:{}", service_address, service_port);
        
        match tokio::time::timeout(
            std::time::Duration::from_secs(2), // 2 second timeout for health checks
            tokio::net::TcpStream::connect(&address)
        ).await {
            Ok(Ok(_)) => {
                // Connection successful
                true
            }
            Ok(Err(e)) => {
                println!("🔍 [HEALTH] Service {} connection failed: {}", address, e);
                false
            }
            Err(_) => {
                println!("🔍 [HEALTH] Service {} connection timeout", address);
                false
            }
        }
    }

    /// Start active health monitoring for all services
    async fn start_health_monitoring(&self) {
        let hub_service = self.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(5)); // Check every 5 seconds
            
            loop {
                interval.tick().await;
                
                // Get all services for health checking
                let services_to_check: Vec<(String, String, u16)> = {
                    let services = hub_service.services.read().await;
                    services.values()
                        .filter(|s| s.status == "online" || s.status == "busy")
                        .map(|s| (s.service_id.clone(), s.service_address.clone(), s.service_port.parse().unwrap_or(0)))
                        .collect()
                };
                
                // Check each service
                for (service_id, address, port) in services_to_check {
                    if port == 0 {
                        continue; // Skip invalid ports
                    }
                    
                    let is_healthy = hub_service.health_check_service(&service_id, &address, port).await;
                    
                    if !is_healthy {
                        hub_service.mark_service_offline(&service_id, "Health check failed").await;
                        
                        // Verify the status was actually updated
                        let services = hub_service.services.read().await;
                        if let Some(service) = services.get(&service_id) {
                            println!("   ✅ Verified: Service status in map is now: {}", service.status);
                        }
                    }
                }
            }
        });
    }
}

#[tonic::async_trait]
impl GrpcHub for GrpcHubService {
    async fn register_service(
        &self,
        request: Request<RegisterServiceRequest>,
    ) -> Result<Response<RegisterServiceResponse>, Status> {
        let req = request.into_inner();
        
        let mut services = self.services.write().await;
        
        // Check if a service with the same name and address/port already exists
        let existing_service = services.values().find(|s| 
            s.service_name == req.service_name && 
            s.service_address == req.service_address && 
            s.service_port == req.service_port
        );
        
        let service_id = if let Some(existing) = existing_service {
            // Update existing service instead of creating a new one
            println!("Updating existing service: {}", existing.service_id);
            existing.service_id.clone()
        } else {
            // Create new service
            Uuid::new_v4().to_string()
        };
        
        let service_name = req.service_name.clone();
        let service_id_for_event = service_id.clone();
        
        let service_info = ServiceInfo {
            service_id: service_id.clone(),
            service_name: req.service_name,
            service_version: req.service_version,
            service_address: req.service_address,
            service_port: req.service_port,
            methods: req.methods,
            metadata: req.metadata,
            registered_at: Utc::now(),
            last_heartbeat: Utc::now(),
            status: "online".to_string(), // New services start as online
        };
        
            services.insert(service_id.clone(), service_info);
        drop(services); // Release the lock
        
        println!("Service registered: {}", service_id);
        
        // Broadcast service registered event
        let event = SSEEvent {
            event_type: "service_registered".to_string(),
            data: serde_json::json!({
                "service_id": service_id_for_event,
                "service_name": service_name,
                "status": "online"
            }).to_string(),
        };
        self.broadcast_event(event).await;
        
        Ok(Response::new(RegisterServiceResponse {
            success: true,
            message: "Service registered successfully".to_string(),
            service_id,
        }))
    }

    async fn unregister_service(
        &self,
        request: Request<UnregisterServiceRequest>,
    ) -> Result<Response<UnregisterServiceResponse>, Status> {
        let req = request.into_inner();
        
        let mut services = self.services.write().await;
        let removed = services.remove(&req.service_id);
        
        if removed.is_some() {
            println!("Service unregistered: {}", req.service_id);
            Ok(Response::new(UnregisterServiceResponse {
                success: true,
                message: "Service unregistered successfully".to_string(),
            }))
        } else {
            Ok(Response::new(UnregisterServiceResponse {
                success: false,
                message: "Service not found".to_string(),
            }))
        }
    }

    async fn list_services(
        &self,
        request: Request<ListServicesRequest>,
    ) -> Result<Response<ListServicesResponse>, Status> {
        let req = request.into_inner();
        let services = self.services.read().await;
        
        let mut service_list: Vec<grpc_hub::ServiceInfo> = services
            .values()
            .filter(|service| {
                if let Some(filter) = &req.filter {
                    service.service_name.contains(filter) || 
                    service.service_version.contains(filter)
                } else {
                    true
                }
            })
            .map(|service| service.clone().into())
            .collect();
        
        service_list.sort_by(|a, b| a.service_name.cmp(&b.service_name));
        
        Ok(Response::new(ListServicesResponse {
            services: service_list,
        }))
    }

    async fn get_service(
        &self,
        request: Request<GetServiceRequest>,
    ) -> Result<Response<GetServiceResponse>, Status> {
        let req = request.into_inner();
        let services = self.services.read().await;
        
        if let Some(service) = services.get(&req.service_id) {
            Ok(Response::new(GetServiceResponse {
                service: Some(service.clone().into()),
                found: true,
            }))
        } else {
            Ok(Response::new(GetServiceResponse {
                service: None,
                found: false,
            }))
        }
    }

    async fn health_check(
        &self,
        request: Request<HealthCheckRequest>,
    ) -> Result<Response<HealthCheckResponse>, Status> {
        let req = request.into_inner();
        let mut services = self.services.write().await;
        
        if let Some(service) = services.get_mut(&req.service_id) {
            let was_offline = service.status == "offline";
            let service_name = service.service_name.clone();
            service.last_heartbeat = Utc::now();
            // Mark service as online when it sends heartbeat
            service.status = "online".to_string();
            
            // Broadcast status change if service came back online
            if was_offline {
                drop(services);
                let event = SSEEvent {
                    event_type: "status_change".to_string(),
                    data: serde_json::json!({
                        "service_id": req.service_id,
                        "service_name": service_name,
                        "status": "online"
                    }).to_string(),
                };
                self.broadcast_event(event).await;
            }
            
            Ok(Response::new(HealthCheckResponse {
                healthy: true,
                message: "Service is healthy".to_string(),
            }))
        } else {
            Ok(Response::new(HealthCheckResponse {
                healthy: false,
                message: "Service not found".to_string(),
            }))
        }
    }

    async fn call_service(
        &self,
        _request: Request<ServiceCallRequest>,
    ) -> Result<Response<ServiceCallResponse>, Status> {
        // For now, return an error indicating reflection is in development
        Err(Status::unimplemented(
            "Full dynamic gRPC reflection is in development. The hub framework is ready but needs implementation of the ServerReflection API client."
        ))
    }

    type SubscribeToServiceStream = ReceiverStream<Result<ServiceEvent, Status>>;

    async fn subscribe_to_service(
        &self,
        request: Request<SubscribeRequest>,
    ) -> Result<Response<Self::SubscribeToServiceStream>, Status> {
        let req = request.into_inner();
        let (tx, rx) = tokio::sync::mpsc::channel(128);
        
        // Send initial event
        let _ = tx.send(Ok(ServiceEvent {
            event_type: "subscribed".to_string(),
            service_name: req.service_name,
            data: "{}".to_string(),
            timestamp: Utc::now().to_rfc3339(),
        })).await;
        
        Ok(Response::new(ReceiverStream::new(rx)))
    }
}

async fn start_http_server(
    hub_service: Arc<GrpcHubService>,
    host: String,
    port: u16,
) -> Result<(), Box<dyn std::error::Error>> {
    use hyper::body::Incoming;
    use hyper::service::service_fn;
    use hyper::Method;
    use hyper_util::rt::TokioExecutor;
    use hyper_util::server::conn::auto::Builder;
    
    let listener = tokio::net::TcpListener::bind(format!("{}:{}", host, port)).await?;
    println!("HTTP server listening on http://{}:{}", host, port);
    
    loop {
        let (stream, _) = listener.accept().await?;
        let hub_service = hub_service.clone();
        
        tokio::task::spawn(async move {
            let service = service_fn(move |req: hyper::Request<hyper::body::Incoming>| {
                let hub_service = hub_service.clone();
                async move {
                    handle_http_request(req, hub_service).await
                }
            });
            
            let io = hyper_util::rt::TokioIo::new(stream);
            
            if let Err(err) = Builder::new(TokioExecutor::new())
                .serve_connection(io, service)
                .await
            {
                eprintln!("Error serving connection: {:?}", err);
            }
        });
    }
}

type BoxBody = http_body_util::combinators::BoxBody<hyper::body::Bytes, hyper::Error>;

fn full_response(bytes: hyper::body::Bytes) -> BoxBody {
    http_body_util::Full::new(bytes).map_err(|e| match e {}).boxed()
}

async fn handle_http_request(
    req: hyper::Request<hyper::body::Incoming>,
    hub_service: Arc<GrpcHubService>,
) -> Result<hyper::Response<BoxBody>, hyper::Error> {
    use hyper::body::Bytes;
    use hyper::Method;
    
    let path = req.uri().path();
    let method = req.method();
    
    match (method, path) {
        (&Method::GET, "/api/services") => {
    let services = hub_service.services.read().await;
    let service_list: Vec<grpc_hub::ServiceInfo> = services
        .values()
        .map(|service| service.clone().into())
        .collect();
    
    // Convert to a serializable format
    let serializable_services: Vec<serde_json::Value> = service_list
        .into_iter()
                .map(|service| serde_json::json!({
            "service_id": service.service_id,
            "service_name": service.service_name,
            "service_version": service.service_version,
            "service_address": service.service_address,
            "service_port": service.service_port,
            "methods": service.methods,
            "metadata": service.metadata,
            "registered_at": service.registered_at,
            "last_heartbeat": service.last_heartbeat,
                    "status": service.status,
        }))
        .collect();
    
            let json = serde_json::json!({
        "services": serializable_services
    });
    
            Ok(hyper::Response::builder()
                .status(200)
                .header("content-type", "application/json")
                .body(full_response(Bytes::from(json.to_string())))
                .unwrap())
        }
        (&Method::DELETE, path) if path.starts_with("/api/services/") => {
            let service_id = path.trim_start_matches("/api/services/");
            let mut services = hub_service.services.write().await;
            let removed = services.remove(service_id);
            
            let json = if removed.is_some() {
                serde_json::json!({"success": true, "message": "Service unregistered successfully"})
                } else {
                serde_json::json!({"success": false, "message": "Service not found"})
            };
            
            Ok(hyper::Response::builder()
                .status(200)
                .header("content-type", "application/json")
                .body(full_response(Bytes::from(json.to_string())))
                .unwrap())
        }
        (&Method::GET, "/api/service-schema") => {
            let services = hub_service.services.read().await;
            
            let schemas: Vec<serde_json::Value> = services.values()
                .map(|service| serde_json::json!({
                "service_name": service.service_name,
                "service_version": service.service_version,
                "service_address": service.service_address,
                "service_port": service.service_port,
                    "methods": service.methods.iter().map(|method| serde_json::json!({
                        "name": method,
                        "description": format!("{} method", method),
                        "request_schema": {
                            "type": "object",
                            "properties": {}
                        }
                    })).collect::<Vec<_>>(),
                "metadata": service.metadata
                }))
        .collect();
    
            let json = serde_json::json!({"schemas": schemas});
            
            Ok(hyper::Response::builder()
                .status(200)
                .header("content-type", "application/json")
                .body(full_response(Bytes::from(json.to_string())))
                .unwrap())
        }
        (&Method::POST, "/api/service-status") => {
            // Read request body
            let bytes = match http_body_util::BodyExt::collect(req.into_body()).await {
                Ok(body) => body.to_bytes(),
                Err(_) => {
                    let json = serde_json::json!({
                        "success": false,
                        "error": "Failed to read request body"
                    });
                    return Ok(hyper::Response::builder()
                        .status(400)
                        .header("content-type", "application/json")
                        .body(full_response(Bytes::from(json.to_string())))
                        .unwrap());
                }
            };
            let body_str = String::from_utf8_lossy(&bytes);
            
            let request: serde_json::Value = match serde_json::from_str(&body_str) {
                Ok(req) => req,
                Err(_) => {
                    let json = serde_json::json!({
                        "success": false,
                        "error": "Invalid JSON request"
                    });
                    return Ok(hyper::Response::builder()
                        .status(400)
                        .header("content-type", "application/json")
                        .body(full_response(Bytes::from(json.to_string())))
                        .unwrap());
                }
            };
            
            // Extract service_id and status
            let (service_id, status) = match (
                request.get("service_id").and_then(|v| v.as_str()),
                request.get("status").and_then(|v| v.as_str()),
            ) {
                (Some(id), Some(st)) => (id, st),
                _ => {
                    let json = serde_json::json!({
                        "success": false,
                        "error": "Missing required fields: service_id, status"
                    });
                    return Ok(hyper::Response::builder()
                        .status(400)
                        .header("content-type", "application/json")
                        .body(full_response(Bytes::from(json.to_string())))
                        .unwrap());
                }
            };
            
            // Update service status
            let service_name = {
                let mut services = hub_service.services.write().await;
                if let Some(service) = services.get_mut(service_id) {
                    let old_status = service.status.clone();
                    service.status = status.to_string();
                    let service_name = service.service_name.clone();
                    
                    println!("🔄 Service {} status changed: {} -> {}", service_name, old_status, status);
                    
                    if old_status != status {
                        Some(service_name)
                    } else {
                        None
                    }
                } else {
                    None
                }
            }; // Lock is dropped here
            
            // Broadcast status change if it actually changed
            if let Some(name) = service_name {
                hub_service.broadcast_event(SSEEvent {
                    event_type: "status_change".to_string(),
                    data: serde_json::json!({
                        "service_id": service_id,
                        "service_name": name,
                        "status": status,
                        "reason": "Service reported status change"
                    }).to_string(),
                }).await;
                
                let json = serde_json::json!({
                    "success": true,
                    "message": format!("Service {} status updated to {}", service_id, status)
                });
                
                Ok(hyper::Response::builder()
                    .status(200)
                    .header("content-type", "application/json")
                    .body(full_response(Bytes::from(json.to_string())))
                    .unwrap())
            } else {
                let json = serde_json::json!({
                    "success": false,
                    "error": format!("Service {} not found", service_id)
                });
                
                Ok(hyper::Response::builder()
                    .status(404)
                    .header("content-type", "application/json")
                    .body(full_response(Bytes::from(json.to_string())))
                    .unwrap())
            }
        }
        (&Method::POST, "/api/grpc-call") => {
            // Read request body using http-body-util
            let bytes = match http_body_util::BodyExt::collect(req.into_body()).await {
                Ok(body) => body.to_bytes(),
                Err(_) => {
                    let json = serde_json::json!({
                        "success": false,
                        "error": "Failed to read request body"
                    });
                    return Ok(hyper::Response::builder()
                        .status(400)
                        .header("content-type", "application/json")
                        .body(full_response(Bytes::from(json.to_string())))
                        .unwrap());
                }
            };
            let body_str = String::from_utf8_lossy(&bytes);
            
            let request: serde_json::Value = match serde_json::from_str(&body_str) {
                Ok(req) => req,
                Err(_) => {
                    let json = serde_json::json!({
                        "success": false,
                        "error": "Invalid JSON request"
                    });
                    return Ok(hyper::Response::builder()
                        .status(400)
                        .header("content-type", "application/json")
                        .body(full_response(Bytes::from(json.to_string())))
                        .unwrap());
                }
            };
            
            // Extract request parameters - support both service name only and host+port
            let (service_name, method_name, host, port, input_data): (String, String, String, u16, serde_json::Value) = match (
                request.get("service").and_then(|v| v.as_str()),
                request.get("method").and_then(|v| v.as_str()),
                request.get("host").and_then(|v| v.as_str()),
                request.get("port").and_then(|v| v.as_str().and_then(|s| s.parse::<u16>().ok())),
                request.get("input").cloned(),
            ) {
                (Some(svc), Some(meth), Some(hst), Some(prt), inp_data) => {
                    // Direct addressing mode: host and port provided
                    (svc.to_string(), meth.to_string(), hst.to_string(), prt, inp_data.unwrap_or(serde_json::json!({})))
                }
                (Some(svc), Some(meth), None, None, inp_data) => {
                    // Intelligent selection mode: only service name provided
                    // Extract service name from full gRPC service name (e.g., "web_content_extract.WebContentExtract" -> "web-content-extract")
                    let short_service_name = svc.split('.').next().unwrap_or(svc)
                        .replace("_", "-")
                        .to_lowercase();
                    
                    println!("🔍 [DEBUG] Hub: Intelligent selection mode for service: {}", short_service_name);
                    
                    if let Some((service_id, selected_host, selected_port)) = hub_service.get_best_service_by_name(&short_service_name).await {
                        println!("🎯 [DEBUG] Hub: Selected service {} at {}:{}", service_id, selected_host, selected_port);
                        (svc.to_string(), meth.to_string(), selected_host, selected_port, inp_data.unwrap_or(serde_json::json!({})))
                    } else {
                        let json = serde_json::json!({
                            "success": false,
                            "error": format!("No available service found for '{}'", short_service_name)
                        });
                        return Ok(hyper::Response::builder()
                            .status(404)
                            .header("content-type", "application/json")
                            .body(full_response(Bytes::from(json.to_string())))
                            .unwrap());
                    }
                }
                _ => {
                    let json = serde_json::json!({
                        "success": false,
                        "error": "Missing required fields: service, method, and either (host, port) or service name for intelligent selection"
                    });
                    return Ok(hyper::Response::builder()
                        .status(400)
                        .header("content-type", "application/json")
                        .body(full_response(Bytes::from(json.to_string())))
                        .unwrap());
                }
            };
            
            // Set service to busy before making the call
            println!("🔍 [DEBUG] Hub: Looking for service at {}:{}", host, port);
            if let Some(service_id) = hub_service.get_service_by_address(&host, port).await {
                println!("🔍 [DEBUG] Hub: Found service ID: {}, setting to busy", service_id);
                hub_service.set_service_busy(&service_id).await;
            } else {
                println!("❌ [DEBUG] Hub: No service found at {}:{}", host, port);
            }
            
            // Call the gRPC method using grpcurl
            let result = call_grpc_method(
                &host,
                port,
                &service_name,
                &method_name,
                input_data,
            ).await;
            
            let json = match result {
                Ok(response_data) => {
                    // Set service back to online after successful call
                    if let Some(service_id) = hub_service.get_service_by_address(&host, port).await {
                        hub_service.set_service_online(&service_id).await;
                    }
                    
                    serde_json::json!({
                        "success": true,
                        "data": response_data
                    })
                }
                Err(e) => {
                    let error_msg = e.to_string();
                    
                    // Instantly mark service as offline if direct connection to THIS service failed
                    // Check if the error is about connecting to the target service (not a downstream service)
                    let is_direct_connection_failure = 
                        (error_msg.contains("connection refused") || 
                         error_msg.contains("connection reset") ||
                         error_msg.contains("connection error")) &&
                        !error_msg.contains("Web content service") && // Not a downstream service error
                        !error_msg.contains("unavailable:"); // Not a gRPC service-level error
                    
                    if is_direct_connection_failure {
                        if let Some(service_id) = hub_service.get_service_by_address(&host, port).await {
                            println!("🔴 [INSTANT] Detected direct service failure at {}:{}", host, port);
                            hub_service.mark_service_offline(&service_id, "Direct connection failed").await;
                        }
                    } else {
                        // For other errors (including downstream service failures), just set back to online
                        if let Some(service_id) = hub_service.get_service_by_address(&host, port).await {
                            hub_service.set_service_online(&service_id).await;
                        }
                    }
                    
                    serde_json::json!({
                        "success": false,
                        "error": error_msg
                    })
                }
            };
            
            Ok(hyper::Response::builder()
                .status(200)
                .header("content-type", "application/json")
                .body(full_response(Bytes::from(json.to_string())))
                .unwrap())
        }
        (&Method::GET, "/api/events") => {
            println!("🔌 New SSE connection established");
            
            // Create a broadcast channel for SSE events
            let (tx, mut rx) = tokio::sync::broadcast::channel::<SSEEvent>(100);
            
            // Add sender to hub service for broadcasting
            let hub_clone = hub_service.clone();
            let tx_clone = tx.clone();
            tokio::spawn(async move {
                hub_clone.add_event_sender(tx_clone).await;
                println!("✅ SSE sender registered with hub");
            });
            
            // Send initial connection message
            let initial_data = serde_json::json!({
                "type": "connected",
                "message": "SSE connection established"
            });
            let _ = tx.send(SSEEvent {
                event_type: "connection".to_string(),
                data: initial_data.to_string(),
            });
            
            // Create a stream from the broadcast receiver with keep-alive
            let mut keep_alive_interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
            let stream = async_stream::stream! {
                loop {
                    tokio::select! {
                        // Receive events from broadcast channel
                        result = rx.recv() => {
                            match result {
                                Ok(event) => {
                                    let message = format!("event: {}\ndata: {}\n\n", event.event_type, event.data);
                                    println!("📤 Sending SSE message: event={}", event.event_type);
                                    yield Ok::<Frame<hyper::body::Bytes>, hyper::Error>(Frame::data(Bytes::from(message)));
                                }
                                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                                    println!("🔌 SSE connection closed by client");
                                    break;
                                }
                                Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                                    println!("⚠️  SSE client lagged, skipped {} messages", skipped);
                                }
                            }
                        }
                        // Send keep-alive comment every 30 seconds
                        _ = keep_alive_interval.tick() => {
                            let keep_alive = Bytes::from(": keep-alive\n\n");
                            yield Ok::<Frame<hyper::body::Bytes>, hyper::Error>(Frame::data(keep_alive));
                        }
                    }
                }
            };
            
            // Convert stream to StreamBody  
            use hyper::body::Frame;
            let body = http_body_util::StreamBody::new(stream);
            let boxed_body = BoxBody::new(body);
            
            Ok(hyper::Response::builder()
                .status(200)
                .header("content-type", "text/event-stream")
                .header("cache-control", "no-cache")
                .header("connection", "keep-alive")
                .header("keep-alive", "timeout=60")
                .header("x-accel-buffering", "no")
                .header("access-control-allow-origin", "*")
                .header("access-control-allow-headers", "cache-control")
                .body(boxed_body)
                .unwrap())
        }
        _ => {
            Ok(hyper::Response::builder()
                .status(404)
                .body(full_response(Bytes::from("Not Found")))
                .unwrap())
        }
    }
}

async fn cleanup_stale_services(hub_service: Arc<GrpcHubService>) {
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(1));
    
    loop {
        interval.tick().await;
        
        let now = Utc::now();
        let mut services = hub_service.services.write().await;
        
        let mut events_to_send = Vec::new();
        
        for (service_id, service_info) in services.iter_mut() {
            let time_since_heartbeat = now - service_info.last_heartbeat;
            
            // Mark services as offline if they haven't sent heartbeat in 10 seconds
            // Services send heartbeats every 7 seconds, so 10 seconds gives buffer for network delays
            if time_since_heartbeat > chrono::Duration::seconds(10) {
                if service_info.status == "online" {
                    println!("⚠️  Marking service '{}' as offline (last heartbeat: {}s ago)", 
                        service_info.service_name, 
                        time_since_heartbeat.num_seconds()
                    );
                    let service_name_clone = service_info.service_name.clone();
                    let service_id_clone = service_id.clone();
                    service_info.status = "offline".to_string();
                    
                    // Collect event to send after releasing lock
                    events_to_send.push((service_id_clone, service_name_clone));
                }
            }
        }
        
        drop(services); // Release lock before async call
        
        // Broadcast all status change events
        for (service_id, service_name) in events_to_send {
            let event = SSEEvent {
                event_type: "status_change".to_string(),
                data: serde_json::json!({
                    "service_id": service_id,
                    "service_name": service_name,
                    "status": "offline"
                }).to_string(),
            };
            hub_service.broadcast_event(event).await;
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    
    let grpc_addr = format!("{}:{}", args.grpc_host, args.grpc_port);
    let http_addr = format!("{}:{}", args.http_host, args.http_port);
    
    println!("Starting gRPC Hub Server");
    println!("gRPC server: {}", grpc_addr);
    println!("HTTP server: http://{}", http_addr);
    
    let hub_service = Arc::new(GrpcHubService::default());
    
    // Start cleanup task for stale services
    let cleanup_hub = hub_service.clone();
    tokio::spawn(async move {
        cleanup_stale_services(cleanup_hub).await;
    });
    
    // Start active health monitoring
    println!("🏥 Starting health monitoring (checks every 5 seconds)");
    hub_service.start_health_monitoring().await;
    
    // Start HTTP server in background
    let http_hub = hub_service.clone();
    let http_task = tokio::spawn(async move {
        if let Err(e) = start_http_server(http_hub, args.http_host, args.http_port).await {
            eprintln!("HTTP server error: {}", e);
        }
    });
    
    // Start gRPC server - clone the service
    let grpc_service_clone = (*hub_service).clone();
    Server::builder()
        .add_service(GrpcHubServer::new(grpc_service_clone))
        .serve(grpc_addr.parse()?)
        .await?;
    
    http_task.abort();
    Ok(())
}
