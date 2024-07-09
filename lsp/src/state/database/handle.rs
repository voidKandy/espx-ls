use std::{
    process::{Child, Command},
    thread::{self, JoinHandle},
};
// use tokio::task::JoinHandle;
use tracing::debug;

use crate::config::DatabaseConfig;

#[derive(Debug)]
/// Handles logic for when database instance is spawned in a child process of the LSP
pub(super) struct DatabaseHandle(JoinHandle<Child>);

impl DatabaseHandle {
    /// Tries to initialize child process handle, if a host is passed, returns None
    pub(super) fn try_init(config: &DatabaseConfig) -> Option<Self> {
        if config.host.is_some() {
            debug!("Host is present in config, Bypassing database handle initialization");
            return None;
        }
        let (user, pass, port) = (
            config.user.to_owned().unwrap_or("root".to_owned()),
            config.pass.to_owned().unwrap_or("root".to_owned()),
            config.port,
        );
        debug!(
            "Initializing database in child process. User: {} Pass: {}",
            user, pass
        );

        let handle = thread::spawn(move || Self::start_database(user, pass, port));
        debug!("Database Handle initialized");
        Some(Self(handle))
    }

    pub(super) fn kill(self) -> anyhow::Result<()> {
        self.0
            .join()
            .map_err(|err| {
                anyhow::anyhow!(
                    "an error occurred when joining the database child handle: {:?}",
                    err
                )
            })?
            .kill()?;
        Ok(())
    }

    pub(super) fn start_database(user: String, pass: String, port: i32) -> Child {
        Command::new("surreal")
            .args([
                "start",
                "--log",
                "error",
                "--no-banner",
                "--user",
                &user,
                "--pass",
                &pass,
                "--bind",
                &format!("0.0.0.0:{}", port),
                "memory",
            ])
            .spawn()
            .expect("Failed to run database start command")
    }
}
