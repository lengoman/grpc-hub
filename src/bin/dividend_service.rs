use std::collections::HashMap;
use tonic::{transport::Server, Request, Response, Status};
use chrono::Utc;
use tonic_reflection::server::Builder;

mod grpc_hub {
    tonic::include_proto!("grpc_hub");
}

mod web_content_extract {
    tonic::include_proto!("web_content_extract");
}

mod dividend_service {
    tonic::include_proto!("dividend_service");
}

// Include the grpc_hub_connector from the parent module
mod grpc_hub_connector {
    include!("../grpc_hub_connector.rs");
}

use grpc_hub::grpc_hub_client::GrpcHubClient;
use grpc_hub::{RegisterServiceRequest, HealthCheckRequest};

// Dividend Service Implementation
#[derive(Debug, Clone)]
struct DividendService {
    dividend_history: std::collections::HashMap<String, Vec<serde_json::Value>>,
    hub_connector: grpc_hub_connector::GrpcHubConnector,
}

impl DividendService {
    fn new() -> Self {
        Self {
            dividend_history: std::collections::HashMap::new(),
            hub_connector: grpc_hub_connector::GrpcHubConnector::new(),
        }
    }

    async fn call_web_content_service(&self) -> Result<Vec<serde_json::Value>, Box<dyn std::error::Error>> {
        println!("🔍 [DEBUG] call_web_content_service: Starting service discovery");
        
        // Use the hub connector to get the web content service address
        let (address, port) = self.hub_connector.get_service_address("web-content-extract").await?;
        
        // Call web content service using the discovered address and port
        let web_endpoint = format!("http://{}:{}", address, port);
        println!("🔍 [DEBUG] call_web_content_service: Connecting to web content service at {}", web_endpoint);
        
        let mut web_client = web_content_extract::web_content_extract_client::WebContentExtractClient::connect(web_endpoint).await?;
        println!("🔍 [DEBUG] call_web_content_service: Successfully connected to web content service");
        
        let request = tonic::Request::new(web_content_extract::ExtractFinancialDataRequest {
            url: "https://example.com/dividend-data".to_string(),
            fields: vec!["dividend_amount".to_string(), "payment_date".to_string(), "stock_symbol".to_string()],
            extraction_type: "financial_data".to_string(),
        });
        
        let response = web_client.extract_financial_data(request).await?;
        let response_data = response.into_inner();
        
        if response_data.success {
            // Convert web content response to dividend format
            let dividends = vec![
                serde_json::json!({
                    "date": "2024-01-15",
                    "amount": 2.50,
                    "status": "paid",
                    "stock_symbol": "AAPL",
                    "source": "web_content",
                    "confidence": response_data.confidence_score,
                    "processing_time": response_data.processing_time_ms
                }),
                serde_json::json!({
                    "date": "2023-10-15", 
                    "amount": 2.25,
                    "status": "paid",
                    "stock_symbol": "AAPL",
                    "source": "web_content",
                    "confidence": response_data.confidence_score,
                    "processing_time": response_data.processing_time_ms
                }),
            ];
            
            Ok(dividends)
        } else {
            Err(format!("Web content service error: {}", response_data.data).into())
        }
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
    println!("💰 Dividend Service - Starting service that processes dividend data");
    
    // Connect to the gRPC hub
    let mut hub_client = GrpcHubClient::connect("http://127.0.0.1:50099").await?;
    
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
        service_port: "8083".to_string(),
        methods: methods.clone(),
        metadata: metadata.clone(),
    };
    
    let register_request = Request::new(registration_details.clone());
    let register_response = hub_client.register_service(register_request).await?;
    let register_response = register_response.into_inner();
    let service_id = register_response.service_id.clone();
    println!("✅ Registered dividend-service: {}", service_id);
    
    // Create the dividend service instance
    let dividend_service_instance = DividendService::new();
    
    // Spawn task to poll web-content-extract service using hub connector
    let dividend_service_for_polling = dividend_service_instance.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
        loop {
            interval.tick().await;
            
            println!("🔍 [DEBUG] Polling task: Starting polling cycle");
            println!("\n🔄 Polling web-content-extract for data...");
            
            // Use hub connector for service discovery
            let (address, port) = match dividend_service_for_polling.hub_connector.get_service_address("web-content-extract").await {
                Ok(addr_port) => addr_port,
                Err(e) => {
                    println!("❌ [DEBUG] Polling task: Failed to get web content service: {}", e);
                    continue;
                }
            };
            
            let endpoint = format!("http://{}:{}", address, port);
            let mut client = match web_content_extract::web_content_extract_client::WebContentExtractClient::connect(endpoint.clone()).await {
                Ok(client) => client,
                Err(e) => {
                    println!("❌ Failed to connect to web-content-extract at {}: {}", endpoint, e);
                    continue;
                }
            };
            
            let request = tonic::Request::new(web_content_extract::ExtractFinancialDataRequest {
                url: "https://example.com/dividend-data".to_string(),
                fields: vec!["dividend_amount".to_string(), "payment_date".to_string(), "stock_symbol".to_string()],
                extraction_type: "financial_data".to_string(),
            });
            
            let response_data = match client.extract_financial_data(request).await {
                Ok(response) => response.into_inner(),
                Err(e) => {
                    println!("❌ Failed to call web-content-extract: {}", e);
                    continue;
                }
            };
            
            match (response_data.success, serde_json::from_str::<serde_json::Value>(&response_data.data)) {
                (true, Ok(json_data)) => {
                    println!("✅ Successfully received data from web-content-extract");
                    if let Some(dividend_amount) = json_data.get("dividend_amount").and_then(|v| v.as_f64()) {
                        let calculated_dividend = dividend_amount * 1.1;
                        println!("💰 Calculated dividend: ${:.2}", calculated_dividend);
                    }
                }
                (false, _) => println!("❌ web-content-extract returned unsuccessful response"),
                (true, Err(e)) => println!("❌ Failed to parse response data: {}", e),
            }
        }
    });
    
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
    
    // Start the gRPC server on port 8083
    let addr = "127.0.0.1:8083".parse()?;
    
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
