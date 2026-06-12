use crate::db::Database;
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
