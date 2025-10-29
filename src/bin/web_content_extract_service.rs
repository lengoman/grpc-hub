use std::collections::HashMap;
use tonic::{transport::Server, Request, Response, Status};
use tonic_reflection::server::Builder;
use clap::Parser;


mod grpc_hub {
    tonic::include_proto!("grpc_hub");
}

// Define the web content extract service proto
mod web_content_extract {
    tonic::include_proto!("web_content_extract");
}

use grpc_hub::grpc_hub_client::GrpcHubClient;
use grpc_hub::{RegisterServiceRequest, HealthCheckRequest};

#[derive(Parser, Debug)]
#[command(name = "web-content-extract-service")]
#[command(about = "Web Content Extract Service - Extracts financial data from web content")]
struct Args {
    /// Port to listen on for gRPC requests
    #[arg(long, default_value = "8085")]
    port: u16,
    
    /// gRPC Hub host address
    #[arg(long, default_value = "127.0.0.1")]
    grpc_hub_host: String,
    
    /// gRPC Hub port
    #[arg(long, default_value = "50099")]
    grpc_hub_port: u16,
}

// Helper function to extract method names from the proto service definition
// Using a macro to extract method names at compile time from the generated code
fn get_service_methods() -> Vec<String> {
    // Since tonic generates code with method names, we can derive them from the trait
    // The Service trait has METHOD_INFO constant that we can use
    
    // A practical approach: query the reflection service or use the generated method names
    // This list is automatically synced with the proto file
    include_str!("../../proto/web_content_extract.proto")
        .lines()
        .filter(|line| line.contains("rpc"))
        .map(|line| {
            // Extract method name from "rpc MethodName(...) returns (...);"
            line.split_whitespace()
                .nth(1)
                .unwrap_or("")
                .to_string()
        })
        .collect()
}

// Mock Web Content Extract Service Implementation
#[derive(Debug)]
struct WebContentExtractService {
    // In-memory storage for extracted content
    extracted_data: std::collections::HashMap<String, serde_json::Value>,
    hub_connector: grpc_hub_connector::GrpcHubConnector,
    service_id: Option<String>,
}

impl WebContentExtractService {
    fn new() -> Self {
        let mut extracted_data = std::collections::HashMap::new();
        
        // Pre-populate with some sample financial data
        extracted_data.insert("https://financial-data.com/dividend-info".to_string(), 
            serde_json::json!({
                "dividend_amount": 2.50,
                "payment_date": "2024-01-15",
                "stock_symbol": "AAPL",
                "company_name": "Apple Inc.",
                "ex_dividend_date": "2024-01-08",
                "dividend_frequency": "quarterly",
                "yield_percentage": 0.45
            }));
            
        extracted_data.insert("https://financial-data.com/earnings".to_string(),
            serde_json::json!({
                "revenue": 123900000000i64,
                "net_income": 33980000000i64,
                "eps": 2.18,
                "quarter": "Q4 2023",
                "growth_rate": 0.08
            }));
        
        Self { 
            extracted_data,
            hub_connector: grpc_hub_connector::GrpcHubConnector::new(),
            service_id: None,
        }
    }

    fn new_with_service_id(hub_endpoint: String, service_id: String) -> Self {
        let mut extracted_data = std::collections::HashMap::new();
        
        // Pre-populate with some sample financial data
        extracted_data.insert("https://financial-data.com/dividend-info".to_string(), 
            serde_json::json!({
                "dividend_amount": 2.50,
                "payment_date": "2024-01-15",
                "stock_symbol": "AAPL",
                "company_name": "Apple Inc.",
                "ex_dividend_date": "2024-01-08",
                "dividend_frequency": "quarterly",
                "yield_percentage": 0.45
            }));
            
        extracted_data.insert("https://financial-data.com/earnings".to_string(),
            serde_json::json!({
                "revenue": 123900000000i64,
                "net_income": 33980000000i64,
                "eps": 2.18,
                "quarter": "Q4 2023",
                "growth_rate": 0.08
            }));
        
        Self { 
            extracted_data,
            hub_connector: grpc_hub_connector::GrpcHubConnector::with_hub_endpoint(hub_endpoint),
            service_id: Some(service_id),
        }
    }

    fn new_with_hub_connection(hub_host: String, hub_port: u16, service_id: String) -> Self {
        let mut extracted_data = std::collections::HashMap::new();
        
        // Pre-populate with some sample financial data
        extracted_data.insert("https://financial-data.com/dividend-info".to_string(), 
            serde_json::json!({
                "dividend_amount": 2.50,
                "payment_date": "2024-01-15",
                "stock_symbol": "AAPL",
                "company_name": "Apple Inc.",
                "ex_dividend_date": "2024-01-08",
                "dividend_frequency": "quarterly",
                "yield_percentage": 0.45
            }));
            
        extracted_data.insert("https://financial-data.com/earnings".to_string(),
            serde_json::json!({
                "revenue": 123900000000i64,
                "net_income": 33980000000i64,
                "eps": 2.18,
                "quarter": "Q4 2023",
                "growth_rate": 0.08
            }));
        
        Self { 
            extracted_data,
            hub_connector: grpc_hub_connector::GrpcHubConnector::with_hub_connection(hub_host, hub_port),
            service_id: Some(service_id),
        }
    }
}

impl Default for WebContentExtractService {
    fn default() -> Self {
        Self::new()
    }
}

#[tonic::async_trait]
impl web_content_extract::web_content_extract_server::WebContentExtract for WebContentExtractService {
    async fn extract_financial_data(
        &self,
        request: Request<web_content_extract::ExtractFinancialDataRequest>,
    ) -> Result<Response<web_content_extract::ExtractFinancialDataResponse>, Status> {
        let req = request.into_inner();
        println!("🌐 WebContentExtract.ExtractFinancialData called for URL: {}", req.url);
        
        // Report busy status (fire-and-forget, no blocking)
        if let Some(service_id) = &self.service_id {
            println!("🟠 [DEBUG] WebContentExtract: Reporting busy status for service_id: {}", service_id);
            let hub_connector = self.hub_connector.clone();
            let service_id_clone = service_id.clone();
            
            // Fire-and-forget task - don't wait for completion
            tokio::spawn(async move {
                let _ = hub_connector.set_service_busy(&service_id_clone).await;
            });
        } else {
            println!("⚠️ [DEBUG] WebContentExtract: No service_id available for busy status reporting");
        }
        
        let result = async {
            // This is to test the service making it slow - BEFORE processing
            println!("🐌 [DEBUG] WebContentExtract: Starting 5-second sleep for load balancing test");
            // tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            println!("✅ [DEBUG] WebContentExtract: Sleep completed, processing request");
            
            // Simulate web scraping and data extraction
            let extracted_data = self.extracted_data.get(&req.url)
                .cloned()
                .unwrap_or_else(|| {
                    // Generate mock data if URL not found
                    serde_json::json!({
                        "dividend_amount": 1.25,
                        "payment_date": "2024-02-15",
                        "stock_symbol": "MSFT",
                        "company_name": "Microsoft Corporation",
                        "ex_dividend_date": "2024-02-08",
                        "dividend_frequency": "quarterly",
                        "yield_percentage": 0.32
                    })
                });
            
            let confidence_score = if req.url.contains("financial-data.com") { 0.95 } else { 0.75 };

            Ok(Response::new(web_content_extract::ExtractFinancialDataResponse {
                success: true,
                data: extracted_data.to_string(),
                confidence_score,
                extraction_method: "ai_parser".to_string(),
                processing_time_ms: 150,
            }))
        }.await;
        
        // Report online status (fire-and-forget, no blocking)
        if let Some(service_id) = &self.service_id {
            let hub_connector = self.hub_connector.clone();
            let service_id_clone = service_id.clone();
            
            // Fire-and-forget task - don't wait for completion
            tokio::spawn(async move {
                let _ = hub_connector.set_service_online(&service_id_clone).await;
            });
        }
        
        result
    }

    async fn extract_text_content(
        &self,
        request: Request<web_content_extract::ExtractTextContentRequest>,
    ) -> Result<Response<web_content_extract::ExtractTextContentResponse>, Status> {
        let req = request.into_inner();
        println!("🌐 WebContentExtract.ExtractTextContent called for URL: {}", req.url);
        
        Ok(Response::new(web_content_extract::ExtractTextContentResponse {
            success: true,
            content: "Sample extracted text content from the web page...".to_string(),
            word_count: 150,
            language: "en".to_string(),
            extraction_method: "text_parser".to_string(),
        }))
    }

    async fn extract_structured_data(
        &self,
        request: Request<web_content_extract::ExtractStructuredDataRequest>,
    ) -> Result<Response<web_content_extract::ExtractStructuredDataResponse>, Status> {
        let req = request.into_inner();
        println!("🌐 WebContentExtract.ExtractStructuredData called for URL: {}", req.url);
        
        let structured_data = serde_json::json!({
            "title": "Financial Report - Q4 2023",
            "author": "Financial Team",
            "published_date": "2024-01-15",
            "sections": ["executive_summary", "financial_metrics", "outlook"],
            "tags": ["earnings", "dividend", "quarterly"]
        });
        
        Ok(Response::new(web_content_extract::ExtractStructuredDataResponse {
            success: true,
            data: structured_data.to_string(),
            extraction_method: "structured_parser".to_string(),
        }))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command-line arguments
    let args = Args::parse();
    
    println!("🌐 Web Content Extract Service - Starting mock web scraping service");
    println!("📋 Configuration:");
    println!("   - Service Port: {}", args.port);
    println!("   - gRPC Hub: {}:{}", args.grpc_hub_host, args.grpc_hub_port);
    
    // Build hub endpoint from arguments
    let hub_endpoint = format!("http://{}:{}", args.grpc_hub_host, args.grpc_hub_port);
    
    // Connect to the gRPC hub
    let mut hub_client = GrpcHubClient::connect(hub_endpoint).await?;
    
    // Register this service with the hub
    let mut metadata = HashMap::new();
    metadata.insert("team".to_string(), "data-extraction".to_string());
    metadata.insert("environment".to_string(), "production".to_string());
    metadata.insert("purpose".to_string(), "web_scraping".to_string());
    metadata.insert("capabilities".to_string(), "financial_data,text_content,structured_data".to_string());
    
    // Automatically discover methods from the proto file
    let methods = get_service_methods();
    println!("📋 Discovered {} methods from proto file", methods.len());
    
    // Store registration details for re-registration
    let registration_details = RegisterServiceRequest {
        service_name: "web-content-extract".to_string(),
        service_version: "2.0.0".to_string(),
        service_address: "127.0.0.1".to_string(),
        service_port: args.port.to_string(),
        methods: methods.clone(),
        metadata: metadata.clone(),
    };
    
    let register_request = Request::new(registration_details.clone());
    let register_response = hub_client.register_service(register_request).await?;
    let register_response = register_response.into_inner();
    let service_id = register_response.service_id.clone();
    println!("✅ Registered web-content-extract: {}", service_id);
    
    // Start the gRPC server in a background task
    let addr = format!("127.0.0.1:{}", args.port).parse()?;
    let web_extract_service = WebContentExtractService::new_with_hub_connection(
        args.grpc_hub_host.clone(), 
        args.grpc_hub_port, 
        service_id.clone()
    );
    
    println!("🚀 Web Content Extract Service starting on {}", addr);
    
    let server_task = tokio::spawn(async move {
        // Enable gRPC reflection for dynamic discovery
        let descriptor_bytes = include_bytes!(concat!(env!("OUT_DIR"), "/proto_descriptor.bin"));
        let reflection_service = Builder::configure()
            .register_encoded_file_descriptor_set(descriptor_bytes)
            .build_v1()
            .unwrap();
        
        Server::builder()
            .add_service(web_content_extract::web_content_extract_server::WebContentExtractServer::new(web_extract_service))
            .add_service(reflection_service)
            .serve(addr)
            .await
            .unwrap();
    });
    
    // Send periodic heartbeats to the hub in a separate task with reconnection logic
    let service_id_for_heartbeat = service_id.clone();
    let registration_details_for_heartbeat = registration_details.clone();
    let heartbeat_task = tokio::spawn(async move {
        let hub_addr = "http://127.0.0.1:50099";
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(7)); // Send heartbeat every 7 seconds
        let mut heartbeat_client: Option<GrpcHubClient<tonic::transport::Channel>> = None;
        let mut current_service_id = service_id_for_heartbeat.clone();
        let mut needs_re_register = false;
        let mut is_first_heartbeat = true;
        
        loop {
            interval.tick().await;
            
            // On first heartbeat after service start, mark that we need to establish connection
            if is_first_heartbeat {
                heartbeat_client = None;
                is_first_heartbeat = false;
            }
            
            // Reconnect if client is None (first time or after disconnection)
            if heartbeat_client.is_none() {
                println!("🔌 Connecting to gRPC hub at {}...", hub_addr);
                match GrpcHubClient::connect(hub_addr).await {
                    Ok(client) => {
                        heartbeat_client = Some(client);
                        println!("✅ Connected to gRPC hub!");
                        
                        // Always re-register when connecting (covers both initial connection and reconnection)
                        println!("📝 Registering/re-registering service with hub...");
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
            
            // Send heartbeat
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
                        heartbeat_client = None; // Force reconnection on next iteration
                        needs_re_register = true;
                    }
                }
            }
        }
    });
    
    // Wait for either task to complete (usually they run forever)
    tokio::select! {
        result = server_task => result?,
        result = heartbeat_task => result?,
    }

    Ok(())
}
