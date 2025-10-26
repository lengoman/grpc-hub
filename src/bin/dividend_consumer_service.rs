use std::collections::HashMap;
use tonic::{transport::Server, Request, Response, Status};
use uuid::Uuid;
use chrono::Utc;

mod grpc_hub {
    tonic::include_proto!("grpc_hub");
}

// Define the dividend service proto
mod dividend_service {
    tonic::include_proto!("dividend_service");
}

use grpc_hub::grpc_hub_client::GrpcHubClient;
use grpc_hub::{RegisterServiceRequest, ServiceCallRequest};

// Mock Dividend Service Implementation
#[derive(Debug)]
struct DividendConsumerService {
    // In-memory storage for dividend calculations
    dividend_history: std::collections::HashMap<String, Vec<serde_json::Value>>,
}

impl DividendConsumerService {
    fn new() -> Self {
        Self {
            dividend_history: std::collections::HashMap::new(),
        }
    }
}

impl Default for DividendConsumerService {
    fn default() -> Self {
        Self::new()
    }
}

#[tonic::async_trait]
impl dividend_service::dividend_service_server::DividendService for DividendConsumerService {
    async fn calculate_dividends(
        &self,
        request: Request<dividend_service::CalculateDividendsRequest>,
    ) -> Result<Response<dividend_service::CalculateDividendsResponse>, Status> {
        let req = request.into_inner();
        println!("💰 DividendService.CalculateDividends called for amount: {}", req.amount);
        
        // Calculate dividend with bonus
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
        println!("💰 DividendService.GetDividendHistory called for user: {}", req.user_id);
        
        // Return mock dividend history
        let dividends = vec![
            serde_json::json!({
                "date": "2024-01-15",
                "amount": 2.50,
                "status": "paid",
                "stock_symbol": "AAPL"
            }),
            serde_json::json!({
                "date": "2023-10-15",
                "amount": 2.25,
                "status": "paid",
                "stock_symbol": "AAPL"
            }),
        ];
        
        Ok(Response::new(dividend_service::GetDividendHistoryResponse {
            dividends: dividends.iter().map(|d| d.to_string()).collect(),
            total_dividends: dividends.len() as i32,
            retrieved_at: Utc::now().to_rfc3339(),
        }))
    }

    async fn process_dividend_data(
        &self,
        request: Request<dividend_service::ProcessDividendDataRequest>,
    ) -> Result<Response<dividend_service::ProcessDividendDataResponse>, Status> {
        let req = request.into_inner();
        println!("💰 DividendService.ProcessDividendData called with {} records", req.record_count);
        
        // Simulate data processing
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("💰 Dividend Consumer Service - Starting service that processes dividend data from web content");
    
    // Connect to the gRPC hub
    let mut hub_client = GrpcHubClient::connect("http://127.0.0.1:50099").await?;
    
    // Register this service with the hub
    let mut metadata = HashMap::new();
    metadata.insert("team".to_string(), "finance".to_string());
    metadata.insert("environment".to_string(), "production".to_string());
    metadata.insert("purpose".to_string(), "dividend_processing".to_string());
    metadata.insert("dependencies".to_string(), "web-content-extract".to_string());
    
    let register_request = Request::new(RegisterServiceRequest {
        service_name: "dividend-consumer".to_string(),
        service_version: "1.0.0".to_string(),
        service_address: "127.0.0.1".to_string(),
        service_port: "8086".to_string(),
        methods: vec![
            "CalculateDividends".to_string(),
            "GetDividendHistory".to_string(),
            "ProcessDividendData".to_string(),
        ],
        metadata,
    });
    
    let register_response = hub_client.register_service(register_request).await?;
    let register_response = register_response.into_inner();
    println!("✅ Registered dividend-consumer: {}", register_response.service_id);
    
    // Demonstrate service-to-service communication
    println!("\n🌉 Demonstrating service-to-service communication through the hub...");
    
    // Call web-content-extract service through the hub
    let extract_request = Request::new(ServiceCallRequest {
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
    
    match hub_client.call_service(extract_request).await {
        Ok(response) => {
            let response = response.into_inner();
            if response.success {
                println!("✅ Successfully received data from web-content-extract:");
                println!("   Response: {}", response.response_data);
                
                // Parse the extracted data
                if let Ok(extracted_data) = serde_json::from_str::<serde_json::Value>(&response.response_data) {
                    if let Some(dividend_amount) = extracted_data.get("dividend_amount").and_then(|v| v.as_f64()) {
                        println!("📊 Processing extracted dividend data...");
                        
                        // Calculate dividend with bonus
                        let calculated_dividend = dividend_amount * 1.1; // 10% bonus
                        println!("💰 Original dividend: ${:.2}", dividend_amount);
                        println!("🎁 Calculated dividend: ${:.2}", calculated_dividend);
                        println!("📈 Bonus applied: 10%");
                        
                        if let Some(payment_date) = extracted_data.get("payment_date").and_then(|v| v.as_str()) {
                            println!("📅 Payment date: {}", payment_date);
                        }
                        
                        if let Some(stock_symbol) = extracted_data.get("stock_symbol").and_then(|v| v.as_str()) {
                            println!("🏷️  Stock symbol: {}", stock_symbol);
                        }
                    }
                }
            } else {
                println!("❌ Failed to get data from web-content-extract: {}", response.error_message);
            }
        }
        Err(e) => println!("❌ Service call error: {}", e),
    }
    
    // Start the gRPC server
    let addr = "127.0.0.1:8086".parse()?;
    let dividend_service = DividendConsumerService::new();
    
    println!("\n🚀 Dividend Consumer Service starting on {}", addr);
    println!("🔄 Service ready to process dividend data from web content extraction...");
    println!("🛑 Press Ctrl+C to stop");
    
    Server::builder()
        .add_service(dividend_service::dividend_service_server::DividendServiceServer::new(dividend_service))
        .serve(addr)
        .await?;

    Ok(())
}

