use anyhow::anyhow;
use std::{
    path::PathBuf,
    process::{Child, Command, Stdio},
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
        let cfg = config.clone();
        let handle = thread::spawn(move || {
            debug!("database running in child process");
            Self::start_database(&cfg)
        });
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

    #[tracing::instrument(name = "clear database port")]
    fn clear_database_port(port: i32) -> anyhow::Result<()> {
        let _ = Command::new("lsof")
            .args(["-i", &format!(":{}", port)])
            .stdout(Stdio::piped())
            .spawn()?;

        // debug!("lsof return: {:?}", lsof.stdout);
        let _ = Command::new("awk")
            .arg("NR>1 {print $2}")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;
        // debug!("awk return: {:?}", awk.stdout);

        let out = Command::new("xargs")
            .args(["kill", "-9"])
            .stdin(Stdio::piped())
            .output()?;

        debug!("output return: {:?}", out.stdout);
        if !out.status.success() {
            return Err(anyhow!("output return failure: {:?}", out.status.code()));
        }
        Ok(())
    }

    fn start_database(cfg: &DatabaseConfig) -> Child {
        // Self::clear_database_port(cfg.port).expect("could not clear database port");
        // let mut memory_store_path = std::env::current_dir().unwrap().canonicalize().unwrap();
        // memory_store_path.push(PathBuf::from(".espx-ls/db.surql"));

        Command::new("surreal")
            .args([
                "start",
                "--log",
                "error",
                "--no-banner",
                "--user",
                &cfg.user,
                "--pass",
                &cfg.pass,
                "--bind",
                &format!("0.0.0.0:{}", cfg.port),
                // &format!("file:{}", memory_store_path.to_str().unwrap()),
                "memory",
            ])
            .spawn()
            .expect("Failed to run database start command")
    }
}
