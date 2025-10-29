import { useState, useEffect } from 'react';
import { useEventSource } from './hooks/useEventSource';
import './App.css';

// Service status updates are now handled by SSE events only

interface Service {
  service_id: string;
  service_name: string;
  service_version: string;
  service_address: string;
  service_port: string;
  methods: string[];
  metadata: Record<string, string>;
  registered_at: string;
  last_heartbeat: string;
  status: string; // "online", "offline", or "busy"
}

interface MethodSchema {
  name: string;
  description: string;
  request_schema: any;
}

interface ServiceSchema {
  service_name: string;
  service_version: string;
  service_address: string;
  service_port: string;
  methods: MethodSchema[];
  metadata: Record<string, string>;
}

interface ServiceCall {
  method: string;
  request: any;
  response?: any;
  error?: string;
  timestamp: string;
}

interface ServicesResponse {
  services: Service[];
}

interface SchemaResponse {
  schemas: ServiceSchema[];
}

function App() {
  const [services, setServices] = useState<Service[]>([]);
  const [schemas, setSchemas] = useState<ServiceSchema[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [lastUpdate, setLastUpdate] = useState<Date>(new Date());
  const [selectedService, setSelectedService] = useState<string | null>(null);

  // Note: Service status updates now come only from SSE events, not local state changes

  // Detect system theme preference
  useEffect(() => {
    const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
    const updateTheme = (e: MediaQueryListEvent | MediaQueryList) => {
      document.documentElement.setAttribute('data-theme', e.matches ? 'dark' : 'light');
    };
    
    // Set initial theme
    updateTheme(mediaQuery);
    
    // Listen for changes
    mediaQuery.addEventListener('change', updateTheme);
    
    return () => mediaQuery.removeEventListener('change', updateTheme);
  }, []);

  const fetchServices = async () => {
    try {
      const response = await fetch('/api/services');
      if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`);
      }
      const data: ServicesResponse = await response.json();
      setServices(data.services);
      setError(null);
      setLastUpdate(new Date());
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to fetch services');
    } finally {
      setLoading(false);
    }
  };

  const fetchSchemas = async () => {
    try {
      const response = await fetch('/api/service-schema');
      if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`);
      }
      const data: SchemaResponse = await response.json();
      setSchemas(data.schemas);
    } catch (err) {
      console.error('Failed to fetch schemas:', err);
    }
  };

  // Use the EventSource hook
  const isHubConnected = useEventSource('/api/events', {
    onStatusChange: (data) => {
      console.log('Status change received:', data);
      setServices(prevServices => {
        const service = prevServices.find(s => s.service_id === data.service_id);
        // Only update if status actually changed to prevent unnecessary re-renders
        if (service && service.status !== data.status) {
          console.log(`Service ${data.service_id} status changed: ${service.status} -> ${data.status}`);
          return prevServices.map(service => 
            service.service_id === data.service_id 
              ? { ...service, status: data.status }
              : service
          );
        }
        return prevServices;
      });
    },
    onServiceRegistered: () => {
      fetchServices();
      fetchSchemas();
    },
    onConnection: (data) => {
      console.log('SSE connected:', data.message);
    }
  });

  useEffect(() => {
    // Initial load
    fetchServices();
    fetchSchemas();
  }, []);

  const handleRefresh = () => {
    setLoading(true);
    fetchServices();
  };

  return (
    <div className="app">
      <header className="header">
        <div className="header-content">
          <h1>🚀 gRPC Hub</h1>
          <p>Service Discovery & Registry</p>
        </div>
      </header>

      <div className="controls">
        <div className="status">
          <span className={isHubConnected ? 'hub-connected' : 'hub-disconnected'}>
            {isHubConnected ? '🟢' : '🔴'} Hub: {isHubConnected ? 'Connected' : 'Disconnected'}
          </span>
          {' | '}
          Status: {loading ? 'Loading...' : error ? 'Error' : `${services.length} service(s) registered`}
      </div>
        <button className="refresh-btn" onClick={handleRefresh} disabled={loading}>
          🔄 Refresh
        </button>
      </div>

      <div className="content">
        {error && (
          <div className="error">
            Error loading services: {error}
          </div>
        )}
        
        {loading && !error && (
          <div className="loading">
            Loading services...
          </div>
        )}
        
        {!loading && !error && services.length === 0 && (
          <div className="no-services">
            No services registered yet.
          </div>
        )}
        
        {!loading && !error && services.length > 0 && (
          <div className="services-layout">
            <div className="services-list">
              {services.map((service) => {
                return (
                  <div 
                    key={service.service_id}
                    className={`service-item ${selectedService === service.service_id ? 'selected' : ''} ${service.status === 'busy' ? 'busy' : ''}`}
                    onClick={() => setSelectedService(service.service_id)}
                  >
                    <div className="service-item-header">
                      <span className="service-item-name">{service.service_name}</span>
                      <span className="service-item-version">v{service.service_version}</span>
                      <span className={`service-status ${service.status}`}>
                        {service.status === 'online' && '🟢'}
                        {service.status === 'offline' && '🔴'}
                        {service.status === 'busy' && '🟠'}
                      </span>
                    </div>
                    <div className="service-item-address">{service.service_address}:{service.service_port}</div>
                    <div className="service-item-methods">{service.methods.length} method{service.methods.length !== 1 ? 's' : ''}</div>
                  </div>
                );
              })}
            </div>
            
            <div className="service-details-panel">
              {selectedService ? (
                (() => {
                  const service = services.find(s => s.service_id === selectedService);
                  if (!service) return null;
                  const schema = schemas.find(s => s.service_name === service.service_name);
                  return (
                    <ServiceCard 
                      key={service.service_id} 
                      service={service} 
                      schema={schema} 
                      onUnregister={() => {
                        fetchServices();
                        setSelectedService(null);
                      }}
                    />
                  );
                })()
              ) : (
                <div className="no-service-selected">
                  <p>👈 Select a service from the list to view details</p>
                </div>
              )}
            </div>
          </div>
        )}
      </div>

      <footer className="footer">
        <p>Last updated: {lastUpdate.toLocaleTimeString()}</p>
      </footer>
    </div>
  );
}

function ServiceCard({ 
  service, 
  schema: _schema, 
  onUnregister
}: { 
  service: Service; 
  schema?: ServiceSchema; 
  onUnregister: () => void;
}) {
  const [activeTab, setActiveTab] = useState<string>('methods');
  const [testResults, setTestResults] = useState<ServiceCall[]>([]);
  const [selectedMethod, setSelectedMethod] = useState<string>('');
  const [requestData, setRequestData] = useState<string>('{}');
  const [showForm, setShowForm] = useState(false);
  const [isUnregistering, setIsUnregistering] = useState(false);
  const [autoOpenResults, setAutoOpenResults] = useState(true);

  const testServiceMethod = async () => {
    if (!selectedMethod) return;
    
    // Don't set global isTesting state - allow multiple concurrent calls
    const timestamp = new Date().toISOString();
    
    // Note: Service status changes are now handled by SSE events from the backend
    
    try {
      let request: any;
      try {
        request = JSON.parse(requestData);
      } catch (e) {
        throw new Error('Invalid JSON in request data');
      }

      // Extract just the method name without parentheses (e.g., "GetDividendHistory(GetDividendHistoryRequest)" -> "GetDividendHistory")
      const cleanMethodName = selectedMethod.split('(')[0];

      // Map service names to full gRPC service names
      const getGrpcServiceName = (serviceName: string) => {
        switch (serviceName) {
          case 'dividend-service':
            return 'dividend_service.DividendService';
          case 'web-content-extract':
            return 'web_content_extract.WebContentExtract';
          default:
            return serviceName; // fallback to original name
        }
      };

      // Note: Service status changes are handled by SSE events from the backend

      const response = await fetch(`/api/grpc-call`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          service: getGrpcServiceName(service.service_name),
          method: cleanMethodName,
          input: request,
          host: service.service_address,
          port: service.service_port
        })
      });

      const result = await response.json();
      
      const serviceCall: ServiceCall = {
        method: selectedMethod,
        request,
        response: result.success ? result.data : null,
        error: result.success ? null : result.error,
        timestamp
      };
      
      setTestResults(prev => [serviceCall, ...prev]);
      if (autoOpenResults) {
        setActiveTab('results');
      }
    } catch (error) {
      const serviceCall: ServiceCall = {
        method: selectedMethod,
        request: {},
        error: error instanceof Error ? error.message : 'Unknown error',
        timestamp
      };
      setTestResults(prev => [serviceCall, ...prev]);
      if (autoOpenResults) {
        setActiveTab('results');
      }
    }
  };

  const handleUnregister = async () => {
    if (!confirm(`Are you sure you want to unregister ${service.service_name}?`)) {
      return;
    }

    setIsUnregistering(true);
    try {
      const response = await fetch(`/api/services/${service.service_id}`, {
        method: 'DELETE',
      });

      const result = await response.json();
      
      if (result.success) {
        console.log('Service unregistered:', service.service_name);
        onUnregister();
      } else {
        alert('Failed to unregister service: ' + result.message);
      }
    } catch (error) {
      alert('Error unregistering service: ' + (error instanceof Error ? error.message : 'Unknown error'));
    } finally {
      setIsUnregistering(false);
    }
  };

  return (
    <div className={`service-card ${service.status === 'busy' ? 'busy' : ''}`}>
      <div className="service-header">
        <div className="service-name">{service.service_name}</div>
        <div className="service-version">v{service.service_version}</div>
      </div>
      
      <div className="service-address-bar">
        <span className="service-address">{service.service_address}:{service.service_port}</span>
        <div className="service-status">
          {service.status === 'online' && <span className="status-online">🟢 Online</span>}
          {service.status === 'offline' && <span className="status-offline">🔴 Offline</span>}
          {service.status === 'busy' && <span className="status-busy">🟠 Busy</span>}
        </div>
        <button 
          className="unregister-button" 
          onClick={(e) => {
            e.stopPropagation();
            handleUnregister();
          }}
          disabled={isUnregistering}
        >
          {isUnregistering ? 'Unregistering...' : '❌ Unregister'}
        </button>
      </div>

      <div className="service-content">
        <div className="service-details">
          <div className="service-tabs">
            <button 
              className={`service-tab ${activeTab === 'methods' ? 'active' : ''}`}
              onClick={(e) => {
                e.stopPropagation();
                setActiveTab('methods');
              }}
            >
              🔧 Methods
            </button>
            <button 
              className={`service-tab ${activeTab === 'results' ? 'active' : ''}`}
              onClick={(e) => {
                e.stopPropagation();
                setActiveTab('results');
              }}
            >
              📊 Results ({testResults.length})
            </button>
            <button 
              className={`service-tab ${activeTab === 'info' ? 'active' : ''}`}
              onClick={(e) => {
                e.stopPropagation();
                setActiveTab('info');
              }}
            >
              ℹ️ Info
            </button>
          </div>

          <div className={`tab-content ${activeTab === 'methods' ? 'active' : ''}`}>
            {service.status === 'offline' && (
              <div className="offline-warning">
                ⚠️ This service is currently offline. Methods cannot be tested while the service is unavailable.
              </div>
            )}
            <div className="methods-section">
              <h4>📋 Available Methods:</h4>
              <div className="methods-list">
                {service.methods.map((method, index) => (
                  <button
                    key={index}
                    className="method-button"
                    onClick={() => {
                      setSelectedMethod(method);
                      setShowForm(true);
                      setRequestData('{}');
                    }}
                  >
                    {method}
                  </button>
                ))}
              </div>
            </div>

          {showForm && selectedMethod && (
            <div className="test-form">
              <h4>Test Method: {selectedMethod}</h4>
              <div className="form-group">
                <label htmlFor="request-data">Request Data (JSON):</label>
                <textarea
                  id="request-data"
                  value={requestData}
                  onChange={(e) => setRequestData(e.target.value)}
                  className="request-textarea"
                  rows={8}
                  placeholder="Enter JSON request data..."
                />
              </div>
              <div className="form-actions">
                <div className="form-controls">
                  <label className="toggle-switch">
                    <input
                      type="checkbox"
                      checked={autoOpenResults}
                      onChange={(e) => setAutoOpenResults(e.target.checked)}
                    />
                    <span className="toggle-slider"></span>
                    <span className="toggle-label">Auto-open Results</span>
                  </label>
                </div>
                <div className="form-buttons">
                  <button
                    className="test-button"
                    onClick={testServiceMethod}
                    disabled={service.status === 'offline'}
                    title={service.status === 'offline' ? 'Service is offline and cannot be called' : service.status === 'busy' ? 'Service is busy but you can still make calls' : ''}
                  >
                    {service.status === 'offline' ? 'Service Offline' : 'Test Method'}
                  </button>
                  <button
                    className="cancel-button"
                    onClick={() => setShowForm(false)}
                  >
                    Cancel
                  </button>
                </div>
              </div>
            </div>
          )}
          </div>

          <div className={`tab-content ${activeTab === 'results' ? 'active' : ''}`}>
            {testResults.length > 0 ? (
              <div className="test-results">
                <h4>📊 Test Results ({testResults.length}):</h4>
                {testResults.slice(0, 10).map((result, index) => (
                  <div key={index} className="test-result">
                    <div className="test-header">
                      <div className="test-method">{result.method}</div>
                      <div className="test-timestamp">{new Date(result.timestamp).toLocaleTimeString()}</div>
                    </div>
                    {result.error ? (
                      <div className="test-error">❌ {result.error}</div>
                    ) : (
                      <div className="test-success">
                        <div className="success-indicator">✅ Success</div>
                        <pre className="response-data">
                          {JSON.stringify(result.response, null, 2)}
                        </pre>
                      </div>
                    )}
                  </div>
                ))}
              </div>
            ) : (
              <div style={{ textAlign: 'center', padding: '2rem', color: '#6c757d' }}>
                <p>No test results yet. Run a test method to see results here.</p>
              </div>
            )}
          </div>

          <div className={`tab-content ${activeTab === 'info' ? 'active' : ''}`}>
            <div className="metadata-section">
              <h4>📝 Metadata:</h4>
              <pre className="metadata-content">
                {JSON.stringify(service.metadata, null, 2)}
              </pre>
            </div>

            <div className="timestamps">
              <div className="timestamp">
                <strong>Registered:</strong>
                <span>{new Date(service.registered_at).toLocaleString()}</span>
              </div>
              <div className="timestamp">
                <strong>Last Heartbeat:</strong>
                <span>{new Date(service.last_heartbeat).toLocaleString()}</span>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

export default App;