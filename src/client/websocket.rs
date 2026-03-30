//! WebSocket client for real-time event streaming.

use bytes::Bytes;
use futures::{SinkExt, StreamExt};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{debug, error, info, warn};

use super::{ClientError, Result};
use crate::events::Event;

/// Ping interval for keepalive (5 seconds - server may have short timeout)
const PING_INTERVAL: Duration = Duration::from_secs(5);

/// Event stream handle for receiving events from the server
pub struct EventStream {
    receiver: mpsc::UnboundedReceiver<Event>,
    connected: Arc<AtomicBool>,
    _reader_handle: tokio::task::JoinHandle<()>,
    _ping_handle: tokio::task::JoinHandle<()>,
}

impl EventStream {
    /// Connect to the WebSocket and start receiving events
    pub async fn connect(url: &str) -> Result<Self> {
        info!("Connecting to WebSocket: {}", url);

        let (ws_stream, _) = connect_async(url)
            .await
            .map_err(|e| ClientError::WebSocket(e.to_string()))?;

        info!("Connected to WebSocket");

        let (write, read) = ws_stream.split();
        let (tx, rx) = mpsc::unbounded_channel();
        let connected = Arc::new(AtomicBool::new(true));
        
        // Wrap write in Arc<Mutex> so both tasks can use it
        let write = Arc::new(tokio::sync::Mutex::new(write));
        let write_for_reader = write.clone();
        let write_for_ping = write.clone();
        
        let connected_for_reader = connected.clone();
        let connected_for_ping = connected.clone();

        // Spawn task to read events and respond to pings
        let reader_handle = tokio::spawn(async move {
            let mut read = read;
            while let Some(msg_result) = read.next().await {
                match msg_result {
                    Ok(Message::Text(text)) => {
                        debug!("Received WebSocket message: {}", &text[..text.len().min(200)]);
                        let mut event = match serde_json::from_str::<Event>(&text) {
                            Ok(event) => event,
                            Err(e) => {
                                warn!("Failed to parse event: {} - {}", e, &text[..text.len().min(100)]);
                                continue;
                            }
                        };
                        // Unknown events may contain error info (code/detail fields)
                        if matches!(event, Event::Unknown) {
                            if let Ok(raw) = serde_json::from_str::<serde_json::Value>(&text) {
                                if let Some(code) = raw.get("code").and_then(|v| v.as_str()) {
                                    event = Event::AgentErrorEvent(crate::events::AgentErrorEvent {
                                        base: crate::events::EventBase {
                                            id: raw.get("id").and_then(|v| v.as_str()).map(|s| s.to_string()),
                                            timestamp: raw.get("timestamp").and_then(|v| v.as_str()).map(|s| s.to_string()),
                                            source: raw.get("source").and_then(|v| v.as_str()).map(|s| s.to_string()),
                                        },
                                        error: code.to_string(),
                                    });
                                }
                            }
                        }
                        if tx.send(event).is_err() {
                            info!("Event receiver dropped, stopping WebSocket");
                            break;
                        }
                    }
                    Ok(Message::Ping(data)) => {
                        debug!("Received ping, sending pong");
                        let mut write = write_for_reader.lock().await;
                        if let Err(e) = write.send(Message::Pong(data)).await {
                            error!("Failed to send pong: {}", e);
                            break;
                        }
                    }
                    Ok(Message::Close(frame)) => {
                        info!("WebSocket closed by server: {:?}", frame);
                        break;
                    }
                    Ok(Message::Pong(_)) => {
                        debug!("Received pong (keepalive OK)");
                    }
                    Ok(_) => {
                        // Ignore other message types (Binary, Frame)
                    }
                    Err(e) => {
                        error!("WebSocket error: {}", e);
                        break;
                    }
                }
            }
            connected_for_reader.store(false, Ordering::SeqCst);
            info!("WebSocket reader task ended");
        });

        // Spawn task to send periodic pings for keepalive
        let ping_handle = tokio::spawn(async move {
            // Use interval_at to skip the immediate first tick
            let start = tokio::time::Instant::now() + PING_INTERVAL;
            let mut interval = tokio::time::interval_at(start, PING_INTERVAL);
            
            loop {
                interval.tick().await;
                
                if !connected_for_ping.load(Ordering::SeqCst) {
                    debug!("Connection closed, stopping ping task");
                    break;
                }
                
                let mut write = write_for_ping.lock().await;
                debug!("Sending keepalive ping");
                if let Err(e) = write.send(Message::Ping(Bytes::new())).await {
                    warn!("Failed to send keepalive ping: {}", e);
                    connected_for_ping.store(false, Ordering::SeqCst);
                    break;
                }
            }
        });

        Ok(Self {
            receiver: rx,
            connected,
            _reader_handle: reader_handle,
            _ping_handle: ping_handle,
        })
    }

    /// Try to receive the next event without blocking
    pub fn try_recv(&mut self) -> Option<Event> {
        self.receiver.try_recv().ok()
    }

    /// Receive the next event, blocking if necessary
    pub async fn recv(&mut self) -> Option<Event> {
        self.receiver.recv().await
    }
    
    /// Check if the WebSocket is still connected
    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::SeqCst) && !self.receiver.is_closed()
    }
}

/// Managed WebSocket connection that can reconnect
pub struct ManagedEventStream {
    url: String,
    stream: Option<EventStream>,
}

impl ManagedEventStream {
    pub fn new(url: String) -> Self {
        Self { url, stream: None }
    }

    /// Ensure connection is established
    pub async fn ensure_connected(&mut self) -> Result<()> {
        if self.stream.is_none() {
            self.stream = Some(EventStream::connect(&self.url).await?);
        }
        Ok(())
    }

    /// Try to receive the next event
    pub fn try_recv(&mut self) -> Option<Event> {
        self.stream.as_mut().and_then(|s| s.try_recv())
    }

    /// Disconnect the stream
    pub fn disconnect(&mut self) {
        self.stream = None;
    }
}
