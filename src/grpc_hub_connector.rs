use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use std::sync::atomic::{AtomicU64, Ordering};
use anyhow::Result;

mod grpc_hub {
    tonic::include_proto!("grpc_hub");
}

use grpc_hub::grpc_hub_client::GrpcHubClient;
use grpc_hub::ListServicesRequest;

/// A reusable connector for discovering and connecting to services through the gRPC hub
#[derive(Debug, Clone)]
pub struct GrpcHubConnector {
    hub_endpoint: String,
    service_cache: Arc<RwLock<Option<(String, u16)>>>,
    cache_timestamp: Arc<AtomicU64>,
    cache_duration_seconds: u64,
}

impl GrpcHubConnector {
    /// Create a new connector with default settings
    pub fn new() -> Self {
        Self::with_hub_endpoint("http://127.0.0.1:50099".to_string())
    }

    /// Create a new connector with a custom hub endpoint
    pub fn with_hub_endpoint(hub_endpoint: String) -> Self {
        Self {
            hub_endpoint,
            service_cache: Arc::new(RwLock::new(None)),
            cache_timestamp: Arc::new(AtomicU64::new(0)),
            cache_duration_seconds: 30, // Default 30 seconds cache
        }
    }

    /// Set custom cache duration in seconds
    pub fn with_cache_duration(mut self, duration_seconds: u64) -> Self {
        self.cache_duration_seconds = duration_seconds;
        self
    }

    /// Get the hub endpoint
    pub fn get_hub_endpoint(&self) -> String {
        self.hub_endpoint.clone()
    }

    /// Get the address and port of a service, using cache if available
    pub async fn get_service_address(&self, service_name: &str) -> Result<(String, u16)> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        let last_update = self.cache_timestamp.load(Ordering::Relaxed);
        
        // Check if cache is still valid
        if now - last_update < self.cache_duration_seconds {
            if let Some(cached) = self.service_cache.read().await.as_ref() {
                println!("🔍 [DEBUG] GrpcHubConnector: Using cached service {}:{}", cached.0, cached.1);
                return Ok(cached.clone());
            }
        }
        
        println!("🔍 [DEBUG] GrpcHubConnector: Cache expired or empty, discovering service: {}", service_name);
        self.discover_service(service_name).await
    }

    /// Discover a service from the hub (bypasses cache)
    pub async fn discover_service(&self, service_name: &str) -> Result<(String, u16)> {
        println!("🔍 [DEBUG] GrpcHubConnector: Starting service discovery for: {}", service_name);
        
        // Connect to the hub's gRPC API
        println!("🔍 [DEBUG] GrpcHubConnector: Connecting to hub at {}", self.hub_endpoint);
        
        let mut hub_client = GrpcHubClient::connect(self.hub_endpoint.clone()).await?;
        println!("🔍 [DEBUG] GrpcHubConnector: Successfully connected to hub");
        
        // Get registered services from the hub
        let request = tonic::Request::new(ListServicesRequest {
            filter: None,
        });
        println!("🔍 [DEBUG] GrpcHubConnector: Requesting service list from hub");
        
        let response = hub_client.list_services(request).await?;
        println!("🔍 [DEBUG] GrpcHubConnector: Received service list from hub");
        
        let services = response.into_inner().services;
        println!("🔍 [DEBUG] GrpcHubConnector: Found {} services in hub", services.len());
        
        // Find all services with the matching name
        let matching_services: Vec<_> = services
            .iter()
            .filter(|s| s.service_name == service_name)
            .collect();
        
        if matching_services.is_empty() {
            return Err(anyhow::anyhow!("Service '{}' not found in hub", service_name));
        }
        
        println!("🔍 [DEBUG] GrpcHubConnector: Found {} services with name '{}'", matching_services.len(), service_name);
        
        // Prioritize services that are online and not busy
        // Note: We can't directly check status from the gRPC response, so we'll use the first available service
        // In a real implementation, the hub would need to include status in the ListServices response
        let target_service = matching_services[0];
        
        let address = target_service.service_address.clone();
        let port = target_service.service_port.parse::<u16>()
            .map_err(|e| anyhow::anyhow!("Invalid port '{}' for service '{}': {}", target_service.service_port, service_name, e))?;
        
        println!("🔍 [DEBUG] GrpcHubConnector: Selected service '{}' at {}:{} (load balancing: first available)", service_name, address, port);
        
        // Cache the result
        {
            let mut cache = self.service_cache.write().await;
            *cache = Some((address.clone(), port));
        }
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        self.cache_timestamp.store(now, Ordering::Relaxed);
        
        Ok((address, port))
    }

    /// Get all registered services from the hub
    pub async fn list_all_services(&self) -> Result<Vec<grpc_hub::ServiceInfo>> {
        println!("🔍 [DEBUG] GrpcHubConnector: Listing all services from hub");
        
        let mut hub_client = GrpcHubClient::connect(self.hub_endpoint.clone()).await?;
        
        let request = tonic::Request::new(ListServicesRequest {
            filter: None,
        });
        
        let response = hub_client.list_services(request).await?;
        let services = response.into_inner().services;
        
        println!("🔍 [DEBUG] GrpcHubConnector: Found {} services in hub", services.len());
        
        Ok(services)
    }

    /// Check if a service is online
    pub async fn is_service_online(&self, service_name: &str) -> Result<bool> {
        let services = self.list_all_services().await?;
        
        if let Some(service) = services.iter().find(|s| s.service_name == service_name) {
            Ok(service.status == "online")
        } else {
            Ok(false)
        }
    }

    /// Clear the service cache (force fresh discovery on next call)
    pub async fn clear_cache(&self) {
        println!("🔍 [DEBUG] GrpcHubConnector: Clearing service cache");
        let mut cache = self.service_cache.write().await;
        *cache = None;
        self.cache_timestamp.store(0, Ordering::Relaxed);
    }

    /// Get cache statistics
    pub async fn get_cache_info(&self) -> (bool, u64) {
        let has_cached = self.service_cache.read().await.is_some();
        let last_update = self.cache_timestamp.load(Ordering::Relaxed);
        (has_cached, last_update)
    }

    /// Set service status to busy (optimized for speed)
    pub async fn set_service_busy(&self, service_id: &str) -> Result<()> {
        let hub_endpoint = self.get_hub_endpoint();
        let hub_url = format!("{}/api/service-status", hub_endpoint.replace("http://", "http://").replace(":50099", ":8080"));
        
        let request_body = serde_json::json!({
            "service_id": service_id,
            "status": "busy"
        });
        
        // Single attempt with short timeout for speed
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(50)) // 50ms timeout
            .build()?;
            
        let response = client
            .post(&hub_url)
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Failed to set service busy: {}", response.status()));
        }
        
        Ok(())
    }

    /// Set service status to online (optimized for speed)
    pub async fn set_service_online(&self, service_id: &str) -> Result<()> {
        let hub_endpoint = self.get_hub_endpoint();
        let hub_url = format!("{}/api/service-status", hub_endpoint.replace("http://", "http://").replace(":50099", ":8080"));
        
        let request_body = serde_json::json!({
            "service_id": service_id,
            "status": "online"
        });
        
        // Single attempt with short timeout for speed
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(50)) // 50ms timeout
            .build()?;
            
        let response = client
            .post(&hub_url)
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Failed to set service online: {}", response.status()));
        }
        
        Ok(())
    }
}

impl Default for GrpcHubConnector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_connector_creation() {
        let connector = GrpcHubConnector::new();
        assert_eq!(connector.hub_endpoint, "http://127.0.0.1:50099");
        assert_eq!(connector.cache_duration_seconds, 30);
    }

    #[tokio::test]
    async fn test_connector_with_custom_endpoint() {
        let connector = GrpcHubConnector::with_hub_endpoint("http://localhost:9999".to_string());
        assert_eq!(connector.hub_endpoint, "http://localhost:9999");
    }

    #[tokio::test]
    async fn test_connector_with_custom_cache_duration() {
        let connector = GrpcHubConnector::new().with_cache_duration(60);
        assert_eq!(connector.cache_duration_seconds, 60);
    }

    #[tokio::test]
    async fn test_cache_operations() {
        let connector = GrpcHubConnector::new();
        
        // Initially no cache
        let (has_cached, _) = connector.get_cache_info().await;
        assert!(!has_cached);
        
        // Clear cache (should be no-op)
        connector.clear_cache().await;
        
        // Still no cache
        let (has_cached, _) = connector.get_cache_info().await;
        assert!(!has_cached);
    }
}
