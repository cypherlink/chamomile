use simplelog::{CombinedLogger, Config as LogConfig, LevelFilter, TermLogger, TerminalMode};
use std::env::args;
use std::net::SocketAddr;

use chamomile::prelude::{start, Broadcast, Config, ReceiveMessage, SendMessage};
use smol::Timer;
use std::time::Duration;

fn main() {
    CombinedLogger::init(vec![TermLogger::new(
        LevelFilter::Debug,
        LogConfig::default(),
        TerminalMode::Mixed,
    )])
    .unwrap();

    smol::block_on(async {
        let addr_str = args().nth(1).expect("missing path");
        let self_addr: SocketAddr = addr_str.parse().expect("invalid addr");

        println!("START A PERMISSIONLESS PEER. socket: {}", self_addr);

        let mut config = Config::default(self_addr);
        config.permission = false; // Permissionless.
        config.only_stable_data = false; // Receive all peer's data.
        config.db_dir = std::path::PathBuf::from(addr_str);

        let (peer_id, send, recv) = start(config).await.unwrap();
        println!("peer id: {}", peer_id.to_hex());

        if args().nth(2).is_some() {
            let remote_addr: SocketAddr = args().nth(2).unwrap().parse().expect("invalid addr");
            println!("start DHT connect to remote: {}", remote_addr);
            send.send(SendMessage::Connect(remote_addr))
                .await
                .expect("channel failure");

            println!("sleep 3s and then broadcast...");
            Timer::after(Duration::from_secs(3)).await;

            fn mod_reduce(mut i: u32) -> u8 {
                loop {
                    if i > 255 {
                        i = i - 255
                    } else {
                        break;
                    }
                }
                i as u8
            }

            let mut bytes = vec![];
            for i in 0..10u32 {
                bytes.push(mod_reduce(i));
            }

            println!("Will send bytes: {}-{:?}", bytes.len(), &bytes);
            send.send(SendMessage::Broadcast(Broadcast::Gossip, bytes))
                .await
                .expect("channel failure");
        }

        while let Ok(message) = recv.recv().await {
            match message {
                ReceiveMessage::Data(peer_id, bytes) => {
                    println!(
                        "Recv permissionless data from: {}, {}-{:?}",
                        peer_id.short_show(),
                        bytes.len(),
                        bytes
                    );
                }
                ReceiveMessage::Stream(..) => {
                    panic!("Nerver here (stream)");
                }
                ReceiveMessage::StableConnect(from, data) => {
                    println!("Recv peer what to build a stable connected: {:?}", data);

                    let tid = 2u64;

                    send.send(SendMessage::StableResult(
                        tid,
                        from,
                        true,
                        false,
                        vec![3, 3, 3, 3],
                    ))
                    .await
                    .expect("channel failure");
                }
                ReceiveMessage::StableLeave(peer_id) => {
                    println!("Recv stable connected leave: {}", peer_id.to_hex());
                }
                ReceiveMessage::StableResult(peer_id, is_ok, remark) => {
                    println!(
                        "Recv stable connected result: {} {} {:?}",
                        peer_id.to_hex(),
                        is_ok,
                        remark
                    );
                }
                ReceiveMessage::ResultConnect(from, _data) => {
                    println!("Recv Result Connect {}", from.to_hex());
                }
                ReceiveMessage::Delivery(t, tid, had, _data) => {
                    println!("Recv {:?} Delivery: {} {}", t, tid, had);
                }
                ReceiveMessage::NetworkLost => {
                    println!("No peers conneced.")
                }
            }
        }
    });
}
