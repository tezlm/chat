use std::sync::Arc;

use dashmap::DashMap;
use matrix_sdk::ruma::OwnedRoomId;
use serde::Deserialize;
use tokio::sync::mpsc;
use types::ThreadId;

use crate::data::MessageMetadata;
use crate::matrix::MatrixMessage;
use crate::portal::{Portal, PortalMessage};
use crate::util::MatrixRoomId;
use crate::chat::UnnamedMessage;

#[derive(Clone)]
pub struct Globals {
    pub pool: sqlx::SqlitePool,
    pub config: Config,
    pub portals: Arc<DashMap<ThreadId, mpsc::UnboundedSender<PortalMessage>>>,
    pub last_ids: Arc<DashMap<ThreadId, MessageMetadata>>,
    pub mx_chan: mpsc::Sender<MatrixMessage>,
    pub ch_chan: mpsc::Sender<UnnamedMessage>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub portal: Vec<ConfigPortal>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConfigPortal {
    pub my_thread_id: ThreadId,
    pub matrix_room_id: OwnedRoomId,
}

impl Config {
    pub fn portal_by_matrix_id(&self, id: &MatrixRoomId) -> Option<&ConfigPortal> {
        self.portal.iter().find(|i| &i.matrix_room_id == id)
    }

    pub fn portal_by_thread_id(&self, id: ThreadId) -> Option<&ConfigPortal> {
        self.portal.iter().find(|i| i.my_thread_id == id)
    }
}

pub trait GlobalsTrait {
    fn portal_send(&mut self, thread_id: ThreadId, msg: PortalMessage);
    fn portal_send_mx(&mut self, matrix_id: &MatrixRoomId, msg: PortalMessage);
}

impl GlobalsTrait for Arc<Globals> {
    fn portal_send(&mut self, thread_id: ThreadId, msg: PortalMessage) {
        let Some(config) = self.config.portal_by_thread_id(thread_id) else {
            return;
        };
        let portal = self
            .portals
            .entry(config.my_thread_id)
            .or_insert_with(|| Portal::summon(self.clone(), config.to_owned()));
        let _ = portal.send(msg);
    }

    fn portal_send_mx(&mut self, channel_id: &MatrixRoomId, msg: PortalMessage) {
        let Some(config) = self.config.portal_by_matrix_id(channel_id) else {
            return;
        };
        let portal = self
            .portals
            .entry(config.my_thread_id)
            .or_insert_with(|| Portal::summon(self.clone(), config.to_owned()));
        let _ = portal.send(msg);
    }
}
