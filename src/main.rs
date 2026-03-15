use anyhow::Result;
use iroh::{Endpoint, SecretKey};

#[tokio::main]
async fn main() -> Result<()>{
    let secret_key= SecretKey::generate(rand::rngs::OsRng); 
    println!("> our secret key: {}",secret_key);
    
    let endpoint = Endpoint::builder()
        .secret_key(secret_key)
        .discovery_n0()
        .bind()
        .await?;
    println!("> our node id is: {}",endpoint.node_id());
    
    let gossip = Gossip::new();
    
    let router = Router::builder(endpoint.clone())
        .accept(iroh_gossip::ALPN, gossip.clone())
        .spawn()
        .await?;
    Ok(())
}