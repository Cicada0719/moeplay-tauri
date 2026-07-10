use serde_json::Value;
use std::collections::BTreeMap;
use url::Url;

const SENSITIVE_HEADERS: &[&str] = &[
    "authorization",
    "proxy-authorization",
    "x-api-key",
    "api-key",
    "cookie",
    "set-cookie",
];

pub fn redact_headers(headers: &BTreeMap<String, String>) -> BTreeMap<String, String> {
    headers
        .iter()
        .map(|(name, value)| {
            let redacted = if SENSITIVE_HEADERS
                .iter()
                .any(|sensitive| name.eq_ignore_ascii_case(sensitive))
            {
                "[REDACTED]".to_string()
            } else {
                redact_text(value, &[])
            };
            (name.clone(), redacted)
        })
        .collect()
}

pub fn redact_url(raw: &str) -> String {
    let Ok(mut url) = Url::parse(raw) else {
        return "[INVALID_URL]".to_string();
    };
    let _ = url.set_username("");
    let _ = url.set_password(None);
    url.set_query(None);
    url.set_fragment(None);
    url.to_string()
}

pub fn redact_json(value: &Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(key, value)| {
                    let lower = key.to_ascii_lowercase();
                    let redacted = if lower.contains("secret")
                        || lower.contains("token")
                        || lower.contains("password")
                        || lower.contains("api_key")
                        || lower.contains("apikey")
                        || lower == "authorization"
                        || lower == "cookie"
                    {
                        Value::String("[REDACTED]".to_string())
                    } else {
                        redact_json(value)
                    };
                    (key.clone(), redacted)
                })
                .collect(),
        ),
        Value::Array(values) => Value::Array(values.iter().map(redact_json).collect()),
        Value::String(value) => Value::String(redact_text(value, &[])),
        other => other.clone(),
    }
}

/// Redacts explicitly supplied sentinels plus common bearer/key patterns.
pub fn redact_text(input: &str, sentinels: &[&str]) -> String {
    let mut output = input.to_string();
    for sentinel in sentinels.iter().filter(|value| !value.is_empty()) {
        output = output.replace(sentinel, "[REDACTED]");
    }

    let mut redact_next = false;
    output
        .split_whitespace()
        .map(|word| {
            let lower = word.to_ascii_lowercase();
            if redact_next {
                redact_next = false;
                return "[REDACTED]".to_string();
            }
            if lower == "bearer" || lower.ends_with("=bearer") || lower.ends_with(":bearer") {
                redact_next = true;
                return "[REDACTED]".to_string();
            }
            if lower.starts_with("bearer-")
                || lower.starts_with("bearer_")
                || lower.starts_with("sk-")
                || lower.starts_with("key-")
            {
                return "[REDACTED]".to_string();
            }
            word.to_string()
        })
        .collect::<Vec<_>>()
        .join(" ")
}
