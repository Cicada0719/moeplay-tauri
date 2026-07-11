use moeplay_lib::{
    db_sqlite::SqliteDb,
    task_queue::{JobOperation, TaskEventLevel, TaskQueue, TaskStatus, JOB_OPERATION_VERSION},
};
use std::sync::Arc;

fn queue() -> (Arc<SqliteDb>, TaskQueue) {
    let database = Arc::new(SqliteDb::open_in_memory().expect("in-memory SQLite database"));
    let queue = TaskQueue::from_database(Arc::clone(&database));
    (database, queue)
}

#[test]
fn persistent_timeline_redacts_sensitive_values_and_retains_only_the_latest_200_events() {
    let (_database, queue) = queue();
    let task = queue
        .enqueue_operation(
            "Verify source".to_owned(),
            JobOperation::ProviderVerify {
                media_type: "comic".to_owned(),
                provider_id: "catalog-a".to_owned(),
            },
            None,
        )
        .expect("enqueue typed verification operation");

    for index in 1..=205 {
        let message = if index == 100 {
            "Authorization: Bearer stage3-super-secret".to_owned()
        } else {
            format!("verification milestone {index}")
        };
        queue
            .append_event(
                &task.id,
                TaskEventLevel::Info,
                "verification_milestone".to_owned(),
                message,
                Some(index as f64 / 205.0),
            )
            .expect("append timeline event");
    }

    let events = queue
        .get_task_events(&task.id, None, 500)
        .expect("read retained timeline");
    assert_eq!(
        events.len(),
        200,
        "timeline must use its fixed retention cap"
    );
    assert_eq!(events.first().expect("first retained event").sequence, 6);
    assert_eq!(events.last().expect("last retained event").sequence, 205);
    assert!(events
        .windows(2)
        .all(|pair| pair[0].sequence < pair[1].sequence));
    assert!(events
        .iter()
        .all(|event| !event.message.contains("stage3-super-secret")));
    assert!(events
        .iter()
        .any(|event| event.message.contains("[REDACTED")));
}

#[test]
fn typed_operation_envelope_survives_reopening_the_same_database() {
    let (database, queue) = queue();
    let created = queue
        .enqueue_operation(
            "Import Steam library".to_owned(),
            JobOperation::Import {
                source: "steam".to_owned(),
                reference_id: "account-42".to_owned(),
            },
            Some("steam-import-account-42".to_owned()),
        )
        .expect("enqueue typed import operation");

    drop(queue);
    let reopened = TaskQueue::from_database(database);
    let detail = reopened
        .get_task_detail(&created.id)
        .expect("read detail after reopening queue");
    let operation = detail.operation.expect("persisted supported operation");

    assert_eq!(operation.version, JOB_OPERATION_VERSION);
    assert_eq!(
        operation.operation,
        JobOperation::Import {
            source: "steam".to_owned(),
            reference_id: "account-42".to_owned(),
        }
    );
    assert_eq!(detail.job.id, created.id);
}

#[test]
fn restart_recovery_pauses_downloads_but_fails_non_replayable_operations() {
    let (database, queue) = queue();
    let download = queue.enqueue("Chapter download".to_owned(), "download".to_owned());
    let update = queue
        .enqueue_operation("Check updates".to_owned(), JobOperation::UpdateCheck, None)
        .expect("enqueue update check");

    queue
        .update(&download.id, Some(TaskStatus::Running), Some(0.4), None)
        .expect("start download");
    queue
        .mark_running(&update.id, None, Some(0.2))
        .expect("start update check");

    drop(queue);
    let reopened = TaskQueue::from_database(database);
    let recovered_download = reopened.get(&download.id).expect("recover download");
    let recovered_update = reopened
        .get_task_center(&update.id)
        .expect("recover non-replayable operation");

    assert_eq!(recovered_download.status, TaskStatus::Paused);
    assert!(recovered_download.recovered);
    assert!(recovered_download.resumable);
    assert_eq!(
        reopened.metadata(&download.id).expect("download metadata")["recoveryReason"],
        "process_restart"
    );

    assert_eq!(recovered_update.status, TaskStatus::Failed);
    assert_eq!(recovered_update.error_kind, Some("unknown".to_owned()));
    assert!(!recovered_update.retryable);
    assert_eq!(
        reopened.metadata(&update.id).expect("update metadata")["recoveryReason"],
        "process_restart"
    );
}

#[test]
fn late_success_cannot_replace_a_cancelled_terminal_operation() {
    let (_database, queue) = queue();
    let task = queue
        .enqueue_operation(
            "Export diagnostics".to_owned(),
            JobOperation::DiagnosticsExport,
            None,
        )
        .expect("enqueue diagnostics operation");
    queue
        .mark_running(&task.id, Some("collecting logs".to_owned()), Some(0.5))
        .expect("start diagnostics operation");
    queue.cancel(&task.id).expect("cancel operation");

    assert!(queue
        .mark_succeeded(&task.id, Some("late worker completion".to_owned()))
        .is_err());
    assert_eq!(
        queue.get(&task.id).expect("read cancelled task").status,
        TaskStatus::Cancelled
    );
    assert!(queue
        .get_task_events(&task.id, None, 200)
        .expect("read timeline")
        .iter()
        .all(|event| event.code != "job_succeeded"));
}
