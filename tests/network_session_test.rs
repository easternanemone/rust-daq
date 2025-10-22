#[cfg(feature = "networking")]
mod network_session_tests {
    use rust_daq::network::session::SessionManager;
    use std::time::Duration;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_session_creation() {
        let manager = SessionManager::new();
        let session = manager
            .create_session("sess-1".to_string(), "client-1".to_string(), 6)
            .await;

        assert_eq!(session.id, "sess-1");
        assert_eq!(session.client_id, "client-1");
        assert!(session.is_active());
    }

    #[tokio::test]
    async fn test_session_retrieval() {
        let manager = SessionManager::new();
        manager
            .create_session("sess-2".to_string(), "client-2".to_string(), 6)
            .await;

        let retrieved = manager.get_session("sess-2").await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().client_id, "client-2");
    }

    #[tokio::test]
    async fn test_session_not_found() {
        let manager = SessionManager::new();
        let session = manager.get_session("nonexistent").await;
        assert!(session.is_none());
    }

    #[tokio::test]
    async fn test_heartbeat_update() {
        let manager = SessionManager::new();
        manager
            .create_session("sess-3".to_string(), "client-3".to_string(), 6)
            .await;

        let original = manager.get_session("sess-3").await.unwrap();
        let original_hb = original.last_heartbeat;

        sleep(Duration::from_millis(100)).await;
        let updated = manager.update_heartbeat("sess-3").await;
        assert!(updated);

        let new_session = manager.get_session("sess-3").await.unwrap();
        assert!(new_session.last_heartbeat >= original_hb);
    }

    #[tokio::test]
    async fn test_session_removal() {
        let manager = SessionManager::new();
        manager
            .create_session("sess-4".to_string(), "client-4".to_string(), 6)
            .await;

        assert!(manager.get_session("sess-4").await.is_some());
        let removed = manager.remove_session("sess-4").await;
        assert!(removed);
        assert!(manager.get_session("sess-4").await.is_none());
    }

    #[tokio::test]
    async fn test_cleanup_expired_sessions() {
        let manager = SessionManager::new();

        manager
            .create_session("sess-5".to_string(), "client-5".to_string(), 1)
            .await;

        manager
            .create_session("sess-6".to_string(), "client-6".to_string(), 100)
            .await;

        sleep(Duration::from_secs(2)).await;

        manager.cleanup_expired_sessions().await;

        assert!(manager.get_session("sess-5").await.is_none());
        assert!(manager.get_session("sess-6").await.is_some());
    }

    #[tokio::test]
    async fn test_get_active_sessions() {
        let manager = SessionManager::new();

        manager
            .create_session("sess-7".to_string(), "client-7".to_string(), 10)
            .await;

        manager
            .create_session("sess-8".to_string(), "client-8".to_string(), 10)
            .await;

        manager
            .create_session("sess-9".to_string(), "client-9".to_string(), 1)
            .await;

        sleep(Duration::from_secs(2)).await;

        let active = manager.get_active_sessions().await;
        let active_ids: Vec<_> = active.iter().map(|s| s.id.clone()).collect();

        assert!(active_ids.contains(&"sess-7".to_string()));
        assert!(active_ids.contains(&"sess-8".to_string()));
        assert!(!active_ids.contains(&"sess-9".to_string()));
    }

    #[tokio::test]
    async fn test_multiple_clients_same_manager() {
        let manager = SessionManager::new();

        for i in 0..10 {
            manager
                .create_session(
                    format!("sess-{}", i),
                    format!("client-{}", i),
                    6,
                )
                .await;
        }

        let active = manager.get_active_sessions().await;
        assert_eq!(active.len(), 10);
    }
}
