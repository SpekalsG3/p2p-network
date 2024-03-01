use std::collections::HashMap;
use std::io::Write;
use std::net::{SocketAddr, TcpStream};
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::sync::mpsc::Sender;
use std::time::{Duration, SystemTime};
use anyhow::{anyhow, bail, Context, Result};
use crate::protocol::encode_frame_data::ProtocolFrame;
use crate::types::package::AppPackage;

pub struct MetaData {
    pub(crate) ping: Duration,
    pub(crate) ping_started_at: Option<SystemTime>,
    pub(crate) topology_alpha: f32, // angel relative to the first connection, used to determine who's closer to another user
}

pub(crate) struct AppStateInnerRef {}
pub(crate) struct AppStateInnerMut {
    package_sender: Sender<AppPackage>,
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
        package_sender: Sender<AppPackage>,
    ) -> Self {
        Self(Arc::new(AppStateInner {
            _r: AppStateInnerRef {
            },
            m: RwLock::new(AppStateInnerMut {
                package_sender,
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

    pub fn add_stream(
        lock: &mut RwLockWriteGuard<'_, AppStateInnerMut>,
        addr: SocketAddr,
        stream: TcpStream,
    ) {
        lock.streams.insert(addr, (stream, MetaData {
            ping: Duration::from_secs(0),
            ping_started_at: None,
            topology_alpha: 0_f32,
        }));
    }

    pub fn send_package(
        lock: &mut RwLockWriteGuard<'_, AppStateInnerMut>,
        package: AppPackage,
    ) -> Result<()> {
        lock.package_sender.send(package).context("---Failed to send app message")
    }

    pub fn send_stream_message(
        lock: &mut RwLockWriteGuard<'_, AppStateInnerMut>,
        addr: &SocketAddr,
        frame: ProtocolFrame,
    ) -> Result<()> {
        let (ref mut stream, _) = match lock.streams.get_mut(addr) {
            Some(s) => s,
            None => {
                bail!("No stream for that address");
            }
        };

        for chunk in frame {
            stream.write(&chunk).map_err(|e| anyhow!("---Failed to write to stream: {}", e.to_string()))?;
        }

        Ok(())
    }
}

impl Clone for AppState {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}