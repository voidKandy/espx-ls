use super::AppSectionState;
use crate::state::SharedState;
use egui::{Layout, RichText, TextEdit, Ui};
use surrealdb::method::Health;
use tokio::{sync::mpsc::error::TryRecvError, task::JoinHandle};

#[derive(Debug)]
pub struct DBSectionState {
    thread_handle: Option<JoinHandle<()>>,
    recv: tokio::sync::mpsc::Receiver<DbUiMessage>,
    sender: Option<tokio::sync::mpsc::Sender<DbUiMessage>>,
    health_status: Option<bool>,
}

enum DbUiMessage {
    Healthy(bool),
}

impl Default for DBSectionState {
    fn default() -> Self {
        let (sender, recv) = tokio::sync::mpsc::channel::<DbUiMessage>(5);
        Self {
            thread_handle: None,
            recv,
            sender: Some(sender),
            health_status: None,
        }
    }
}

impl DBSectionState {
    fn reset_channel(&mut self) {
        let (sender, recv) = tokio::sync::mpsc::channel::<DbUiMessage>(5);
        self.recv = recv;
        self.sender = Some(sender);
    }
}

impl AppSectionState for DBSectionState {
    fn render(&mut self, ui: &mut Ui, state: SharedState) {
        let r = state.get_read().unwrap();
        let thread_state_arc = state.clone();
        ui.with_layout(Layout::top_down_justified(egui::Align::Min), |ui| {
            match self.health_status {
                Some(healthy) => {
                    let message = if healthy { "is" } else { "is not" };
                    ui.label(format!("DB {message} healthy"));
                }
                None => {
                    let health_button = ui.button("Health Check");
                    if health_button.clicked() {
                        if self.thread_handle.is_none() {
                            let sender = self.sender.take().expect("No sender?");
                            self.thread_handle = Some(tokio::task::spawn(async move {
                                let r = thread_state_arc.get_read().unwrap();
                                if let Some(db) = r.database.as_ref() {
                                    let is_healthy = db.client.health().await.is_ok();
                                    sender
                                        .send(DbUiMessage::Healthy(is_healthy))
                                        .await
                                        .expect("failed to send");
                                }
                            }));
                        }
                    }
                }
            }

            if let Some(db) = r.database.as_ref() {
                let namespace =
                    RichText::new(format!("Namespace: {} ", db.config.namespace)).size(20.);
                let database =
                    RichText::new(format!("Database: {} ", db.config.database)).size(20.);
                ui.label(namespace);
                ui.label(database);
            }
            drop(r);
        });
        if self.thread_handle.is_some() {
            match self.recv.try_recv() {
                Err(TryRecvError::Empty) => {
                    ui.spinner();
                }
                Err(TryRecvError::Disconnected) => {
                    self.thread_handle = None;
                    self.reset_channel();
                }
                Ok(mes) => {
                    let DbUiMessage::Healthy(healthy) = mes;
                    self.health_status = Some(healthy);
                }
            }
        }
    }
}
