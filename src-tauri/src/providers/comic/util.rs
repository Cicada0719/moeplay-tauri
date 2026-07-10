use super::{ComicProviderError, ComicResult, HealthTracker};
use crate::domain::ProviderErrorKind;
use serde_json::Value;
use std::future::Future;

pub fn string(value: &Value, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        value
            .get(*key)
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
    })
}

pub fn i64_value(value: &Value, keys: &[&str]) -> Option<i64> {
    keys.iter().find_map(|key| {
        value
            .get(*key)
            .and_then(|v| v.as_i64().or_else(|| v.as_f64().map(|n| n as i64)))
    })
}

pub fn f32_value(value: &Value, keys: &[&str]) -> Option<f32> {
    keys.iter()
        .find_map(|key| value.get(*key).and_then(|v| v.as_f64().map(|n| n as f32)))
}

pub fn array<'a>(value: &'a Value, keys: &[&str]) -> &'a [Value] {
    if let Some(items) = value.as_array() {
        return items;
    }
    for key in keys {
        if let Some(items) = value.get(*key).and_then(Value::as_array) {
            return items;
        }
    }
    &[]
}

pub fn required_id(value: &Value, keys: &[&str]) -> String {
    string(value, keys)
        .or_else(|| i64_value(value, keys).map(|n| n.to_string()))
        .unwrap_or_default()
}

pub fn provider_parse_error(
    provider_id: &str,
    operation: &str,
    message: impl Into<String>,
) -> ComicProviderError {
    ComicProviderError::Provider(crate::domain::ProviderError {
        kind: ProviderErrorKind::ParseChanged,
        message: message.into(),
        retryable: false,
        retry_after_ms: None,
        provider_id: Some(provider_id.to_string()),
        operation: Some(operation.to_string()),
    })
}

pub async fn tracked<T, F>(health: &HealthTracker, operation: &str, future: F) -> ComicResult<T>
where
    F: Future<Output = ComicResult<T>>,
{
    let started = health.before(operation)?;
    let result = future.await;
    match &result {
        Ok(_) => health.success(operation, started),
        Err(error) => health.failure(operation, error),
    }
    result
}

pub fn encode_segment(value: &str) -> String {
    urlencoding::encode(value).into_owned()
}

pub fn parse_number_from_title(title: &str) -> Option<f32> {
    let mut current = String::new();
    for ch in title.chars() {
        if ch.is_ascii_digit() || (ch == '.' && !current.contains('.')) {
            current.push(ch);
        } else if !current.is_empty() {
            break;
        }
    }
    current.parse().ok()
}
