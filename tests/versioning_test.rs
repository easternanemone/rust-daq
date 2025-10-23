use anyhow::Result;
use rust_daq::config::versioning::{VersionManager, VersionId};
use rust_daq::config::{ApplicationSettings, Settings, StorageSettings};
use std::collections::HashMap;
use tempfile::tempdir;

fn create_test_settings() -> Settings {
    Settings {
        log_level: "info".to_string(),
        application: ApplicationSettings {
            broadcast_channel_capacity: 1024,
            command_channel_capacity: 32,
        },
        storage: StorageSettings {
            default_path: "/tmp".to_string(),
            default_format: "csv".to_string(),
        },
        instruments: HashMap::new(),
        processors: None,
    }
}

#[tokio::test]
async fn test_snapshot_creation() -> Result<()> {
    let dir = tempdir()?;
    let version_manager = VersionManager::new(dir.path().to_path_buf());
    let settings = create_test_settings();

    let version_id = version_manager.create_snapshot(&settings, None).await?;
    let versions = version_manager.list_versions().await?;

    assert_eq!(versions.len(), 1);
    assert_eq!(versions[0].id, version_id);

    Ok(())
}

#[tokio::test]
async fn test_rollback_restores_settings() -> Result<()> {
    let dir = tempdir()?;
    let version_manager = VersionManager::new(dir.path().to_path_buf());
    let mut settings = create_test_settings();

    let version_id_1 = version_manager.create_snapshot(&settings, None).await?;

    settings.log_level = "debug".to_string();
    let version_id_2 = version_manager.create_snapshot(&settings, None).await?;

    let rolled_back_settings = version_manager.rollback(&version_id_1).await?;
    assert_eq!(rolled_back_settings.log_level, "info");

    let rolled_back_settings_2 = version_manager.rollback(&version_id_2).await?;
    assert_eq!(rolled_back_settings_2.log_level, "debug");

    Ok(())
}

#[tokio::test]
async fn test_auto_cleanup_keeps_n_versions() -> Result<()> {
    let dir = tempdir()?;
    let version_manager = VersionManager::new(dir.path().to_path_buf());
    let settings = create_test_settings();

    for _ in 0..15 {
        version_manager.create_snapshot(&settings, None).await?;
    }

    let versions = version_manager.list_versions().await?;
    assert_eq!(versions.len(), 10);

    Ok(())
}

#[tokio::test]
async fn test_labeled_snapshots_preserved() -> Result<()> {
    let dir = tempdir()?;
    let version_manager = VersionManager::new(dir.path().to_path_buf());
    let settings = create_test_settings();

    version_manager
        .create_snapshot(&settings, Some("labeled".to_string()))
        .await?;

    for _ in 0..15 {
        version_manager.create_snapshot(&settings, None).await?;
    }

    let versions = version_manager.list_versions().await?;
    assert_eq!(versions.len(), 11); // 10 unlabeled + 1 labeled

    let labeled_version = versions.iter().find(|v| v.label.is_some()).unwrap();
    assert_eq!(labeled_version.label.as_deref(), Some("labeled"));

    Ok(())
}

#[tokio::test]
async fn test_diff_generation() -> Result<()> {
    let dir = tempdir()?;
    let version_manager = VersionManager::new(dir.path().to_path_buf());
    let mut settings = create_test_settings();

    let version_id_1 = version_manager.create_snapshot(&settings, None).await?;

    settings.log_level = "debug".to_string();
    let version_id_2 = version_manager.create_snapshot(&settings, None).await?;

    let diff = version_manager
        .diff_versions(&version_id_1, &version_id_2)
        .await?;

    assert!(diff.contains("-log_level = \"info\""));
    assert!(diff.contains("+log_level = \"debug\""));

    Ok(())
}