// 自动化测试 - 集成测试
use moeplay_lib::utils;

#[test]
fn test_sanitize_filename() {
    assert_eq!(utils::sanitize_filename("hello"), "hello");
    assert_eq!(utils::sanitize_filename("hello/world"), "hello_world");
    assert_eq!(utils::sanitize_filename("test:file"), "test_file");
}

#[test]
fn test_format_file_size() {
    assert_eq!(utils::format_file_size(0), "0.0 B");
    assert_eq!(utils::format_file_size(1024), "1.0 KB");
    assert_eq!(utils::format_file_size(1048576), "1.0 MB");
}

#[test]
fn test_format_duration() {
    assert_eq!(utils::format_duration(30), "30s");
    assert_eq!(utils::format_duration(120), "2m");
    assert_eq!(utils::format_duration(3661), "1h 1m");
}

#[test]
fn test_truncate() {
    assert_eq!(utils::truncate("hello", 10), "hello");
    assert_eq!(utils::truncate("hello world", 5), "hello");
}

#[test]
fn test_truncate_with_ellipsis() {
    assert_eq!(utils::truncate_with_ellipsis("hi", 10), "hi");
    assert_eq!(utils::truncate_with_ellipsis("hello world", 8), "hello...");
}

#[test]
fn test_is_executable() {
    use std::path::Path;
    assert!(utils::is_executable(Path::new("game.exe")));
    assert!(utils::is_executable(Path::new("run.bat")));
    assert!(!utils::is_executable(Path::new("readme.txt")));
}

#[test]
fn test_retry_sync_success() {
    let result = utils::retry_sync(|| Ok::<i32, String>(42), 3);
    assert_eq!(result.unwrap(), 42);
}

#[test]
fn test_retry_sync_failure() {
    let mut count = 0;
    let result = utils::retry_sync(
        || {
            count += 1;
            Err::<i32, String>("fail".into())
        },
        2,
    );
    assert!(result.is_err());
    assert_eq!(count, 3);
}
