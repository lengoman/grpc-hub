use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tonic::{transport::Server, Request, Response, Status};
use chrono::Utc;
use tonic_reflection::server::Builder;
use clap::Parser;


mod grpc_hub {
    tonic::include_proto!("grpc_hub");
}

mod web_content_extract {
    tonic::include_proto!("web_content_extract");
}

mod dividend_service {
    tonic::include_proto!("dividend_service");
}


use grpc_hub::grpc_hub_client::GrpcHubClient;
use grpc_hub::{RegisterServiceRequest, HealthCheckRequest};

#[derive(Parser, Debug)]
#[command(name = "dividend-service")]
#[command(about = "Dividend Service - Processes dividend data and connects to web content extract service")]
struct Args {
    /// Port to listen on for gRPC requests
    #[arg(long, default_value = "8083")]
    port: u16,
    
    /// gRPC Hub host address
    #[arg(long, default_value = "127.0.0.1")]
    grpc_hub_host: String,
    
    /// gRPC Hub port
    #[arg(long, default_value = "50099")]
    grpc_hub_port: u16,
}

// Dividend Service Implementation
#[derive(Debug, Clone)]
struct DividendService {
    dividend_history: std::collections::HashMap<String, Vec<serde_json::Value>>,
    hub_connector: grpc_hub_connector::GrpcHubConnector,
    web_content_mutex: Arc<Mutex<()>>, // Mutex to prevent concurrent web content service calls
    service_id: Option<String>, // Store the actual service ID
}

impl DividendService {
    fn new() -> Self {
        Self {
            dividend_history: std::collections::HashMap::new(),
            hub_connector: grpc_hub_connector::GrpcHubConnector::new(),
            web_content_mutex: Arc::new(Mutex::new(())),
            service_id: None,
        }
    }

    fn new_with_hub_endpoint(hub_endpoint: String) -> Self {
        Self {
            dividend_history: std::collections::HashMap::new(),
            hub_connector: grpc_hub_connector::GrpcHubConnector::with_hub_endpoint(hub_endpoint),
            web_content_mutex: Arc::new(Mutex::new(())),
            service_id: None,
        }
    }

    fn new_with_service_id(hub_endpoint: String, service_id: String) -> Self {
        Self {
            dividend_history: std::collections::HashMap::new(),
            hub_connector: grpc_hub_connector::GrpcHubConnector::with_hub_endpoint(hub_endpoint),
            web_content_mutex: Arc::new(Mutex::new(())),
            service_id: Some(service_id),
        }
    }

    fn new_with_hub_connection(hub_host: String, hub_port: u16, service_id: String) -> Self {
        Self {
            dividend_history: std::collections::HashMap::new(),
            hub_connector: grpc_hub_connector::GrpcHubConnector::with_hub_connection(hub_host, hub_port),
            web_content_mutex: Arc::new(Mutex::new(())),
            service_id: Some(service_id),
        }
    }

    /// Get the current service ID from the hub
    async fn get_service_id(&self) -> Option<String> {
        println!("🔍 [DEBUG] get_service_id: Returning service_id: {:?}", self.service_id);
        self.service_id.clone()
    }

    async fn call_web_content_service(&self) -> Result<Vec<serde_json::Value>, Box<dyn std::error::Error>> {
        println!("🔍 [DEBUG] call_web_content_service: Starting service discovery");
        
        // Acquire mutex to prevent concurrent calls to web content service
        let _guard = self.web_content_mutex.lock().await;
        println!("🔒 [DEBUG] call_web_content_service: Acquired mutex lock");
        
        // Use the hub connector to get the web content service address
        let (address, port) = self.hub_connector.get_service_address("web-content-extract").await?;
        
        println!("🔍 [DEBUG] call_web_content_service: Calling web content service via hub at {}:{}", address, port);
        
        // Call web content service through the hub to track busy status
        let hub_endpoint = self.hub_connector.get_hub_endpoint();
        let hub_url = format!("{}/api/grpc-call", hub_endpoint.replace("http://", "http://").replace(":50099", ":8080"));
        
        let request_body = serde_json::json!({
            "service": "web_content_extract.WebContentExtract",
            "method": "ExtractFinancialData",
            "input": {
                "url": "https://example.com/dividend-data",
                "fields": ["dividend_amount", "payment_date", "stock_symbol"],
                "extraction_type": "financial_data"
            }
        });
        
        let client = reqwest::Client::new();
        let response = client
            .post(&hub_url)
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(format!("Hub call failed with status: {}", response.status()).into());
        }
        
        let result: serde_json::Value = response.json().await?;
        
        if !result.get("success").and_then(|v| v.as_bool()).unwrap_or(false) {
            let error = result.get("error").and_then(|v| v.as_str()).unwrap_or("Unknown error");
            return Err(format!("Web content service error: {}", error).into());
        }
        
        let response_data = result.get("data").ok_or("No data in response")?;
        
        // Convert web content response to dividend format
        let dividends = vec![
            serde_json::json!({
                "date": "2024-01-15",
                "amount": 2.50,
                "status": "paid",
                "stock_symbol": "AAPL",
                "source": "web_content",
                "confidence": response_data.get("confidenceScore").and_then(|v| v.as_f64()).unwrap_or(0.75),
                "processing_time": response_data.get("processingTimeMs").and_then(|v| v.as_i64()).unwrap_or(150)
            }),
            serde_json::json!({
                "date": "2023-10-15", 
                "amount": 2.25,
                "status": "paid",
                "stock_symbol": "AAPL",
                "source": "web_content",
                "confidence": response_data.get("confidenceScore").and_then(|v| v.as_f64()).unwrap_or(0.75),
                "processing_time": response_data.get("processingTimeMs").and_then(|v| v.as_i64()).unwrap_or(150)
            }),
        ];
        
        println!("🔓 [DEBUG] call_web_content_service: Releasing mutex lock");
        Ok(dividends)
    }
}

impl Default for DividendService {
    fn default() -> Self {
        Self::new()
    }
}

#[tonic::async_trait]
impl dividend_service::dividend_service_server::DividendService for DividendService {
    async fn calculate_dividends(
        &self,
        request: Request<dividend_service::CalculateDividendsRequest>,
    ) -> Result<Response<dividend_service::CalculateDividendsResponse>, Status> {
        let req = request.into_inner();
        println!("💰 DividendService.CalculateDividends called for amount: {}", req.amount);
        
        let bonus_percentage = 10.0;
        let calculated_dividend = req.amount * (1.0 + bonus_percentage / 100.0);
        
        Ok(Response::new(dividend_service::CalculateDividendsResponse {
            original_amount: req.amount,
            calculated_dividend,
            bonus_percentage,
            calculated_at: Utc::now().to_rfc3339(),
        }))
    }

    async fn get_dividend_history(
        &self,
        request: Request<dividend_service::GetDividendHistoryRequest>,
    ) -> Result<Response<dividend_service::GetDividendHistoryResponse>, Status> {
        let req = request.into_inner();
        println!("🔍 [DEBUG] GetDividendHistory: Method called for user: {}", req.user_id);
        
        // Get service ID for status reporting
        let service_id = self.get_service_id().await;
        
        // Report busy status (fire-and-forget, no blocking)
        if let Some(id) = &service_id {
            let hub_connector = self.hub_connector.clone();
            let service_id_clone = id.clone();
            
            // Fire-and-forget task - don't wait for completion
            tokio::spawn(async move {
                let _ = hub_connector.set_service_busy(&service_id_clone).await;
            });
        }
        
        let result = async {
            // Call web content service - fail if unavailable
            println!("🔍 [DEBUG] GetDividendHistory: About to call web content service");
            let web_content_data = self.call_web_content_service().await
                .map_err(|e| {
                    println!("❌ [DEBUG] GetDividendHistory: Failed to get web content data: {}", e);
                    Status::unavailable(format!("Web content service unavailable: {}", e))
                })?;
            
            println!("🔍 [DEBUG] GetDividendHistory: Successfully received web content data");
            
            Ok(Response::new(dividend_service::GetDividendHistoryResponse {
                dividends: web_content_data.iter().map(|d| d.to_string()).collect(),
                total_dividends: web_content_data.len() as i32,
                retrieved_at: Utc::now().to_rfc3339(),
            }))
        }.await;
        
        // Report online status (fire-and-forget, no blocking)
        if let Some(id) = &service_id {
            let hub_connector = self.hub_connector.clone();
            let service_id_clone = id.clone();
            
            // Fire-and-forget task - don't wait for completion
            tokio::spawn(async move {
                let _ = hub_connector.set_service_online(&service_id_clone).await;
            });
        }
        
        result
    }

    async fn process_dividend_data(
        &self,
        request: Request<dividend_service::ProcessDividendDataRequest>,
    ) -> Result<Response<dividend_service::ProcessDividendDataResponse>, Status> {
        let req = request.into_inner();
        println!("💰 DividendService.ProcessDividendData called with {} records", req.record_count);
        
        let successful_records = (req.record_count as f64 * 0.95) as i32;
        let failed_records = req.record_count - successful_records;
        
        Ok(Response::new(dividend_service::ProcessDividendDataResponse {
            processed_records: req.record_count,
            successful_records,
            failed_records,
            processing_time_ms: 250,
            processed_at: Utc::now().to_rfc3339(),
        }))
    }
}

// Helper function to extract method names from the proto file
fn get_service_methods() -> Vec<String> {
    include_str!("../../proto/dividend_service.proto")
        .lines()
        .filter(|line| line.contains("rpc"))
        .map(|line| {
            line.split_whitespace()
                .nth(1)
                .unwrap_or("")
                .to_string()
        })
        .collect()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command-line arguments
    let args = Args::parse();
    
    println!("💰 Dividend Service - Starting service that processes dividend data");
    println!("📋 Configuration:");
    println!("   - Service Port: {}", args.port);
    println!("   - gRPC Hub: {}:{}", args.grpc_hub_host, args.grpc_hub_port);
    
    // Build hub endpoint from arguments (for backward compatibility)
    let hub_endpoint = format!("http://{}:{}", args.grpc_hub_host, args.grpc_hub_port);
    
    // Connect to the gRPC hub
    let mut hub_client = GrpcHubClient::connect(hub_endpoint.clone()).await?;
    
    // Register this service with the hub
    let mut metadata = HashMap::new();
    metadata.insert("team".to_string(), "finance".to_string());
    metadata.insert("environment".to_string(), "production".to_string());
    metadata.insert("purpose".to_string(), "dividend_calculation".to_string());
    
    let methods = get_service_methods();
    println!("📋 Discovered {} methods from proto file", methods.len());
    
    let registration_details = RegisterServiceRequest {
        service_name: "dividend-service".to_string(),
        service_version: "1.0.0".to_string(),
        service_address: "127.0.0.1".to_string(),
        service_port: args.port.to_string(),
        methods: methods.clone(),
        metadata: metadata.clone(),
    };
    
    let register_request = Request::new(registration_details.clone());
    let register_response = hub_client.register_service(register_request).await?;
    let register_response = register_response.into_inner();
    let service_id = register_response.service_id.clone();
    println!("✅ Registered dividend-service: {}", service_id);
    
    // Create the dividend service instance with the hub connection parameters and service ID
    let dividend_service_instance = DividendService::new_with_hub_connection(
        args.grpc_hub_host.clone(), 
        args.grpc_hub_port, 
        service_id.clone()
    );
    
    // Note: Polling task removed to prevent race conditions with user requests
    // The dividend service works on-demand when users call GetDividendHistory
    
    // Spawn heartbeat task
    let service_id_for_heartbeat = service_id.clone();
    let registration_details_for_heartbeat = registration_details.clone();
    tokio::spawn(async move {
        let hub_addr = "http://127.0.0.1:50099";
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(7));
        let mut heartbeat_client: Option<GrpcHubClient<tonic::transport::Channel>> = None;
        let mut current_service_id = service_id_for_heartbeat.clone();
        let mut needs_re_register = false;
        
        loop {
            interval.tick().await;
            
            if heartbeat_client.is_none() {
                println!("🔌 Connecting to gRPC hub at {}...", hub_addr);
                match GrpcHubClient::connect(hub_addr).await {
                    Ok(client) => {
                        heartbeat_client = Some(client);
                        println!("✅ Connected to gRPC hub!");
                        
                        if let Some(ref mut client) = heartbeat_client {
                            let re_register_request = Request::new(registration_details_for_heartbeat.clone());
                            match client.register_service(re_register_request).await {
                                Ok(response) => {
                                    current_service_id = response.into_inner().service_id;
                                    println!("✅ Service registered with ID: {}", current_service_id);
                                    needs_re_register = false;
                                }
                                Err(e) => {
                                    println!("❌ Failed to register service: {}", e);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        println!("❌ Failed to connect to gRPC hub: {}. Will retry...", e);
                        needs_re_register = true;
                        continue;
                    }
                }
            }
            
            if let Some(ref mut client) = heartbeat_client {
                let health_request = Request::new(HealthCheckRequest {
                    service_id: current_service_id.clone(),
                });
                
                match client.health_check(health_request).await {
                    Ok(_) => {
                        if needs_re_register {
                            println!("💓 Service heartbeat sent (after re-registration)");
                            needs_re_register = false;
                        } else {
                            println!("💓 Service heartbeat sent");
                        }
                    }
                    Err(e) => {
                        println!("⚠️ Failed to send heartbeat: {}. Will reconnect and re-register...", e);
                        heartbeat_client = None;
                        needs_re_register = true;
                    }
                }
            }
        }
    });
    
    // Start the gRPC server on the specified port
    let addr = format!("127.0.0.1:{}", args.port).parse()?;
    
    println!("\n🚀 Dividend Service starting on {}", addr);
    println!("🔄 Service ready to process dividend data...");
    println!("🛑 Press Ctrl+C to stop");
    
    // Enable gRPC reflection for dynamic discovery
    let descriptor_bytes = include_bytes!(concat!(env!("OUT_DIR"), "/proto_descriptor.bin"));
    let reflection_service = Builder::configure()
        .register_encoded_file_descriptor_set(descriptor_bytes)
        .build_v1()
        .unwrap();
    
    Server::builder()
        .add_service(dividend_service::dividend_service_server::DividendServiceServer::new(dividend_service_instance))
        .add_service(reflection_service)
        .serve(addr)
        .await?;

    Ok(())
}
