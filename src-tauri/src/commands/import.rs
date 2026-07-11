use crate::db::Database;
use crate::task_queue::{JobOperation, TaskQueue};
use std::path::PathBuf;
use tauri::{Emitter, State};

#[tauri::command]
pub fn extract_archive_command(
    archive_path: String,
    output_base: Option<String>,
) -> Result<crate::archive::ExtractResult, String> {
    let config = crate::archive::ExtractConfig {
        output_base: output_base.map(PathBuf::from).unwrap_or_default(),
        ..Default::default()
    };
    crate::archive::extract_archive(&PathBuf::from(&archive_path), &config, &|_, _, _| {})
}

#[tauri::command]
pub fn scan_directory_for_games(
    db: State<'_, Database>,
    app_handle: tauri::AppHandle,
    dir: String,
) -> Result<(usize, usize), String> {
    let dir_path = PathBuf::from(&dir);
    if !dir_path.is_dir() {
        return Err("Directory does not exist".into());
    }
    let _ = app_handle.emit(
        "import-event",
        crate::import::ImportEvent::ScanStarted { dir: dir.clone() },
    );

    let mut state = crate::auto_scrape::PipelineState::default();
    crate::auto_scrape::run_full_pipeline(&db, &dir_path, false, &mut state);

    let _ = app_handle.emit(
        "import-event",
        crate::import::ImportEvent::ScanFinished {
            dir,
            imported: state.imported,
            skipped: state.skipped,
        },
    );
    Ok((state.imported, state.skipped))
}

#[tauri::command]
pub fn start_import_watcher_cmd(
    watcher: State<'_, crate::import::ImportWatcher>,
    db: State<'_, Database>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let settings = db.get_settings();
    watcher.start(app_handle, settings.watch_dirs)
}

#[tauri::command]
pub fn stop_import_watcher_cmd(
    watcher: State<'_, crate::import::ImportWatcher>,
) -> Result<(), String> {
    watcher.stop();
    Ok(())
}

#[tauri::command]
pub fn get_import_watcher_status(watcher: State<'_, crate::import::ImportWatcher>) -> bool {
    watcher.is_running()
}

/// 导入候选预览（不去重写入，只返回候选列表供前端勾选）
#[derive(Debug, Clone, serde::Serialize)]
pub struct ImportPreviewCandidate {
    pub name: String,
    pub exe_path: String,
    pub install_dir: String,
    pub engine: Option<String>,
    pub is_duplicate: bool,
}

#[tauri::command]
pub fn preview_directory_for_games(
    db: State<'_, Database>,
    dir: String,
) -> Result<Vec<ImportPreviewCandidate>, String> {
    use std::path::PathBuf;

    let dir_path = PathBuf::from(&dir);
    if !dir_path.is_dir() {
        return Err("Directory does not exist".into());
    }

    let mut state = crate::auto_scrape::PipelineState::default();
    let candidates = crate::auto_scrape::detect_games(&dir_path, &mut state);
    let existing = db.get_games();

    let mut out = Vec::new();
    for c in candidates {
        // 预览阶段暂不展示压缩包候选
        if c.is_archive {
            continue;
        }
        let Some(ref exe_path) = c.best_exe else {
            continue;
        };
        let exe = exe_path.to_string_lossy().to_string();
        let name = crate::auto_scrape::infer_title_from_folder(&c.suggested_name);
        let is_duplicate = crate::auto_scrape::is_duplicate(&name, &exe, &existing);
        let install_dir = exe_path
            .parent()
            .unwrap_or(&dir_path)
            .to_string_lossy()
            .to_string();

        out.push(ImportPreviewCandidate {
            name,
            exe_path: exe,
            install_dir,
            engine: c.engine,
            is_duplicate,
        });
    }

    Ok(out)
}

/// 根据用户勾选的 exe 路径批量导入
#[tauri::command]
pub fn import_selected_candidates(
    app_handle: tauri::AppHandle,
    queue: State<'_, TaskQueue>,
    paths: Vec<String>,
) -> Result<(usize, usize), String> {
    use std::path::PathBuf;

    let total = paths.len();
    let job = queue.enqueue_operation(
        format!("导入 {total} 个候选游戏"),
        JobOperation::Import {
            source: "selected_candidates".to_string(),
            reference_id: total.to_string(),
        },
        Some(format!("import:selected-candidates:{total}")),
    )?;
    let cancellation = queue.register_operation(&job.id)?;
    queue.mark_running(
        &job.id,
        Some("正在导入已选择的候选游戏".to_string()),
        Some(0.0),
    )?;

    let mut imported = 0;
    let mut skipped = 0;
    for (index, path_str) in paths.into_iter().enumerate() {
        if cancellation.is_cancelled() {
            return Err("任务已取消".to_string());
        }
        let path = PathBuf::from(&path_str);
        if !crate::import::is_executable(&path) {
            skipped += 1;
        } else {
            let install_dir = path.parent().unwrap_or_else(|| std::path::Path::new("."));
            match crate::import::import_game_smart(&app_handle, &path, install_dir) {
                Ok(_) => imported += 1,
                Err(_) => skipped += 1,
            }
        }
        let progress = (index + 1) as f64 / total.max(1) as f64;
        queue.update(&job.id, None, Some(progress), None)?;
    }
    queue.mark_succeeded(
        &job.id,
        Some(format!("导入完成：{imported} 个新增，{skipped} 个跳过")),
    )?;
    Ok((imported, skipped))
}
