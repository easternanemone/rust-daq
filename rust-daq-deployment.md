# Deployment and Production Guide

## Overview

This guide covers deployment strategies, production considerations, packaging, distribution, and maintenance approaches for Rust DAQ applications built in Rust.

## Build and Packaging

### Optimized Release Builds
```toml
# Cargo.toml - Production optimizations
[profile.release]
opt-level = 3
lto = "fat"              # Link-time optimization
codegen-units = 1        # Better optimization, slower compile
panic = "abort"          # Smaller binary size
strip = true             # Remove debug symbols
overflow-checks = false  # Disable integer overflow checks in release

[profile.release-with-debug]
inherits = "release"
debug = true
strip = false

# Platform-specific optimizations
[target.'cfg(target_arch = "x86_64")']
rustflags = ["-C", "target-cpu=native"]

[target.'cfg(target_arch = "aarch64")']
rustflags = ["-C", "target-cpu=native"]
```

### Cross-Platform Building
```bash
#!/bin/bash
# build-release.sh - Cross-platform build script

set -e

echo "Building for multiple platforms..."

# Install required targets
rustup target add x86_64-pc-windows-gnu
rustup target add x86_64-apple-darwin
rustup target add aarch64-apple-darwin
rustup target add x86_64-unknown-linux-gnu

# Build for Windows
echo "Building for Windows x64..."
cargo build --release --target x86_64-pc-windows-gnu

# Build for macOS Intel
echo "Building for macOS Intel..."
cargo build --release --target x86_64-apple-darwin

# Build for macOS Apple Silicon
echo "Building for macOS Apple Silicon..."
cargo build --release --target aarch64-apple-darwin

# Build for Linux
echo "Building for Linux x64..."
cargo build --release --target x86_64-unknown-linux-gnu

# Create distribution packages
echo "Creating distribution packages..."
./package-releases.sh
```

### Application Packaging
```bash
#!/bin/bash
# package-releases.sh - Create distribution packages

APP_NAME="scientific-daq"
VERSION=$(grep "^version" Cargo.toml | cut -d'"' -f2)

create_package() {
    local target=$1
    local platform=$2
    local extension=$3
    
    echo "Packaging for $platform..."
    
    mkdir -p "dist/$platform"
    
    # Copy binary
    cp "target/$target/release/$APP_NAME$extension" "dist/$platform/"
    
    # Copy configuration files
    cp -r config/ "dist/$platform/"
    
    # Copy documentation
    cp README.md LICENSE "dist/$platform/"
    
    # Create platform-specific package
    case $platform in
        "windows")
            # Create installer using WiX or Inno Setup
            zip -r "dist/${APP_NAME}-${VERSION}-windows.zip" "dist/$platform/"
            ;;
        "macos-intel"|"macos-arm64")
            # Create macOS app bundle
            create_macos_bundle "$platform"
            ;;
        "linux")
            # Create AppImage or deb package
            create_linux_package "$platform"
            ;;
    esac
}

create_macos_bundle() {
    local platform=$1
    local bundle_name="${APP_NAME}.app"
    local bundle_path="dist/$platform/$bundle_name"
    
    mkdir -p "$bundle_path/Contents/MacOS"
    mkdir -p "$bundle_path/Contents/Resources"
    
    # Copy binary
    cp "dist/$platform/$APP_NAME" "$bundle_path/Contents/MacOS/"
    
    # Create Info.plist
    cat > "$bundle_path/Contents/Info.plist" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>$APP_NAME</string>
    <key>CFBundleIdentifier</key>
    <string>com.yourcompany.$APP_NAME</string>
    <key>CFBundleName</key>
    <string>Scientific DAQ</string>
    <key>CFBundleVersion</key>
    <string>$VERSION</string>
    <key>CFBundleShortVersionString</key>
    <string>$VERSION</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
</dict>
</plist>
EOF
    
    # Create DMG
    hdiutil create -volname "$APP_NAME" -srcfolder "dist/$platform" -ov -format UDZO "dist/${APP_NAME}-${VERSION}-${platform}.dmg"
}

# Build packages
create_package "x86_64-pc-windows-gnu" "windows" ".exe"
create_package "x86_64-apple-darwin" "macos-intel" ""
create_package "aarch64-apple-darwin" "macos-arm64" ""
create_package "x86_64-unknown-linux-gnu" "linux" ""
```

## Container Deployment

### Docker Configuration
```dockerfile
# Dockerfile - Multi-stage build for minimal runtime image
FROM rust:1.75 as builder

WORKDIR /app

# Copy dependency files first for better caching
COPY Cargo.toml Cargo.lock ./
COPY src/lib.rs src/lib.rs

# Build dependencies
RUN mkdir src/bin && echo 'fn main() {}' > src/bin/dummy.rs
RUN cargo build --release --bin dummy
RUN rm -rf src/bin

# Copy source code and build application
COPY src/ src/
RUN cargo build --release

# Runtime image
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    libssl3 \
    ca-certificates \
    libusb-1.0-0 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 scidaq

# Copy application
COPY --from=builder /app/target/release/scientific-daq /usr/local/bin/
COPY config/ /app/config/

# Set ownership
RUN chown -R scidaq:scidaq /app

USER scidaq
WORKDIR /app

EXPOSE 8080 8081

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

CMD ["scientific-daq"]
```

### Docker Compose for Development
```yaml
# docker-compose.yml
version: '3.8'

services:
  scientific-daq:
    build: .
    ports:
      - "8080:8080"
      - "8081:8081"
    volumes:
      - ./config:/app/config:ro
      - ./data:/app/data
      - ./logs:/app/logs
    environment:
      - RUST_LOG=info
      - SCIDAQ_CONFIG_PATH=/app/config
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 40s

  database:
    image: postgres:15
    environment:
      - POSTGRES_DB=scidaq
      - POSTGRES_USER=scidaq
      - POSTGRES_PASSWORD=${DB_PASSWORD:-changeme}
    volumes:
      - postgres_data:/var/lib/postgresql/data
      - ./init.sql:/docker-entrypoint-initdb.d/init.sql
    ports:
      - "5432:5432"
    restart: unless-stopped

  redis:
    image: redis:7-alpine
    ports:
      - "6379:6379"
    volumes:
      - redis_data:/data
    restart: unless-stopped

  grafana:
    image: grafana/grafana:latest
    ports:
      - "3000:3000"
    environment:
      - GF_SECURITY_ADMIN_PASSWORD=${GRAFANA_PASSWORD:-admin}
    volumes:
      - grafana_data:/var/lib/grafana
      - ./grafana/dashboards:/etc/grafana/provisioning/dashboards
      - ./grafana/datasources:/etc/grafana/provisioning/datasources
    restart: unless-stopped

volumes:
  postgres_data:
  redis_data:
  grafana_data:
```

## Production Configuration

### Environment-Specific Configuration
```rust
// src/config/production.rs
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductionConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub logging: LoggingConfig,
    pub security: SecurityConfig,
    pub monitoring: MonitoringConfig,
    pub data_retention: DataRetentionConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub bind_address: String,
    pub port: u16,
    pub worker_threads: Option<usize>,
    pub max_connections: usize,
    pub request_timeout_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub tls_enabled: bool,
    pub cert_path: Option<PathBuf>,
    pub key_path: Option<PathBuf>,
    pub api_keys: Vec<String>,
    pub cors_origins: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    pub metrics_enabled: bool,
    pub prometheus_endpoint: String,
    pub health_check_interval_seconds: u64,
    pub log_level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataRetentionConfig {
    pub raw_data_retention_days: u32,
    pub processed_data_retention_days: u32,
    pub backup_enabled: bool,
    pub backup_schedule: String,
    pub backup_location: PathBuf,
}

impl ProductionConfig {
    pub fn load() -> Result<Self, ConfigError> {
        let mut config = config::Config::builder()
            .add_source(config::File::with_name("config/production"))
            .add_source(config::Environment::with_prefix("SCIDAQ"));

        // Override with command-line arguments if provided
        if let Some(config_path) = std::env::args().nth(1) {
            config = config.add_source(config::File::with_name(&config_path));
        }

        config.build()?.try_deserialize()
    }
}
```

### Structured Logging for Production
```rust
// src/logging/production.rs
use tracing::{info, warn, error};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use tracing_appender::rolling::{RollingFileAppender, Rotation};

pub fn init_production_logging(config: &LoggingConfig) -> Result<(), Box<dyn std::error::Error>> {
    let file_appender = RollingFileAppender::new(
        Rotation::daily(),
        &config.log_directory,
        "scientific-daq.log"
    );

    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(non_blocking)
                .with_target(false)
                .with_thread_ids(true)
                .with_file(true)
                .with_line_number(true)
                .json() // Structured JSON logging for production
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(std::io::stdout)
                .with_ansi(config.colored_output)
        )
        .with(
            tracing_subscriber::EnvFilter::new(&config.level)
        )
        .init();

    info!("Production logging initialized");
    Ok(())
}

// Structured logging macros
#[macro_export]
macro_rules! log_instrument_event {
    ($level:ident, $instrument:expr, $event:expr, $($field:tt)*) => {
        tracing::$level!(
            instrument.name = $instrument,
            event = $event,
            $($field)*
        );
    };
}

#[macro_export]
macro_rules! log_data_event {
    ($level:ident, $dataset:expr, $event:expr, $($field:tt)*) => {
        tracing::$level!(
            dataset.id = %$dataset,
            event = $event,
            $($field)*
        );
    };
}
```

## Monitoring and Observability

### Metrics Collection
```rust
// src/metrics/mod.rs
use prometheus::{Counter, Histogram, Gauge, Registry, Encoder, TextEncoder};
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct MetricsCollector {
    registry: Registry,
    
    // Application metrics
    pub data_points_processed: Counter,
    pub active_instruments: Gauge,
    pub buffer_usage: Gauge,
    pub processing_duration: Histogram,
    pub error_count: Counter,
    
    // System metrics
    pub memory_usage: Gauge,
    pub cpu_usage: Gauge,
    pub disk_usage: Gauge,
}

impl MetricsCollector {
    pub fn new() -> Result<Self, prometheus::Error> {
        let registry = Registry::new();
        
        let data_points_processed = Counter::new(
            "scidaq_data_points_total",
            "Total number of data points processed"
        )?;
        
        let active_instruments = Gauge::new(
            "scidaq_active_instruments",
            "Number of currently active instruments"
        )?;
        
        let buffer_usage = Gauge::new(
            "scidaq_buffer_usage_percent",
            "Buffer usage as percentage of capacity"
        )?;
        
        let processing_duration = Histogram::with_opts(
            prometheus::HistogramOpts::new(
                "scidaq_processing_duration_seconds",
                "Duration of data processing operations"
            ).buckets(vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0])
        )?;
        
        let error_count = Counter::new(
            "scidaq_errors_total",
            "Total number of errors encountered"
        )?;
        
        let memory_usage = Gauge::new(
            "scidaq_memory_usage_bytes",
            "Current memory usage in bytes"
        )?;
        
        let cpu_usage = Gauge::new(
            "scidaq_cpu_usage_percent",
            "Current CPU usage percentage"
        )?;
        
        let disk_usage = Gauge::new(
            "scidaq_disk_usage_bytes",
            "Current disk usage in bytes"
        )?;
        
        // Register all metrics
        registry.register(Box::new(data_points_processed.clone()))?;
        registry.register(Box::new(active_instruments.clone()))?;
        registry.register(Box::new(buffer_usage.clone()))?;
        registry.register(Box::new(processing_duration.clone()))?;
        registry.register(Box::new(error_count.clone()))?;
        registry.register(Box::new(memory_usage.clone()))?;
        registry.register(Box::new(cpu_usage.clone()))?;
        registry.register(Box::new(disk_usage.clone()))?;
        
        Ok(Self {
            registry,
            data_points_processed,
            active_instruments,
            buffer_usage,
            processing_duration,
            error_count,
            memory_usage,
            cpu_usage,
            disk_usage,
        })
    }
    
    pub fn export_metrics(&self) -> Result<String, prometheus::Error> {
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        encoder.encode_to_string(&metric_families)
    }
    
    pub async fn start_system_metrics_collection(&self) {
        let memory_gauge = self.memory_usage.clone();
        let cpu_gauge = self.cpu_usage.clone();
        let disk_gauge = self.disk_usage.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
            
            loop {
                interval.tick().await;
                
                // Collect system metrics (implement actual collection)
                if let Ok(memory) = get_memory_usage().await {
                    memory_gauge.set(memory as f64);
                }
                
                if let Ok(cpu) = get_cpu_usage().await {
                    cpu_gauge.set(cpu);
                }
                
                if let Ok(disk) = get_disk_usage().await {
                    disk_gauge.set(disk as f64);
                }
            }
        });
    }
}

// System metrics collection functions
async fn get_memory_usage() -> Result<usize, Box<dyn std::error::Error>> {
    // Implement memory usage collection
    Ok(0) // Placeholder
}

async fn get_cpu_usage() -> Result<f64, Box<dyn std::error::Error>> {
    // Implement CPU usage collection
    Ok(0.0) // Placeholder
}

async fn get_disk_usage() -> Result<usize, Box<dyn std::error::Error>> {
    // Implement disk usage collection
    Ok(0) // Placeholder
}
```

### Health Checks
```rust
// src/health/mod.rs
use serde_json::json;
use std::collections::HashMap;
use tokio::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct HealthCheck {
    name: String,
    status: HealthStatus,
    last_check: Instant,
    details: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

pub struct HealthChecker {
    checks: Vec<Box<dyn HealthCheckProvider>>,
    cache_duration: Duration,
    last_results: Arc<RwLock<Vec<HealthCheck>>>,
}

#[async_trait::async_trait]
pub trait HealthCheckProvider: Send + Sync {
    async fn check_health(&self) -> HealthCheck;
    fn name(&self) -> &str;
}

// Database health check
pub struct DatabaseHealthCheck {
    pool: Arc<sqlx::PgPool>,
}

#[async_trait::async_trait]
impl HealthCheckProvider for DatabaseHealthCheck {
    async fn check_health(&self) -> HealthCheck {
        let start = Instant::now();
        let mut details = HashMap::new();
        
        let status = match sqlx::query("SELECT 1").fetch_one(&*self.pool).await {
            Ok(_) => {
                details.insert("connection".to_string(), json!("ok"));
                HealthStatus::Healthy
            }
            Err(e) => {
                details.insert("error".to_string(), json!(e.to_string()));
                HealthStatus::Unhealthy
            }
        };
        
        let duration = start.elapsed();
        details.insert("response_time_ms".to_string(), json!(duration.as_millis()));
        
        HealthCheck {
            name: "database".to_string(),
            status,
            last_check: Instant::now(),
            details,
        }
    }
    
    fn name(&self) -> &str {
        "database"
    }
}

// Instrument connectivity health check
pub struct InstrumentHealthCheck {
    instruments: Arc<RwLock<HashMap<String, Box<dyn Instrument>>>>,
}

#[async_trait::async_trait]
impl HealthCheckProvider for InstrumentHealthCheck {
    async fn check_health(&self) -> HealthCheck {
        let instruments = self.instruments.read().await;
        let mut details = HashMap::new();
        let mut healthy_count = 0;
        let mut total_count = 0;
        
        for (id, instrument) in instruments.iter() {
            total_count += 1;
            
            let connected = instrument.is_connected().await;
            details.insert(id.clone(), json!({"connected": connected}));
            
            if connected {
                healthy_count += 1;
            }
        }
        
        let status = if healthy_count == total_count {
            HealthStatus::Healthy
        } else if healthy_count > 0 {
            HealthStatus::Degraded
        } else {
            HealthStatus::Unhealthy
        };
        
        details.insert("healthy_instruments".to_string(), json!(healthy_count));
        details.insert("total_instruments".to_string(), json!(total_count));
        
        HealthCheck {
            name: "instruments".to_string(),
            status,
            last_check: Instant::now(),
            details,
        }
    }
    
    fn name(&self) -> &str {
        "instruments"
    }
}

impl HealthChecker {
    pub fn new(cache_duration: Duration) -> Self {
        Self {
            checks: Vec::new(),
            cache_duration,
            last_results: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    pub fn add_check(&mut self, check: Box<dyn HealthCheckProvider>) {
        self.checks.push(check);
    }
    
    pub async fn check_all(&self) -> Vec<HealthCheck> {
        // Check cache first
        {
            let cached = self.last_results.read().await;
            if !cached.is_empty() {
                let cache_age = cached[0].last_check.elapsed();
                if cache_age < self.cache_duration {
                    return cached.clone();
                }
            }
        }
        
        // Perform checks
        let mut results = Vec::new();
        for check in &self.checks {
            let result = check.check_health().await;
            results.push(result);
        }
        
        // Update cache
        *self.last_results.write().await = results.clone();
        
        results
    }
    
    pub async fn overall_status(&self) -> HealthStatus {
        let checks = self.check_all().await;
        
        if checks.iter().all(|c| c.status == HealthStatus::Healthy) {
            HealthStatus::Healthy
        } else if checks.iter().any(|c| c.status == HealthStatus::Unhealthy) {
            HealthStatus::Unhealthy
        } else {
            HealthStatus::Degraded
        }
    }
}
```

## Backup and Recovery

### Automated Backup System
```rust
// src/backup/mod.rs
use chrono::{DateTime, Utc};
use tokio::fs;
use std::path::{Path, PathBuf};

pub struct BackupManager {
    config: BackupConfig,
    scheduler: Option<tokio_cron_scheduler::JobScheduler>,
}

#[derive(Debug, Clone)]
pub struct BackupConfig {
    pub enabled: bool,
    pub schedule: String,
    pub retention_days: u32,
    pub backup_location: PathBuf,
    pub compression: bool,
    pub encryption: bool,
}

impl BackupManager {
    pub async fn new(config: BackupConfig) -> Result<Self, BackupError> {
        let mut manager = Self {
            config,
            scheduler: None,
        };
        
        if manager.config.enabled {
            manager.setup_scheduler().await?;
        }
        
        Ok(manager)
    }
    
    async fn setup_scheduler(&mut self) -> Result<(), BackupError> {
        let scheduler = tokio_cron_scheduler::JobScheduler::new().await?;
        let config = self.config.clone();
        
        scheduler.add(
            tokio_cron_scheduler::Job::new_async(&self.config.schedule, move |_uuid, _l| {
                let config = config.clone();
                Box::pin(async move {
                    if let Err(e) = Self::perform_backup(&config).await {
                        tracing::error!("Backup failed: {}", e);
                    }
                })
            })?
        ).await?;
        
        scheduler.start().await?;
        self.scheduler = Some(scheduler);
        
        Ok(())
    }
    
    async fn perform_backup(config: &BackupConfig) -> Result<(), BackupError> {
        let timestamp = Utc::now();
        let backup_name = format!("backup_{}", timestamp.format("%Y%m%d_%H%M%S"));
        let backup_path = config.backup_location.join(&backup_name);
        
        tracing::info!("Starting backup to {:?}", backup_path);
        
        // Create backup directory
        fs::create_dir_all(&backup_path).await?;
        
        // Backup configuration
        let config_backup = backup_path.join("config");
        Self::copy_directory("config", &config_backup).await?;
        
        // Backup data
        let data_backup = backup_path.join("data");
        Self::copy_directory("data", &data_backup).await?;
        
        // Compress if enabled
        if config.compression {
            Self::compress_backup(&backup_path).await?;
        }
        
        // Clean up old backups
        Self::cleanup_old_backups(&config.backup_location, config.retention_days).await?;
        
        tracing::info!("Backup completed successfully");
        Ok(())
    }
    
    async fn copy_directory(src: &str, dst: &Path) -> Result<(), BackupError> {
        let src_path = Path::new(src);
        
        if !src_path.exists() {
            return Ok(());
        }
        
        fs::create_dir_all(dst).await?;
        
        let mut entries = fs::read_dir(src_path).await?;
        while let Some(entry) = entries.next_entry().await? {
            let src_file = entry.path();
            let dst_file = dst.join(entry.file_name());
            
            if src_file.is_dir() {
                Self::copy_directory(&src_file.to_string_lossy(), &dst_file).await?;
            } else {
                fs::copy(&src_file, &dst_file).await?;
            }
        }
        
        Ok(())
    }
    
    async fn compress_backup(backup_path: &Path) -> Result<(), BackupError> {
        // Implementation would use a compression library like flate2
        tracing::info!("Compressing backup at {:?}", backup_path);
        // Placeholder for compression logic
        Ok(())
    }
    
    async fn cleanup_old_backups(backup_dir: &Path, retention_days: u32) -> Result<(), BackupError> {
        let cutoff = Utc::now() - chrono::Duration::days(retention_days as i64);
        
        let mut entries = fs::read_dir(backup_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            if let Ok(metadata) = entry.metadata().await {
                if let Ok(created) = metadata.created() {
                    let created_datetime: DateTime<Utc> = created.into();
                    
                    if created_datetime < cutoff {
                        tracing::info!("Removing old backup: {:?}", entry.path());
                        if entry.path().is_dir() {
                            fs::remove_dir_all(entry.path()).await?;
                        } else {
                            fs::remove_file(entry.path()).await?;
                        }
                    }
                }
            }
        }
        
        Ok(())
    }
}

#[derive(thiserror::Error, Debug)]
pub enum BackupError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Scheduler error: {0}")]
    Scheduler(#[from] tokio_cron_scheduler::JobSchedulerError),
    
    #[error("Compression error: {0}")]
    Compression(String),
}
```

This deployment guide provides comprehensive strategies for building, packaging, deploying, and maintaining your scientific data acquisition application in production environments with proper monitoring, backup, and recovery capabilities.