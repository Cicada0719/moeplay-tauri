// Keep the module paths stable for callers that want either the grouped module or
// the repository types directly from `db_sqlite::repositories`.
pub mod activity;
pub mod ai_results;
pub mod jobs;
pub mod progress;
pub mod provider_config;
pub mod provider_health;

pub use activity::{
    ActivityAggregateQuery, ActivityCursor, ActivityDayAggregate, ActivityPage, ActivityQuery,
    ActivityRepository,
};
pub use ai_results::{AiTaskResultRecord, AiTaskResultRepository};
pub use jobs::BackgroundJobRepository;
pub use progress::ProgressRepository;
pub use provider_config::{
    ensure_non_secret_config, ProviderConfigRecord, ProviderConfigRepository, ProviderConfigUpsert,
};
pub use provider_health::ProviderHealthRepository;
