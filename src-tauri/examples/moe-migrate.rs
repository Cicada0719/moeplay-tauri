// 一次性迁移 CLI：把旧版 C# MoeGame（Playnite LiteDB）库灌进新版 SQLite。
//
// 用法（默认从 %LOCALAPPDATA%\MoeGameSetup\library 迁到 %APPDATA%\moeplay\moegame.db）：
//   cargo run --example moe-migrate
// 可选覆盖：
//   cargo run --example moe-migrate -- <旧库目录> <新数据目录>
//
// 复用应用内 `migrate_from_csharp`，因此与 UI 触发的迁移走完全相同的逻辑：
// 自动发现 LiteDB.dll → PowerShell 导出 JSON → 字段映射 upsert → 复制封面/背景 → 校验。

use std::path::PathBuf;

use moeplay_lib::csharp_migration::{migrate_from_csharp, verify_migration_for_ids};
use moeplay_lib::db_sqlite::SqliteDb;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let source = args.get(1).map(PathBuf::from).unwrap_or_else(|| {
        dirs::data_local_dir()
            .expect("无法定位 %LOCALAPPDATA%")
            .join("MoeGameSetup")
            .join("library")
    });

    let data_dir = args.get(2).map(PathBuf::from).unwrap_or_else(|| {
        dirs::data_dir()
            .expect("无法定位 %APPDATA%")
            .join("moeplay")
    });

    std::fs::create_dir_all(&data_dir).expect("创建数据目录失败");
    let db_path = data_dir.join("moegame.db");

    println!("源库:   {}", source.display());
    println!("目标库: {}", db_path.display());
    println!("迁移中…\n");

    let db = SqliteDb::open(&db_path).expect("打开目标 SQLite 失败");

    let report = migrate_from_csharp(&source, &db, &data_dir, &|p| {
        if p.total == 0 || p.current % 50 == 0 || p.current == p.total {
            println!("  [{}] {}/{}  {}", p.stage, p.current, p.total, p.message);
        }
    })
    .expect("迁移失败");

    let verify =
        verify_migration_for_ids(&db, report.total_found, &report.source_ids).expect("校验失败");

    println!("\n==== 迁移报告 ====");
    println!("发现:     {}", report.total_found);
    println!("新增:     {}", report.imported);
    println!("更新:     {}", report.updated);
    println!("跳过:     {}", report.skipped);
    println!("封面复制: {}", report.media_copied);
    println!("封面缺失: {}", report.media_missing);
    println!("耗时:     {:.1}s", report.duration_secs);
    if let Some(backup_dir) = report.backup_dir.as_deref() {
        println!("备份:     {}", backup_dir);
    }
    println!("---- 校验 ----");
    println!("库内游戏: {}", verify.actual_count);
    println!("本次匹配: {}", verify.matched_count);
    println!("缺失:     {}", verify.missing_count);
    println!("有封面:   {}", verify.with_cover);
    println!("有背景:   {}", verify.with_background);
    if !report.errors.is_empty() {
        println!("\n前 10 条错误:");
        for e in report.errors.iter().take(10) {
            println!("  ! {}", e);
        }
    }
}
