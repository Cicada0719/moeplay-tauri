mod platform;
pub use platform::*;
mod download;
pub use download::*;
mod emulators;
pub use emulators::*;
mod media;
pub use media::*;
mod import;
pub use import::*;
mod settings;
pub use settings::*;
mod secrets;
pub use secrets::*;
mod tasks;
pub use tasks::*;
mod translation;
pub use translation::*;
mod screenshots;
pub use screenshots::*;
mod dashboard;
pub use dashboard::*;
mod data;
pub use data::*;
mod cloud;
pub use cloud::*;
mod diagnostics;
pub use diagnostics::*;
mod games;
pub use games::*;
mod metadata;
pub use metadata::*;
mod play;
#[cfg(test)]
pub(crate) use play::is_platform_launch_uri;
pub use play::*;
mod privacy;
pub use privacy::*;
mod resources;
pub use resources::*;
mod saves;
pub(crate) use saves::resolve_game_dir;
pub use saves::*;
mod system;
pub use system::*;

mod scrape;
pub use scrape::*;

mod comic;
pub use comic::*;

mod manga;
pub use manga::*;

mod anime;
pub use anime::*;

mod activity;
pub use activity::*;

mod library_v2;
pub use library_v2::*;

mod anime_provider;
pub use anime_provider::*;

mod comic_provider;
pub use comic_provider::*;

mod ai_v2;
pub use ai_v2::*;

mod ai_changes;
pub use ai_changes::*;
