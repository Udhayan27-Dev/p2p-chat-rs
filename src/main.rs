use iroh::{Endpoint, key::SecretKey, protocol::Router};
use iroh_gossip::net::Gossip;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let secret_key = SecretKey::generate();
    let pubic_key = secret_key.public();
    println!("P2P Chat ...");
    println!("Node ID: {}",pubic_key);
    
    
    let endpoint = Endpoint::builder()
        .secret_key(secret_key)
        .discovery_n0()
        .bind()
        .await?;
    
    
    
    let gossip = Gossip::spawn(endpoint.clone()).await?;
    
    let router = Router::builder(endpoint.clone())
        .accept(iroh_gossip::ALPN,gossip.clone())
        .spawn()
        .await?;
    
    println!("Network Layer Ready");
    
    tokio::signal::ctrl_c().await?;
    router.shutdown().await?;    
    
    Ok(())
}