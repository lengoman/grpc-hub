use std::collections::HashMap;
use tonic::Request;

mod grpc_hub {
    tonic::include_proto!("grpc_hub");
}

use grpc_hub::grpc_hub_client::GrpcHubClient;
use grpc_hub::{ServiceCallRequest, SubscribeRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🌉 Service Bridge Demo - Demonstrating service-to-service communication through gRPC Hub");
    
    // Connect to the gRPC hub
    let mut hub_client = GrpcHubClient::connect("http://127.0.0.1:50099").await?;
    
    println!("\n📡 Step 1: Dividend service requesting data from web-content-extract...");
    
    // Simulate dividend service calling web-content-extract through the hub
    let call_request = Request::new(ServiceCallRequest {
        target_service: "web-content-extract".to_string(),
        method: "ExtractFinancialData".to_string(),
        request_data: serde_json::json!({
            "url": "https://financial-data.com/dividend-info",
            "extract_type": "financial_data",
            "fields": ["dividend_amount", "payment_date", "stock_symbol", "company_name"]
        }).to_string(),
        caller_service: "dividend-service".to_string(),
        headers: HashMap::new(),
    });
    
    let call_response = hub_client.call_service(call_request).await?;
    let call_response = call_response.into_inner();
    
    if call_response.success {
        println!("✅ Successfully received data from web-content-extract:");
        let extracted_data: serde_json::Value = serde_json::from_str(&call_response.response_data)?;
        println!("   📊 Dividend Amount: ${}", extracted_data.get("dividend_amount").unwrap_or(&serde_json::Value::Null));
        println!("   📅 Payment Date: {}", extracted_data.get("payment_date").unwrap_or(&serde_json::Value::Null));
        println!("   🏷️  Stock Symbol: {}", extracted_data.get("stock_symbol").unwrap_or(&serde_json::Value::Null));
        println!("   🎯 Confidence Score: {}", extracted_data.get("confidence_score").unwrap_or(&serde_json::Value::Null));
        
        println!("\n💰 Step 2: Processing dividend calculation...");
        
        // Now call the dividend service to process the extracted data
        let dividend_request = Request::new(ServiceCallRequest {
            target_service: "dividend-service".to_string(),
            method: "CalculateDividends".to_string(),
            request_data: serde_json::json!({
                "amount": extracted_data.get("dividend_amount").unwrap_or(&serde_json::Value::Number(serde_json::Number::from_f64(0.0).unwrap())),
                "source": "web-content-extract",
                "extraction_confidence": extracted_data.get("confidence_score").unwrap_or(&serde_json::Value::Number(serde_json::Number::from_f64(0.0).unwrap()))
            }).to_string(),
            caller_service: "bridge-demo".to_string(),
            headers: HashMap::new(),
        });
        
        let dividend_response = hub_client.call_service(dividend_request).await?;
        let dividend_response = dividend_response.into_inner();
        
        if dividend_response.success {
            let dividend_data: serde_json::Value = serde_json::from_str(&dividend_response.response_data)?;
            println!("✅ Dividend calculation completed:");
            println!("   💵 Original Amount: ${}", dividend_data.get("original_amount").unwrap_or(&serde_json::Value::Null));
            println!("   🎁 Calculated Dividend: ${}", dividend_data.get("calculated_dividend").unwrap_or(&serde_json::Value::Null));
            println!("   📈 Bonus Percentage: {}%", dividend_data.get("bonus_percentage").unwrap_or(&serde_json::Value::Null));
            
            println!("\n🎉 Service Bridge Demo Complete!");
            println!("   📊 Data Flow: web-content-extract → gRPC Hub → dividend-service");
            println!("   🔄 Communication: Service-to-service through hub bridge");
            println!("   ✅ Result: Successful dividend calculation with extracted data");
            
        } else {
            println!("❌ Dividend calculation failed: {}", dividend_response.error_message);
        }
        
    } else {
        println!("❌ Failed to get data from web-content-extract: {}", call_response.error_message);
    }
    
    println!("\n🔄 Service Bridge Demo finished. The hub successfully bridged communication between services!");
    
    Ok(())
}

