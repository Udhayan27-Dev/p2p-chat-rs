use std::str::FromStr;
use std::{
    collections::HashMap,
    fmt,
    net::{Ipv4Addr, SocketAddrV4},
};

use anyhow::Result;
use clap::Parser;
use futures_lite::StreamExt;
use iroh::PublicKey;
use iroh::{Endpoint, NodeAddr, NodeId, RelayMode, SecretKey};
use iroh_gossip::net::{Event, GOSSIP_ALPN, Gossip, GossipEvent, GossipReceiver};
use iroh_gossip::proto::TopicId;
use serde::{Deserialize, Serialize};

#[derive(Parser, Debug)]
struct Args {
    #[clap(long)]
    no_relay: bool,
    #[clap(short, long)]
    name: Option<String>,
    #[clap(subcommand)]
    command: Command,
}

#[derive(Parser, Debug)]
enum Command {
    Open,
    Join { ticket: String },
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    //parse the cli
    let (topic, peers) = match &args.command {
        Command::Open => {
            let topic = TopicId::from_bytes(rand::random());
            println!("> opening chat rom for topic {topic}");
            (topic, vec![])
        }
        Command::Join { ticket } => {
            let Ticket { topic, peers } = Ticket::from_str(ticket)?;
            println!("> joining chat room for topic {topic}");
            (topic, peers)
        }
    };

    let relay_mode = match args.no_relay {
        true => RelayMode::Disabled,
        false => RelayMode::Default,
    };

    let secret_key = SecretKey::generate(rand::rngs::OsRng);
    println!("> our secret key is {secret_key}");

    let endpoint = Endpoint::builder()
        .secret_key(secret_key)
        .relay_mode(relay_mode)
        .bind_addr_v4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0))
        .bind()
        .await?;
    println!("> our node is {}", endpoint.node_id());

    let gossip = Gossip::builder().spawn(endpoint.clone()).await?;

    let ticket = {
        let me = endpoint.node_addr().await?;
        let peers = peers.iter().cloned().chain([me]).collect();
        Ticket { topic, peers }
    };
    println!("> ticket to join: {ticket}");

    let router = iroh::protocol::Router::builder(endpoint.clone())
        .accept(GOSSIP_ALPN, gossip.clone())
        .spawn()
        .await?;

    let peer_ids: Vec<PublicKey> = peers.iter().map(|p| p.node_id).collect();
    if peers.is_empty() {
        println!("> waiting for peers to join us....")
    } else {
        println!("> trying to connect to {} peers...", peers.len());
        for peer in peers.into_iter() {
            endpoint.add_node_addr(peer)?;
        }
    };

    let (sender, receiver) = gossip.subscribe_and_join(topic, peer_ids).await?.split();
    println!("> connected");

    if let Some(name) = args.name {
        let message = Message::AboutMe {
            node_id: endpoint.node_id(),
            name,
        };
        let encoded_message = message.to_bytes();
        sender.broadcast(encoded_message.into()).await?;
    }

    tokio::spawn(subscribe_loop(receiver));

    let (line_tx, mut line_rx) = tokio::sync::mpsc::channel(1);
    std::thread::spawn(move || input_loop(line_tx));

    println!("> type a message and hit enter to broadcast ...");
    while let Some(text) = line_rx.recv().await {
        let message = Message::Message {
            node_id: endpoint.node_id(),
            text: text.clone(),
        };
        let encoded_message = message.to_bytes();
        sender.broadcast(encoded_message.into()).await?;
        println!("> sent: {text}");
    }
    router.shutdown().await?;

    Ok(())
}

async fn subscribe_loop(mut receiver: GossipReceiver) -> Result<()> {
    let mut names = HashMap::new();
    while let Some(event) = receiver.try_next().await? {
        if let Event::Gossip(GossipEvent::Received(msg)) = event {
            let message = Message::from_bytes(&msg.content)?;
            match message {
                Message::AboutMe { node_id, name } => {
                    names.insert(node_id, name.clone());
                    println!("> {} is now known as {}", node_id.fmt_short(), name);
                }
                Message::Message { node_id, text } => {
                    let name = names
                        .get(&node_id)
                        .map_or_else(|| node_id.fmt_short(), String::to_string);
                    println!("{}: {}", name, text);
                }
            }
        }
    }
    Ok(())
}

fn input_loop(line_tx: tokio::sync::mpsc::Sender<String>) -> Result<()> {
    let mut buffer = String::new();
    let stdin = std::io::stdin();
    loop {
        stdin.read_line(&mut buffer)?;
        line_tx.blocking_send(buffer.clone())?;
        buffer.clear();
    }
}

#[derive(Debug, Serialize, Deserialize)]
enum Message {
    AboutMe { node_id: NodeId, name: String },
    Message { node_id: NodeId, text: String },
}

impl Message {
    pub fn to_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).expect("serde_json::to_vec is infallible")
    }
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        serde_json::from_slice(bytes).map_err(Into::into)
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Ticket {
    topic: TopicId,
    peers: Vec<NodeAddr>,
}

impl Ticket {
    fn from_bytes(bytes: &[u8]) -> Result<Self> {
        serde_json::from_slice(bytes).map_err(Into::into)
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).expect("serde_json::to_vec is infallible ")
    }
}

impl fmt::Display for Ticket {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut text = data_encoding::BASE32HEX_NOPAD.encode(&self.to_bytes()[..]);
        text.make_ascii_lowercase();
        write!(f, "{}", text)
    }
}

impl FromStr for Ticket {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = data_encoding::BASE32HEX_NOPAD.decode(s.to_ascii_uppercase().as_bytes())?;
        Self::from_bytes(&bytes)
    }
}
