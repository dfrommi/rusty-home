use moka::future::Cache;

use crate::command::{Notification, NotificationRecipient};

use super::super::NotificationId;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct NotificationKey {
    recipient: NotificationRecipient,
    notification: NotificationId,
}

pub struct NotificationStateRepository {
    active: Cache<NotificationKey, Notification>,
}

impl Default for NotificationStateRepository {
    fn default() -> Self {
        Self {
            active: Cache::builder().build(),
        }
    }
}

impl NotificationStateRepository {
    pub async fn is_active(&self, recipient: &NotificationRecipient, notification: &Notification) -> bool {
        let key = NotificationKey {
            recipient: recipient.clone(),
            notification: NotificationId::from(notification),
        };

        self.active.get(&key).await.as_ref() == Some(notification)
    }

    pub async fn activate(&self, recipient: &NotificationRecipient, notification: &Notification) {
        let key = NotificationKey {
            recipient: recipient.clone(),
            notification: NotificationId::from(notification),
        };
        self.active.insert(key, notification.clone()).await;
    }

    pub async fn deactivate(&self, recipient: &NotificationRecipient, notification: NotificationId) {
        self.active
            .invalidate(&NotificationKey {
                recipient: recipient.clone(),
                notification,
            })
            .await;
    }
}
