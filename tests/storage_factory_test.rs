//! Comprehensive tests for storage writer factory pattern.

use rust_daq::data::storage_factory::StorageWriterRegistry;
use std::sync::{Arc, RwLock};
use std::thread;

#[test]
#[cfg(feature = "storage_csv")]
fn test_concurrent_writer_creation() {
    // Verify thread-safe concurrent access to factory
    let registry = Arc::new(StorageWriterRegistry::new());
    let mut handles = vec![];

    for _ in 0..10 {
        let reg = registry.clone();
        let handle = thread::spawn(move || {
            let writer = reg.create("csv");
            assert!(writer.is_ok());
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }
}

#[test]
#[cfg(feature = "storage_csv")]
fn test_registration_override() {
    use rust_daq::data::storage::CsvWriter;

    let mut registry = StorageWriterRegistry::new();
    let original_count = registry.list_formats().len();

    // Register same format twice - should override
    registry.register("csv", || Box::new(CsvWriter::new()));

    // Count should not increase (overwrite, not append)
    assert_eq!(registry.list_formats().len(), original_count);
    assert!(registry.is_available("csv"));
}

#[test]
fn test_empty_format_name() {
    let mut registry = StorageWriterRegistry::new();

    // Empty string format name should be allowed (though unusual)
    #[cfg(feature = "storage_csv")]
    {
        use rust_daq::data::storage::CsvWriter;
        registry.register("", || Box::new(CsvWriter::new()));
        assert!(registry.is_available(""));
        assert!(registry.create("").is_ok());
    }
}

#[test]
fn test_whitespace_format_name() {
    let mut registry = StorageWriterRegistry::new();

    // Whitespace format names should work (literal string matching)
    #[cfg(feature = "storage_csv")]
    {
        use rust_daq::data::storage::CsvWriter;
        registry.register("  spaces  ", || Box::new(CsvWriter::new()));
        assert!(registry.is_available("  spaces  "));
        assert!(!registry.is_available("spaces")); // Trimming not performed
    }
}

#[test]
fn test_special_characters_in_format_name() {
    let mut registry = StorageWriterRegistry::new();

    #[cfg(feature = "storage_csv")]
    {
        use rust_daq::data::storage::CsvWriter;
        registry.register("my-custom/format:v2", || Box::new(CsvWriter::new()));
        assert!(registry.is_available("my-custom/format:v2"));
        assert!(registry.create("my-custom/format:v2").is_ok());
    }
}

#[test]
#[cfg(feature = "storage_csv")]
fn test_multiple_formats_in_sequence() {
    use rust_daq::data::storage::CsvWriter;

    let mut registry = StorageWriterRegistry::new();

    // Register multiple custom formats
    registry.register("format1", || Box::new(CsvWriter::new()));
    registry.register("format2", || Box::new(CsvWriter::new()));
    registry.register("format3", || Box::new(CsvWriter::new()));

    // All should be available
    assert!(registry.is_available("format1"));
    assert!(registry.is_available("format2"));
    assert!(registry.is_available("format3"));

    // All should create successfully
    assert!(registry.create("format1").is_ok());
    assert!(registry.create("format2").is_ok());
    assert!(registry.create("format3").is_ok());
}

#[test]
fn test_is_available_accuracy() {
    let registry = StorageWriterRegistry::new();

    // is_available should match create() behavior
    let formats = registry.list_formats();

    for format in &formats {
        assert!(registry.is_available(format));
        assert!(registry.create(format).is_ok());
    }

    // Non-existent format
    assert!(!registry.is_available("definitely_not_registered"));
    assert!(registry.create("definitely_not_registered").is_err());
}

#[test]
fn test_list_formats_completeness() {
    let mut registry = StorageWriterRegistry::new();

    #[cfg(feature = "storage_csv")]
    {
        use rust_daq::data::storage::CsvWriter;
        let initial_formats = registry.list_formats();

        // Add custom format
        registry.register("test_format", || Box::new(CsvWriter::new()));

        let updated_formats = registry.list_formats();
        assert_eq!(updated_formats.len(), initial_formats.len() + 1);
        assert!(updated_formats.contains(&"test_format".to_string()));
    }
}

#[test]
fn test_error_message_clarity() {
    let registry = StorageWriterRegistry::new();

    let result = registry.create("nonexistent");
    assert!(result.is_err());

    let err_msg = format!("{}", result.err().unwrap());

    // Error should mention the invalid format
    assert!(err_msg.contains("nonexistent"));

    // Error should list available formats
    assert!(err_msg.contains("Available formats"));

    #[cfg(feature = "storage_csv")]
    {
        // Should list csv as available
        assert!(err_msg.contains("csv"));
    }
}

#[test]
fn test_default_registry_equals_new() {
    let registry1 = StorageWriterRegistry::new();
    let registry2 = StorageWriterRegistry::default();

    // Both should have same formats registered
    assert_eq!(registry1.list_formats(), registry2.list_formats());
}

#[test]
#[cfg(feature = "storage_csv")]
fn test_concurrent_mixed_operations() {
    use rust_daq::data::storage::CsvWriter;

    let registry = Arc::new(RwLock::new(StorageWriterRegistry::new()));
    let mut handles = vec![];

    // Mix of reads and writes
    for i in 0..20 {
        let reg = registry.clone();

        let handle = if i % 2 == 0 {
            // Even threads: create writers (read-only operation)
            thread::spawn(move || {
                let reg_read = reg.read().unwrap();
                let _ = reg_read.create("csv");
            })
        } else {
            // Odd threads: register new formats (write operation)
            thread::spawn(move || {
                let mut reg_write = reg.write().unwrap();
                reg_write.register(&format!("format_{}", i), || Box::new(CsvWriter::new()));
            })
        };

        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    // All odd-numbered formats should exist
    let final_reg = registry.read().unwrap();
    for i in (1..20).step_by(2) {
        assert!(final_reg.is_available(&format!("format_{}", i)));
    }
}

#[test]
fn test_case_sensitive_format_names() {
    let mut registry = StorageWriterRegistry::new();

    #[cfg(feature = "storage_csv")]
    {
        use rust_daq::data::storage::CsvWriter;

        registry.register("CSV", || Box::new(CsvWriter::new()));
        registry.register("csv", || Box::new(CsvWriter::new()));
        registry.register("Csv", || Box::new(CsvWriter::new()));

        // All three should be registered separately (case-sensitive)
        assert!(registry.is_available("CSV"));
        assert!(registry.is_available("csv"));
        assert!(registry.is_available("Csv"));
    }
}
