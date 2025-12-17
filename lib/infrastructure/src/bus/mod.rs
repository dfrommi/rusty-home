use tokio::sync::broadcast::error::RecvError;

pub struct EventBus<T> {
    tx: tokio::sync::broadcast::Sender<T>,
}

pub struct EventListener<T> {
    rx: tokio::sync::broadcast::Receiver<T>,
}

#[derive(Clone)]
pub struct EventEmitter<T> {
    tx: tokio::sync::broadcast::Sender<T>,
}

impl<T: Clone + std::fmt::Debug> EventBus<T> {
    pub fn new(buffer_size: usize) -> Self {
        let (tx, _) = tokio::sync::broadcast::channel(buffer_size);
        Self { tx }
    }

    pub fn subscribe(&self) -> EventListener<T> {
        EventListener::new(self.tx.subscribe())
    }

    pub fn emitter(&self) -> EventEmitter<T> {
        EventEmitter::new(self.tx.clone())
    }
}

impl<T: Clone> EventListener<T> {
    pub fn new(rx: tokio::sync::broadcast::Receiver<T>) -> Self {
        Self { rx }
    }

    pub async fn recv(&mut self) -> Option<T> {
        match self.rx.recv().await {
            Ok(event) => Some(event),
            Err(RecvError::Closed) => {
                tracing::error!("Channel for event receiver of {} is closed", std::any::type_name::<T>());
                None
            }
            Err(RecvError::Lagged(count)) => {
                tracing::warn!(
                    "Channel for event receiver of {} lagged by {} messages",
                    std::any::type_name::<T>(),
                    count
                );
                None
            }
        }
    }
}

impl<T: Clone + std::fmt::Debug> EventEmitter<T> {
    fn new(tx: tokio::sync::broadcast::Sender<T>) -> Self {
        Self { tx }
    }

    pub fn send(&self, event: T) {
        if let Err(e) = self.tx.send(event.clone()) {
            tracing::error!("Error sending event {:?}: {}", event, e);
        }
    }
}
