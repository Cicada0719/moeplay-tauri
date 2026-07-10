#![allow(dead_code)]

pub mod ai {
    pub use moeplay_lib::ai::*;
}
pub mod db_sqlite {
    pub use moeplay_lib::db_sqlite::*;
}
pub mod secret_store {
    pub use moeplay_lib::secret_store::*;
}
pub mod task_queue {
    pub use moeplay_lib::task_queue::*;
}

#[path = "../src/commands/ai_v2.rs"]
mod ai_v2;

#[test]
fn command_error_dto_is_secret_free_and_stable() {
    let error = ai_v2::AiV2ErrorDto {
        kind: ai::AiErrorKind::Auth,
        message: "credential rejected".to_string(),
        retryable: false,
        retry_after_ms: None,
    };
    let encoded = serde_json::to_string(&error).unwrap();
    assert_eq!(
        encoded,
        r#"{"kind":"auth","message":"credential rejected","retryable":false,"retryAfterMs":null}"#
    );
}
