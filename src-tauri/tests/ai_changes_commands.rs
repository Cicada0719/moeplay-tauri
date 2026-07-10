#![allow(dead_code, unused_imports)]

mod ai {
    pub mod change_set {
        pub use moeplay_lib::ai::change_set::*;
    }
    pub mod contracts {
        pub use moeplay_lib::ai::contracts::*;
    }
}
mod db {
    pub use moeplay_lib::db::*;
}
mod models {
    pub use moeplay_lib::models::*;
}
#[path = "../src/services/ai_changes/mod.rs"]
mod ai_changes;
mod services {
    pub mod ai_changes {
        pub use crate::ai_changes::*;
    }
}
#[path = "../src/commands/ai_changes.rs"]
mod ai_changes_commands;

#[test]
fn command_slice_exports_expected_handlers() {
    let _ = ai_changes_commands::ai_changes_preview;
    let _ = ai_changes_commands::ai_changes_apply;
    let _ = ai_changes_commands::ai_changes_undo;
}
