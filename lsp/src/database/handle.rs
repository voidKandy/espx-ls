use log::debug;
use tokio::{
    process::{Child, Command},
    task::JoinHandle,
};

use crate::config::DatabaseConfig;

#[derive(Debug)]
/// Handles logic for when database instance is spawned in a child process of the LSP
pub(super) struct DatabaseHandle(JoinHandle<Child>);

impl DatabaseHandle {
    /// Tries to initialize child process handle, if a host is passed, returns None
    pub fn try_init(config: &DatabaseConfig) -> Option<Self> {
        if config.host.is_some() {
            debug!("Host is present in config, Bypassing database handle initialization");
            return None;
        }
        let (user, pass, port) = (
            config.user.to_owned().unwrap_or("root".to_owned()),
            config.pass.to_owned().unwrap_or("root".to_owned()),
            config.port,
        );

        let handle = tokio::task::spawn(async move { Self::start_database(user, pass, port) });
        debug!("Database Handle initialized");
        Some(Self(handle))
    }

    pub async fn kill(self) -> Result<(), std::io::Error> {
        self.0.await.unwrap().kill().await?;
        Ok(())
    }

    pub fn start_database(user: String, pass: String, port: i32) -> Child {
        Command::new("surreal")
            .args([
                "start",
                "--log",
                "debug",
                "--no-banner",
                "trace",
                "--user",
                &user,
                "--pass",
                &pass,
                "--bind",
                &format!("0.0.0.0:{}", port),
            ])
            .spawn()
            .expect("Failed to run database start command")
    }
}
