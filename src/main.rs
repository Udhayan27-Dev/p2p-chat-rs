use anyhow::Result;
use iroh::{Endpoint, SecretKey};

#[tokio::main]
async fn main() -> Result<()>{
    let secret_key: SecretKey = SecretKey::generate{rand::rng::OsRng}; 
    println!("> our secret key: {secret_key}");
    
    let endpoint = Endpoint::builder().discovery_n0().bind().await?;
    
}