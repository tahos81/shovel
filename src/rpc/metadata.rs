use std::env;

use reqwest::Client;

pub async fn get_ipfs() -> String {
    let username = env::var("IPFS_USERNAME").unwrap();
    let password = env::var("IPFS_PASSWORD").unwrap();
    let url = "https://ipfs.infura.io:5001/api/v0/get?arg=bafybeieu7pkxjxtyn27hzocjfwmicrmax6ig3kgacprccqkm6ocpgx6wcu/1.json";
    let client = Client::new();
    let req = client.post(url).basic_auth(username, Some(password));
    req.send().await.unwrap().text().await.unwrap()
}
