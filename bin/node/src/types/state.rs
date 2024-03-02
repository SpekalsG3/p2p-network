use std::collections::HashMap;
use std::net::{SocketAddr, TcpStream};
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::sync::mpsc::Sender;
use std::time::SystemTime;
use anyhow::{anyhow, Result};
use crate::commands::NodeCommand;
use crate::types::package::AppPackage;

pub struct MetaData {
    pub(crate) ping: u16,
    pub(crate) ping_started_at: Option<SystemTime>,
    pub(crate) topology_rad: f32, // angel relative to the first connection, used to determine who's closer to another user
    pub(crate) connected_to: Vec<SocketAddr>,
}

pub(crate) struct AppStateInnerRef {}
pub(crate) struct AppStateInnerMut {
    pub(crate) command_sender: Sender<NodeCommand>,
    pub(crate) package_sender: Sender<AppPackage>,
    pub(crate) server_addr: Option<SocketAddr>,
    pub(crate) streams: HashMap<SocketAddr, (TcpStream, MetaData)>,
    selected_room: Option<SocketAddr>,
}
pub(crate) struct AppStateInner {
    _r: AppStateInnerRef,
    m: RwLock<AppStateInnerMut>,
}

pub struct AppState(pub(crate) Arc<AppStateInner>);

impl AppState {
    pub fn new(
        command_sender: Sender<NodeCommand>,
        package_sender: Sender<AppPackage>,
    ) -> Self {
        Self(Arc::new(AppStateInner {
            _r: AppStateInnerRef {
            },
            m: RwLock::new(AppStateInnerMut {
                command_sender,
                package_sender,
                server_addr: None,
                streams: HashMap::new(),
                selected_room: None,
            }),
        }))
    }

    pub fn read_lock(&self) -> Result<RwLockReadGuard<'_, AppStateInnerMut>> {
        self.0.m.read().map_err(|e| anyhow!(e.to_string()))
    }

    pub fn write_lock(&self) -> Result<RwLockWriteGuard<'_, AppStateInnerMut>> {
        self.0.m.write().map_err(|e| anyhow!(e.to_string()))
    }

    pub fn get_selected_room(
        lock: &RwLockReadGuard<'_, AppStateInnerMut>,
    ) -> Option<SocketAddr> {
        lock.selected_room
    }

    pub fn set_selected_room(
        lock: &mut RwLockWriteGuard<'_, AppStateInnerMut>,
        room: Option<SocketAddr>,
    ) {
        lock.selected_room = room;
    }
}

impl Clone for AppState {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
