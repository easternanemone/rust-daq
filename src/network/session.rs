use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub struct Session {
    pub id: String,
    pub client_id: String,
    pub created_at: u64,
    pub last_heartbeat: u64,
    pub timeout_secs: u64,
}

impl Session {
    pub fn new(id: String, client_id: String, timeout_secs: u64) -> Self {
        let now = Utc::now().timestamp_millis() as u64;
        Self {
            id,
            client_id,
            created_at: now,
            last_heartbeat: now,
            timeout_secs,
        }
    }

    pub fn is_active(&self) -> bool {
        let now = Utc::now().timestamp_millis() as u64;
        let elapsed_secs = (now - self.last_heartbeat) / 1000;
        elapsed_secs < self.timeout_secs
    }

    pub fn update_heartbeat(&mut self) {
        self.last_heartbeat = Utc::now().timestamp_millis() as u64;
    }
}

#[derive(Clone)]
pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<String, Session>>>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn create_session(
        &self,
        session_id: String,
        client_id: String,
        timeout_secs: u64,
    ) -> Session {
        let session = Session::new(session_id.clone(), client_id, timeout_secs);
        self.sessions.write().await.insert(session_id, session.clone());
        session
    }

    pub async fn get_session(&self, session_id: &str) -> Option<Session> {
        self.sessions.read().await.get(session_id).cloned()
    }

    pub async fn update_heartbeat(&self, session_id: &str) -> bool {
        match self.sessions.write().await.get_mut(session_id) {
            Some(session) => {
                session.update_heartbeat();
                true
            }
            None => false,
        }
    }

    pub async fn remove_session(&self, session_id: &str) -> bool {
        self.sessions.write().await.remove(session_id).is_some()
    }

    pub async fn cleanup_expired_sessions(&self) {
        let mut sessions = self.sessions.write().await;
        sessions.retain(|_, session| session.is_active());
    }

    pub async fn get_active_sessions(&self) -> Vec<Session> {
        let sessions = self.sessions.read().await;
        sessions
            .values()
            .filter(|s| s.is_active())
            .cloned()
            .collect()
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_session_creation() {
        let manager = SessionManager::new();
        let session = manager
            .create_session(
                "session-1".to_string(),
                "client-1".to_string(),
                6,
            )
            .await;

        assert_eq!(session.id, "session-1");
        assert_eq!(session.client_id, "client-1");
        assert!(session.is_active());
    }

    #[tokio::test]
    async fn test_heartbeat_update() {
        let manager = SessionManager::new();
        manager
            .create_session(
                "session-1".to_string(),
                "client-1".to_string(),
                6,
            )
            .await;

        assert!(manager.update_heartbeat("session-1").await);
        let session = manager.get_session("session-1").await.unwrap();
        assert!(session.is_active());
    }

    #[tokio::test]
    async fn test_session_removal() {
        let manager = SessionManager::new();
        manager
            .create_session(
                "session-1".to_string(),
                "client-1".to_string(),
                6,
            )
            .await;

        assert!(manager.remove_session("session-1").await);
        assert!(manager.get_session("session-1").await.is_none());
    }
}
