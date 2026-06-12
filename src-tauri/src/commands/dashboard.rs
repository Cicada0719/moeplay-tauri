use crate::db::Database;
use crate::models::Game;
use crate::{recommender, stats};
use tauri::State;

#[tauri::command]
pub fn get_recommendations(
    db: State<'_, Database>,
    seed_game_id: Option<String>,
    limit: Option<u32>,
) -> Vec<recommender::Recommendation> {
    recommender::recommend_games(
        &db.get_games(),
        seed_game_id.as_deref(),
        limit.unwrap_or(12) as usize,
    )
}

#[tauri::command]
pub fn get_dashboard_data(db: State<'_, Database>) -> stats::DashboardData {
    stats::generate_dashboard(&db)
}

#[tauri::command]
pub fn get_smart_collections(db: State<'_, Database>) -> Vec<stats::Collection> {
    stats::generate_collections(&db.get_games())
}

#[tauri::command]
pub fn get_collection_games(db: State<'_, Database>, collection_id: String) -> Vec<Game> {
    stats::filter_collection_games(&db.get_games(), &collection_id)
}
