use super::contracts::{LaunchDescriptor, LaunchErrorKind, LaunchOutcome};
use crate::models::Game;
use std::io;
use std::path::Path;
use std::process::Command;

pub fn launch_descriptor(game: &Game) -> LaunchDescriptor {
    if let Some(uri) = game
        .launch_uri
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return LaunchDescriptor::Uri {
            uri: uri.to_string(),
        };
    }
    if game.exe_path.trim().is_empty() {
        return LaunchDescriptor::Unavailable {
            reason: "game has no executable path or launch URI".to_string(),
        };
    }
    LaunchDescriptor::Executable {
        path: game.exe_path.clone(),
        args: Vec::new(),
        working_dir: game.install_dir.clone().or_else(|| {
            Path::new(&game.exe_path)
                .parent()
                .map(|path| path.to_string_lossy().to_string())
        }),
    }
}

pub fn launch(descriptor: LaunchDescriptor) -> LaunchOutcome {
    match descriptor.clone() {
        LaunchDescriptor::Unavailable { reason } => LaunchOutcome::Failed {
            descriptor,
            error_kind: LaunchErrorKind::InvalidDescriptor,
            message: reason,
            retryable: false,
        },
        LaunchDescriptor::Uri { ref uri } => {
            if !supported_uri(uri) {
                return LaunchOutcome::Failed {
                    descriptor,
                    error_kind: LaunchErrorKind::UnsupportedScheme,
                    message: "launch URI scheme is not allowed".to_string(),
                    retryable: false,
                };
            }
            match open::that(uri) {
                Ok(()) => LaunchOutcome::Delegated { descriptor },
                Err(error) => launch_error(descriptor, &error),
            }
        }
        LaunchDescriptor::Executable {
            ref path,
            ref args,
            ref working_dir,
        } => {
            if !Path::new(path).is_file() {
                return LaunchOutcome::Failed {
                    descriptor,
                    error_kind: LaunchErrorKind::NotFound,
                    message: format!("launch executable not found: {path}"),
                    retryable: true,
                };
            }
            let mut command = Command::new(path);
            command.args(args);
            if let Some(dir) = working_dir {
                command.current_dir(dir);
            }
            match command.spawn() {
                Ok(child) => LaunchOutcome::Started {
                    descriptor,
                    pid: Some(child.id()),
                },
                Err(error) => launch_error(descriptor, &error),
            }
        }
    }
}

fn supported_uri(uri: &str) -> bool {
    let Some((scheme, _)) = uri.split_once(':') else {
        return false;
    };
    matches!(
        scheme.to_ascii_lowercase().as_str(),
        "steam" | "epic" | "com.epicgames.launcher" | "goggalaxy" | "http" | "https"
    )
}

fn launch_error(descriptor: LaunchDescriptor, error: &io::Error) -> LaunchOutcome {
    let (error_kind, retryable) = match error.kind() {
        io::ErrorKind::NotFound => (LaunchErrorKind::NotFound, true),
        io::ErrorKind::PermissionDenied => (LaunchErrorKind::PermissionDenied, false),
        _ => (LaunchErrorKind::SpawnFailed, true),
    };
    LaunchOutcome::Failed {
        descriptor,
        error_kind,
        message: error.to_string(),
        retryable,
    }
}
