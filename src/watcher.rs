use crate::Config;
use async_watcher::{
    AsyncDebouncer,
    notify::{EventKind, RecursiveMode},
};
use std::{path::PathBuf, time::Duration};
use tokio::sync::watch::Sender;
use log::debug;

pub async fn watch(
    config_path: PathBuf,
    env_path: Option<PathBuf>,
    tx: Sender<Config>,
) -> anyhow::Result<()> {
    // Initialize the debouncer
    let (mut debouncer, mut file_events) =
        AsyncDebouncer::new_with_channel(Duration::from_secs(1), Some(Duration::from_secs(1)))
            .await?;

    // Watch config path
    let config_path = PathBuf::from(shellexpand::tilde(&config_path.to_string_lossy()).as_ref());
    debouncer
        .watcher()
        .watch(config_path.as_ref(), RecursiveMode::Recursive)
        .unwrap();

    // Watch for env
    let env_file = if let Some(env_path) = env_path {
        let env_file = PathBuf::from(shellexpand::tilde(&env_path.to_string_lossy()).as_ref());
        if tokio::fs::try_exists(&env_file).await.is_ok() {
            debouncer
                .watcher()
                .watch(
                    env_file.parent().unwrap().as_ref(),
                    RecursiveMode::NonRecursive,
                )
                .unwrap();
        }
        Some(env_file)
    } else {
        None
    };

    // Wait for events
    while let Some(Ok(events)) = file_events.recv().await {
        // Check if any file was touched
        let needs_reload = events.iter().any(|e| {
            // Check event type
            let is_relevant_event = matches!(
                e.event.kind,
                EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_)
            );
            if !is_relevant_event {
                return false;
            }

            // Check event path
            if let Some(ref env) = env_file {
                let env_in_event = e.event.paths.iter().any(|p| p == env);
                let config_in_event = e.event.paths.iter().any(|p| p.starts_with(&config_path));

                // Unwatch env if it was removed
                if env_in_event && matches!(e.event.kind, EventKind::Remove(_)) {
                    if let Some(parent) = env.parent() {
                        let _ = debouncer.watcher().unwatch(parent.as_ref());
                    }
                }

                return env_in_event || config_in_event;
            }

            true
        });

        // Idle
        if !needs_reload {
            continue;
        }

        // Reload config
        match Config::load(&config_path).await {
            Ok(new_config) => {
                let _ = tx.send(new_config);
            }
            Err(e) => eprintln!("Config reload failed: {e:?}\nEmpty config loaded"),
        }
    }

    Ok(())
}
