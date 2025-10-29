use clap::Parser;
use std::time::{Duration, Instant};
use tokio::time::sleep;


mod grpc_hub {
    tonic::include_proto!("grpc_hub");
}

use grpc_hub::grpc_hub_client::GrpcHubClient;
use grpc_hub::ServiceCallRequest;

#[derive(Parser, Debug)]
#[command(name = "ping-client")]
#[command(about = "Ping Client - Test gRPC services through the hub")]
struct Args {
    /// Service name (e.g., "web_content_extract.WebContentExtract" or "dividend_service.DividendService")
    #[arg(short, long)]
    service: String,

    /// Method name (e.g., "ExtractFinancialData" or "GetDividendHistory")
    #[arg(short, long)]
    method: String,

    /// Number of times to call the service
    #[arg(short, long, default_value = "1")]
    count: u32,

    /// Interval between calls in milliseconds
    #[arg(short, long, default_value = "1000")]
    interval_ms: u64,

    /// gRPC Hub host address
    #[arg(long, default_value = "127.0.0.1")]
    grpc_hub_host: String,
    
    /// gRPC Hub port
    #[arg(long, default_value = "50099")]
    grpc_hub_port: u16,

    /// Request input data as JSON (default: empty object)
    #[arg(long, default_value = "{}")]
    input: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    println!("🚀 Ping Client - Testing gRPC Service");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Service: {}", args.service);
    println!("Method: {}", args.method);
    println!("Count: {}", args.count);
    println!("Interval: {}ms", args.interval_ms);
    println!("gRPC Hub: {}:{}", args.grpc_hub_host, args.grpc_hub_port);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // Parse input JSON
    let input_data: serde_json::Value = serde_json::from_str(&args.input)?;

    // Connect to gRPC hub
    let hub_endpoint = format!("http://{}:{}", args.grpc_hub_host, args.grpc_hub_port);
    let mut hub_client = GrpcHubClient::connect(hub_endpoint).await?;

    let mut success_count = 0;
    let mut error_count = 0;
    let mut total_time = Duration::new(0, 0);
    let mut min_time = Duration::from_secs(3600); // Start with a large value
    let mut max_time = Duration::new(0, 0);

    let start_time = Instant::now();

    for i in 1..=args.count {
        let call_start = Instant::now();

        // Create gRPC service call request
        let request = tonic::Request::new(ServiceCallRequest {
            target_service: args.service.clone(),
            method: args.method.clone(),
            request_data: serde_json::to_string(&input_data)?,
            caller_service: "ping-client".to_string(),
            headers: std::collections::HashMap::new(),
        });

        match hub_client.call_service(request).await {
            Ok(response) => {
                let call_duration = call_start.elapsed();
                total_time += call_duration;
                min_time = min_time.min(call_duration);
                max_time = max_time.max(call_duration);

                let response = response.into_inner();
                if response.success {
                    success_count += 1;
                    println!(
                        "✅ Call {}: SUCCESS ({}ms)",
                        i,
                        call_duration.as_millis()
                    );
                } else {
                    error_count += 1;
                    let error_msg = if response.error_message.is_empty() {
                        "Unknown error".to_string()
                    } else {
                        response.error_message
                    };
                    println!(
                        "❌ Call {}: FAILED - {} ({}ms)",
                        i,
                        error_msg,
                        call_duration.as_millis()
                    );
                }
            }
            Err(e) => {
                let call_duration = call_start.elapsed();
                error_count += 1;
                println!("❌ Call {}: FAILED - gRPC error: {} ({}ms)", i, e, call_duration.as_millis());
            }
        }

        // Wait for the specified interval before the next call (except for the last call)
        if i < args.count {
            sleep(Duration::from_millis(args.interval_ms)).await;
        }
    }

    let total_duration = start_time.elapsed();
    let avg_time = if args.count > 0 {
        total_time / args.count
    } else {
        Duration::new(0, 0)
    };

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📊 Ping Test Results");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Total Calls: {}", args.count);
    println!("Successful: {} ✅", success_count);
    println!("Failed: {} ❌", error_count);
    println!("Success Rate: {:.2}%", (success_count as f64 / args.count as f64) * 100.0);
    println!("Total Duration: {:.2}s", total_duration.as_secs_f64());
    println!("Average Response Time: {:.2}ms", avg_time.as_secs_f64() * 1000.0);
    println!("Min Response Time: {:.2}ms", min_time.as_secs_f64() * 1000.0);
    println!("Max Response Time: {:.2}ms", max_time.as_secs_f64() * 1000.0);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    Ok(())
}

