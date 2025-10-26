import { useState, useEffect, useRef, useCallback } from 'react';

interface SSEEventHandlers {
    onStatusChange?: (data: any) => void;
    onServiceRegistered?: (data: any) => void;
    onConnection?: (data: any) => void;
}

export function useEventSource(
    url: string,
    handlers: SSEEventHandlers,
    reconnectDelay: number = 3000,
    keepAliveTimeout: number = 60000 // 60 seconds - backend sends keep-alive every 30s, so give it double that
) {
    const [isConnected, setIsConnected] = useState(false);
    const eventSourceRef = useRef<EventSource | null>(null);
    const reconnectTimeoutRef = useRef<number | null>(null);
    const keepAliveCheckIntervalRef = useRef<number | null>(null);
    const lastKeepAliveTimeRef = useRef<number>(Date.now());
    const isUnmountedRef = useRef(false);
    const connectionEstablishedRef = useRef(false);
    const handlersRef = useRef(handlers);

    // Update handlers ref when handlers change
    useEffect(() => {
        handlersRef.current = handlers;
    }, [handlers]);

    const cleanup = useCallback(() => {
        if (keepAliveCheckIntervalRef.current !== null) {
            window.clearInterval(keepAliveCheckIntervalRef.current);
            keepAliveCheckIntervalRef.current = null;
        }
        if (eventSourceRef.current) {
            eventSourceRef.current.close();
            eventSourceRef.current = null;
        }
        if (reconnectTimeoutRef.current !== null) {
            window.clearTimeout(reconnectTimeoutRef.current);
            reconnectTimeoutRef.current = null;
        }
    }, []);

    const scheduleReconnectRef = useRef<(() => void) | null>(null);

    const connect = useCallback(() => {
        console.log('Connecting to SSE...');

        // Set up scheduleReconnect function that can call connect recursively
        const scheduleReconnect = () => {
            if (!isUnmountedRef.current) {
                if (reconnectTimeoutRef.current !== null) {
                    window.clearTimeout(reconnectTimeoutRef.current);
                }
                reconnectTimeoutRef.current = window.setTimeout(() => {
                    if (!isUnmountedRef.current) {
                        console.log('Attempting to reconnect SSE...');
                        connect();
                    }
                }, reconnectDelay);
            }
        };
        scheduleReconnectRef.current = scheduleReconnect;

        // Close existing connection if any
        if (eventSourceRef.current) {
            eventSourceRef.current.close();
            eventSourceRef.current = null;
        }

        // Clear existing intervals
        if (keepAliveCheckIntervalRef.current !== null) {
            window.clearInterval(keepAliveCheckIntervalRef.current);
            keepAliveCheckIntervalRef.current = null;
        }

        // Reset state for new connection attempt
        // Always reset lastKeepAliveTime when starting a new connection
        lastKeepAliveTimeRef.current = Date.now();
        connectionEstablishedRef.current = false;

        // Create new EventSource connection
        const eventSource = new EventSource(url);
        eventSourceRef.current = eventSource;

        // Listen for all messages
        // Note: SSE comments (keep-alive) don't trigger onmessage, so we update timestamp on ANY message
        eventSource.onmessage = (event: MessageEvent) => {
            connectionEstablishedRef.current = true;
            // Update keep-alive timestamp on any message received
            lastKeepAliveTimeRef.current = Date.now();
            setIsConnected(true);

            // Try to parse as JSON event
            try {
                const data = JSON.parse(event.data);
                if (data.type === 'connected' && handlersRef.current.onConnection) {
                    handlersRef.current.onConnection(data);
                }
            } catch (e) {
                // Not JSON, might be other message type - that's OK, we still updated the timestamp
            }
        };

        // Handle status_change events
        eventSource.addEventListener('status_change', (event: MessageEvent) => {
            connectionEstablishedRef.current = true;
            lastKeepAliveTimeRef.current = Date.now();
            setIsConnected(true);
            const data = JSON.parse(event.data);
            console.log('Status change event:', data);
            if (handlersRef.current.onStatusChange) {
                handlersRef.current.onStatusChange(data);
            }
        });

        // Handle service_registered events
        eventSource.addEventListener('service_registered', (event: MessageEvent) => {
            connectionEstablishedRef.current = true;
            lastKeepAliveTimeRef.current = Date.now();
            setIsConnected(true);
            const data = JSON.parse(event.data);
            console.log('Service registered event:', data);
            if (handlersRef.current.onServiceRegistered) {
                handlersRef.current.onServiceRegistered(data);
            }
        });

        // Handle connection events
        eventSource.addEventListener('connection', (event: MessageEvent) => {
            connectionEstablishedRef.current = true;
            lastKeepAliveTimeRef.current = Date.now();
            setIsConnected(true);
            const data = JSON.parse(event.data);
            console.log('SSE connected:', data.message);
            if (handlersRef.current.onConnection) {
                handlersRef.current.onConnection(data);
            }
        });

        eventSource.onopen = () => {
            console.log('SSE connection opened - Hub is online');
            connectionEstablishedRef.current = true;
            setIsConnected(true);
            lastKeepAliveTimeRef.current = Date.now();
        };

        eventSource.onerror = () => {
            const readyState = eventSource.readyState;
            console.error('SSE error event, readyState:', readyState);

            // Only mark as disconnected if the connection is actually closed
            // CONNECTING state errors are often transient and shouldn't trigger disconnection
            if (readyState === EventSource.CLOSED) {
                console.error('SSE connection closed - Hub backend may be offline');
                setIsConnected(false);

                // Close the connection
                eventSource.close();
                eventSourceRef.current = null;

                // Clear keep-alive check
                if (keepAliveCheckIntervalRef.current !== null) {
                    window.clearInterval(keepAliveCheckIntervalRef.current);
                    keepAliveCheckIntervalRef.current = null;
                }

                // Schedule reconnection
                if (scheduleReconnectRef.current) {
                    scheduleReconnectRef.current();
                }
            } else {
                // CONNECTING state - don't mark as disconnected yet, let it try to connect
                console.log('SSE error in CONNECTING state, will retry...');
            }
        };

        // Monitor keep-alive messages more aggressively
        keepAliveCheckIntervalRef.current = window.setInterval(() => {
            const timeSinceLastKeepAlive = Date.now() - lastKeepAliveTimeRef.current;

            if (!eventSourceRef.current) {
                console.log('Keep-alive check: No EventSource exists');
                setIsConnected(false);
                return;
            }

            const readyState = eventSourceRef.current.readyState;

            // Check readyState
            if (readyState === EventSource.CLOSED) {
                console.log('Keep-alive check: SSE is closed');
                setIsConnected(false);
                eventSourceRef.current = null;

                if (keepAliveCheckIntervalRef.current !== null) {
                    window.clearInterval(keepAliveCheckIntervalRef.current);
                    keepAliveCheckIntervalRef.current = null;
                }

                if (scheduleReconnectRef.current) {
                    scheduleReconnectRef.current();
                }
                return;
            }

            if (readyState === EventSource.CONNECTING) {
                // Only log occasionally to avoid spam
                if (Math.random() < 0.1) {
                    console.log('Keep-alive check: SSE is connecting...');
                }
                // Don't timeout CONNECTING state - let the EventSource handle it
                // The onerror handler will manage failures
                return;
            }

            if (readyState === EventSource.OPEN) {
                if (connectionEstablishedRef.current) {
                    // Connection is established - check timeout
                    if (timeSinceLastKeepAlive > keepAliveTimeout) {
                        console.warn(`No events received for ${keepAliveTimeout}ms, but connection is OPEN. Reconnecting...`);
                        setIsConnected(false);
                        
                        // Close and reconnect if no activity for too long
                        eventSourceRef.current.close();
                        eventSourceRef.current = null;
                        
                        if (keepAliveCheckIntervalRef.current !== null) {
                            window.clearInterval(keepAliveCheckIntervalRef.current);
                            keepAliveCheckIntervalRef.current = null;
                        }
                        
                        if (scheduleReconnectRef.current) {
                            scheduleReconnectRef.current();
                        }
                    } else {
                        // Connection is healthy - ensure we're marked as connected
                        setIsConnected(true);
                    }
                } else {
                    // Connection is OPEN but we haven't received any messages yet
                    // Give it some time (check if we've been waiting too long)
                    if (timeSinceLastKeepAlive > 10000) {
                        // If we've been waiting more than 10 seconds for first message, something might be wrong
                        console.warn('Connection OPEN but no messages received yet, attempting reconnect...');
                        setIsConnected(false);
                        eventSourceRef.current.close();
                        eventSourceRef.current = null;
                        
                        if (keepAliveCheckIntervalRef.current !== null) {
                            window.clearInterval(keepAliveCheckIntervalRef.current);
                            keepAliveCheckIntervalRef.current = null;
                        }
                        
                        if (scheduleReconnectRef.current) {
                            scheduleReconnectRef.current();
                        }
                    } else {
                        // Still waiting for first message - this is normal on initial connection
                        // But connection is OPEN, so we should show as connected
                        setIsConnected(true);
                    }
                }
            }
        }, 2000); // Check every 2 seconds for faster detection
    }, [url, reconnectDelay, keepAliveTimeout]);

    useEffect(() => {
        isUnmountedRef.current = false;
        connect();

        return () => {
            isUnmountedRef.current = true;
            cleanup();
        };
    }, [connect, cleanup]);

    return isConnected;
}