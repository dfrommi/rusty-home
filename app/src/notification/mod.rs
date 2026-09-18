mod adapter;

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use derive_more::derive::Display;
use r#macro::{EnumVariants, Id};
use serde::{Deserialize, Serialize};

use crate::command::{Notification, NotificationRecipient};

use self::adapter::HomeAssistantNotificationExecutor;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Display, Id, EnumVariants)]
#[serde(rename_all = "snake_case")]
pub enum NotificationId {
    WindowOpened,
}

impl From<&Notification> for NotificationId {
    fn from(notification: &Notification) -> Self {
        match notification {
            Notification::WindowOpened => Self::WindowOpened,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct NotificationKey {
    recipient: NotificationRecipient,
    notification: NotificationId,
}

struct NotificationService {
    executor: HomeAssistantNotificationExecutor,
    active: RwLock<HashMap<NotificationKey, Notification>>,
}

pub struct NotificationModule {
    service: Arc<NotificationService>,
}

#[derive(Clone)]
pub struct NotificationClient {
    service: Arc<NotificationService>,
}

impl NotificationModule {
    #[allow(clippy::expect_used)]
    pub fn new(url: &str, token: &str) -> Self {
        Self {
            service: Arc::new(NotificationService {
                executor: HomeAssistantNotificationExecutor::new(url, token),
                active: RwLock::new(HashMap::new()),
            }),
        }
    }

    pub fn client(&self) -> NotificationClient {
        NotificationClient {
            service: self.service.clone(),
        }
    }
}

impl NotificationClient {
    #[allow(dead_code)]
    pub fn is_delivered(&self, recipient: &NotificationRecipient, notification: &Notification) -> bool {
        let key = NotificationKey {
            recipient: recipient.clone(),
            notification: NotificationId::from(notification),
        };

        let active = match self.service.active.read() {
            Ok(active) => active,
            Err(poisoned) => poisoned.into_inner(),
        };

        active.get(&key) == Some(notification)
    }

    pub async fn notify(&self, recipient: &NotificationRecipient, notification: &Notification) -> anyhow::Result<()> {
        self.service.executor.notify(recipient, notification).await?;

        let key = NotificationKey {
            recipient: recipient.clone(),
            notification: NotificationId::from(notification),
        };
        let mut active = match self.service.active.write() {
            Ok(active) => active,
            Err(poisoned) => poisoned.into_inner(),
        };
        active.insert(key, notification.clone());

        Ok(())
    }

    pub async fn dismiss(&self, recipient: &NotificationRecipient, notification: NotificationId) -> anyhow::Result<()> {
        self.service.executor.dismiss(recipient, notification).await?;

        let key = NotificationKey {
            recipient: recipient.clone(),
            notification,
        };
        let mut active = match self.service.active.write() {
            Ok(active) => active,
            Err(poisoned) => poisoned.into_inner(),
        };
        active.remove(&key);

        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use mockito::Server;

    use super::*;

    #[tokio::test]
    async fn records_notification_only_after_successful_delivery() {
        let mut server = Server::new_async().await;
        let _mock = server
            .mock("POST", "/api/services/notify/mobile_app_jarvis")
            .with_status(200)
            .create_async()
            .await;
        let module = NotificationModule::new(&server.url(), "token");
        let client = module.client();
        let recipient = NotificationRecipient::Dennis;
        let notification = Notification::WindowOpened;

        client.notify(&recipient, &notification).await.unwrap();

        assert!(client.is_delivered(&recipient, &notification));
    }

    #[tokio::test]
    async fn does_not_record_notification_when_delivery_fails() {
        let mut server = Server::new_async().await;
        let _mock = server
            .mock("POST", "/api/services/notify/mobile_app_jarvis")
            .with_status(500)
            .create_async()
            .await;
        let module = NotificationModule::new(&server.url(), "token");
        let client = module.client();
        let recipient = NotificationRecipient::Dennis;
        let notification = Notification::WindowOpened;

        assert!(client.notify(&recipient, &notification).await.is_err());
        assert!(!client.is_delivered(&recipient, &notification));
    }

    #[tokio::test]
    async fn clears_notification_after_successful_dismissal() {
        let mut server = Server::new_async().await;
        let _mock = server
            .mock("POST", "/api/services/notify/mobile_app_jarvis")
            .with_status(200)
            .expect(2)
            .create_async()
            .await;
        let module = NotificationModule::new(&server.url(), "token");
        let client = module.client();
        let recipient = NotificationRecipient::Dennis;
        let notification = Notification::WindowOpened;

        client.notify(&recipient, &notification).await.unwrap();
        client.dismiss(&recipient, NotificationId::WindowOpened).await.unwrap();

        assert!(!client.is_delivered(&recipient, &notification));
    }
}
