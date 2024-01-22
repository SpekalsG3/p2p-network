mod server;
mod client;
mod types;
mod frontend;

use std::env::args;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::mpsc::channel;
use crate::client::start_client;
use crate::frontend::handle_messages;
use crate::server::start_server;

fn main() {
    let (server_addr, client_addr) = {
        let mut args = args().skip(1);

        let mut server_addr: Option<SocketAddr> = None;
        let mut client_addr: Option<SocketAddr> = None;
        while let Some(arg) = args.next() {
            match arg.as_ref() {
                "-s" => {
                    let addr = args.next().expect("Missing an address of a server to bind this node to");
                    let addr = SocketAddr::from_str(&addr).expect("Invalid address for a server");
                    server_addr = Some(addr)
                },
                "-c" => {
                    let addr = args.next().expect("Missing an address of a server to connect to");
                    let addr = SocketAddr::from_str(&addr).expect("Invalid address for a client");
                    client_addr = Some(addr)
                }
                _ => {
                    panic!("Unknown argument {}", arg);
                }
            }
        }

        (server_addr, client_addr)
    };

    println!("---Init threads");

    let (tx, rx) = channel();
    let mut handles = vec![];

    if let Some(server_addr) = server_addr {
        let tx = tx.clone();
        let handle = std::thread::spawn(move || {
            start_server(tx, server_addr)
        });
        handles.push(handle);
    }
    if let Some(client_addr) = client_addr {
        let tx = tx.clone();
        let handle = std::thread::spawn(move || {
            start_client(tx, client_addr)
        });
        handles.push(handle);
    }
    {
        let handle = std::thread::spawn(move || {
            handle_messages(rx)
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join()
            .map_err(|e| panic!("---Error joining the thread - {:?}", e))
            .expect("---Thread panic'd");
    }
}
