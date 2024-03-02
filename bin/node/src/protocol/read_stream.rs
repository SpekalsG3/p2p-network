use std::net::{Shutdown, SocketAddr, TcpStream};
use std::time::SystemTime;
use crate::commands::NodeCommand;
use crate::protocol::frames::ProtocolMessage;
use crate::protocol::node_info::NodeInfo;
use crate::types::package::{AlertPackage, AlertPackageLevel, AppPackage, MessagePackage};
use crate::types::state::AppState;
use crate::utils::sss_triangle::sss_triangle;

pub fn protocol_read_stream(
    app_state: AppState,
    addr: SocketAddr,
    mut stream: TcpStream, // should be cloned anyway bc otherwise `&mut` at `stream.read` will block whole application
) {
    loop {
        let message = match ProtocolMessage::from_stream(&mut stream) {
            Ok(m) => m,
            Err(e) => {
                let lock = app_state.write_lock().expect("---Failed to get write lock");
                lock
                    .package_sender
                    .send(AppPackage::Alert(AlertPackage {
                        level: AlertPackageLevel::ERROR,
                        msg: format!("Failed to read stream - {}", e),
                    }))
                    .expect("---Failed to send app package");
                break;
            }
        };

        if message.is_none() {
            break;
        }
        let message = message.unwrap();

        match message {
            ProtocolMessage::Data(data) => {
                let lock = app_state.read_lock().expect("---Failed to get write lock");
                lock
                    .package_sender
                    .send(AppPackage::Message(MessagePackage {
                        from: addr,
                        msg: data,
                    }))
                    .expect("---Failed to send app package");
            }
            ProtocolMessage::NodeInfo(info) => {
                let lock = app_state.read_lock().expect("---Failed to get write lock");
                if lock.streams.len() < 4 { // todo: move as config variable
                    lock
                        .command_sender
                        .send(NodeCommand::ClientConnect {
                            src_addr: info.addr,
                            src_ping: info.ping,
                            targ: addr,
                        })
                        .expect("---Failed to send NodeCommand");
                } else {
                    // todo: if ping is lower then biggest latency we have, then disconnect and connect to that one
                }
            }
            ProtocolMessage::Pong(info) => {
                let mut lock = app_state.write_lock().expect("---Failed to get write lock");

                let ping_info = if let Some(info) = info {
                    let src_ping = lock.streams.get(&info.addr).expect("src_addr should exist").1.ping;
                    Some((src_ping, info.ping))
                } else {
                    None
                };

                let (_, ref mut metadata) = lock
                    .streams
                    .get_mut(&addr)
                    .expect("Unknown address");

                if metadata.ping_started_at.is_none() {
                    continue; // haven't requested ping => cannot measure anything
                }

                let ping = SystemTime::now().duration_since(metadata.ping_started_at.unwrap()).unwrap().as_millis();
                if ping > 60_000 { // todo: move to constant
                    lock
                        .package_sender
                        .send(AppPackage::Alert(AlertPackage {
                            level: AlertPackageLevel::WARNING,
                            msg: format!("Ping with host {} is too big ({}). Disconnecting", addr, ping),
                        }))
                        .expect("---Failed to send app package");
                    stream.shutdown(Shutdown::Both).expect("---Failed to shutdown stream");
                    break;
                }
                let ping = ping as u16;

                if let Some((src_ping, targ_ping)) = ping_info {
                    metadata.topology_rad = sss_triangle(src_ping, ping, targ_ping);
                }

                metadata.ping = ping;
                metadata.ping_started_at = None;
            }
            ProtocolMessage::ConnClosed => {
                let mut lock = app_state.write_lock().expect("---Failed to get write lock");

                let (ref mut stream, _) = lock
                    .streams
                    .get_mut(&addr)
                    .expect("Unknown address");

                stream.shutdown(Shutdown::Both).expect("Failed to shutdown");
                lock.streams.remove(&addr);

                break;
            },
            ProtocolMessage::Ping => {
                let info = {
                    let lock = app_state.read_lock().expect("---Failed to get write lock");
                    let (_, metadata) = lock.streams.get(&addr).expect("entry should exist");
                    if let Some(targ_addr) = metadata.connected_to.get(0) {
                        let (_, metadata) = lock
                            .streams
                            .get(&targ_addr)
                            .expect("we should know about it bc `targ_addr` knows about us bc we connected to him");

                        Some(
                            NodeInfo::new(targ_addr.clone(), metadata.ping)
                        )
                    } else {
                        None
                    }
                };

                ProtocolMessage::Pong(info)
                    .send_to_stream(&mut stream)
                    .expect("Failed to send protocol to stream");
            }
        }
    }
}
