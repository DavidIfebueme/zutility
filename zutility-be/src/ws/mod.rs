use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
    time::{Duration, Instant},
};

use axum::extract::ws::{Message, WebSocket};
use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use tokio::{
    select,
    sync::{RwLock, mpsc},
    time::interval,
};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum WsOrderEvent {
    PaymentDetected {
        confirmations: u16,
        required: u16,
    },
    Confirmation {
        confirmations: u16,
        required: u16,
    },
    PaymentConfirmed {
        confirmations: u16,
    },
    Dispatching,
    Completed {
        token: Option<String>,
        reference: String,
    },
    Expired,
    Failed {
        reason: String,
    },
}

#[derive(Clone)]
pub struct WsHub {
    inner: Arc<RwLock<HashMap<Uuid, VecDeque<ConnectionEntry>>>>,
}

#[derive(Clone)]
struct ConnectionEntry {
    id: Uuid,
    sender: mpsc::UnboundedSender<Message>,
    last_seen: Instant,
}

pub struct WsSubscription {
    pub connection_id: Uuid,
    pub sender: mpsc::UnboundedSender<Message>,
    pub receiver: mpsc::UnboundedReceiver<Message>,
    pub evicted_senders: Vec<mpsc::UnboundedSender<Message>>,
}

impl WsHub {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn subscribe(&self, order_id: Uuid) -> WsSubscription {
        let (sender, receiver) = mpsc::unbounded_channel();
        let connection_id = Uuid::new_v4();
        let mut evicted_senders = Vec::new();

        let mut guard = self.inner.write().await;
        let entries = guard.entry(order_id).or_insert_with(VecDeque::new);

        if entries.len() >= 3 {
            if let Some(evicted) = entries.pop_front() {
                evicted_senders.push(evicted.sender);
            }
        }

        entries.push_back(ConnectionEntry {
            id: connection_id,
            sender: sender.clone(),
            last_seen: Instant::now(),
        });

        WsSubscription {
            connection_id,
            sender,
            receiver,
            evicted_senders,
        }
    }

    pub async fn unsubscribe(&self, order_id: Uuid, connection_id: Uuid) {
        let mut guard = self.inner.write().await;
        if let Some(entries) = guard.get_mut(&order_id) {
            entries.retain(|entry| entry.id != connection_id);
            if entries.is_empty() {
                guard.remove(&order_id);
            }
        }
    }

    pub async fn touch(&self, order_id: Uuid, connection_id: Uuid) {
        let mut guard = self.inner.write().await;
        if let Some(entries) = guard.get_mut(&order_id) {
            for entry in entries.iter_mut() {
                if entry.id == connection_id {
                    entry.last_seen = Instant::now();
                    return;
                }
            }
        }
    }

    pub async fn elapsed_since_touch(
        &self,
        order_id: Uuid,
        connection_id: Uuid,
    ) -> Option<Duration> {
        let guard = self.inner.read().await;
        guard
            .get(&order_id)
            .and_then(|entries| entries.iter().find(|entry| entry.id == connection_id))
            .map(|entry| entry.last_seen.elapsed())
    }

    pub async fn active_connections(&self, order_id: Uuid) -> usize {
        let guard = self.inner.read().await;
        guard.get(&order_id).map_or(0, VecDeque::len)
    }

    pub async fn broadcast_event(&self, order_id: Uuid, event: &WsOrderEvent) -> usize {
        let payload = match serde_json::to_string(event) {
            Ok(payload) => payload,
            Err(_) => return 0,
        };

        let mut sent = 0;
        let mut stale_ids = Vec::new();

        {
            let guard = self.inner.read().await;
            if let Some(entries) = guard.get(&order_id) {
                for entry in entries {
                    if entry
                        .sender
                        .send(Message::Text(payload.clone().into()))
                        .is_ok()
                    {
                        sent += 1;
                    } else {
                        stale_ids.push(entry.id);
                    }
                }
            }
        }

        if !stale_ids.is_empty() {
            let mut guard = self.inner.write().await;
            if let Some(entries) = guard.get_mut(&order_id) {
                entries.retain(|entry| !stale_ids.contains(&entry.id));
                if entries.is_empty() {
                    guard.remove(&order_id);
                }
            }
        }

        sent
    }
}

pub async fn serve_connection(
    hub: WsHub,
    order_id: Uuid,
    socket: WebSocket,
    initial_event: Option<WsOrderEvent>,
) {
    let subscription = hub.subscribe(order_id).await;
    let connection_id = subscription.connection_id;
    let sender = subscription.sender.clone();
    let mut receiver = subscription.receiver;

    for evicted_sender in &subscription.evicted_senders {
        let _ = evicted_sender.send(Message::Close(None));
    }

    if let Some(event) = initial_event {
        if let Ok(payload) = serde_json::to_string(&event) {
            let _ = sender.send(Message::Text(payload.into()));
        }
    }

    let (mut ws_tx, mut ws_rx) = socket.split();
    let mut ticker = interval(Duration::from_secs(20));

    loop {
        select! {
            outbound = receiver.recv() => {
                match outbound {
                    Some(message) => {
                        if ws_tx.send(message).await.is_err() {
                            break;
                        }
                    }
                    None => break,
                }
            }
            inbound = ws_rx.next() => {
                match inbound {
                    Some(Ok(Message::Ping(payload))) => {
                        hub.touch(order_id, connection_id).await;
                        if ws_tx.send(Message::Pong(payload)).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(Message::Pong(_))) | Some(Ok(Message::Text(_))) | Some(Ok(Message::Binary(_))) => {
                        hub.touch(order_id, connection_id).await;
                    }
                    Some(Ok(Message::Close(_))) => break,
                    Some(Err(_)) | None => break,
                }
            }
            _ = ticker.tick() => {
                let elapsed = hub.elapsed_since_touch(order_id, connection_id).await;
                if elapsed.is_none_or(|value| value > Duration::from_secs(60)) {
                    let _ = ws_tx.send(Message::Close(None)).await;
                    break;
                }
                if ws_tx.send(Message::Ping(Vec::new().into())).await.is_err() {
                    break;
                }
            }
        }
    }

    hub.unsubscribe(order_id, connection_id).await;
}

impl Default for WsHub {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::sleep;

    #[tokio::test]
    async fn subscribe_enforces_three_connections_per_order() {
        let hub = WsHub::new();
        let order_id = Uuid::new_v4();

        let _ = hub.subscribe(order_id).await;
        let _ = hub.subscribe(order_id).await;
        let _ = hub.subscribe(order_id).await;
        let fourth = hub.subscribe(order_id).await;

        assert_eq!(fourth.evicted_senders.len(), 1);
        assert_eq!(hub.active_connections(order_id).await, 3);
    }

    #[tokio::test]
    async fn broadcast_event_delivers_to_all_subscribers() {
        let hub = WsHub::new();
        let order_id = Uuid::new_v4();
        let mut first = hub.subscribe(order_id).await.receiver;
        let mut second = hub.subscribe(order_id).await.receiver;

        let event = WsOrderEvent::Dispatching;
        let sent = hub.broadcast_event(order_id, &event).await;
        assert_eq!(sent, 2);

        let left = first.recv().await;
        let right = second.recv().await;

        assert!(left.is_some());
        assert!(right.is_some());
    }

    #[tokio::test]
    async fn stale_connection_is_removed_after_disconnect() {
        let hub = WsHub::new();
        let order_id = Uuid::new_v4();
        let sub = hub.subscribe(order_id).await;

        drop(sub.receiver);
        sleep(Duration::from_millis(1)).await;

        let _ = hub.broadcast_event(order_id, &WsOrderEvent::Expired).await;

        assert_eq!(hub.active_connections(order_id).await, 0);
    }
}
