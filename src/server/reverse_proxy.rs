use std::{ptr::null, time::Duration};

use reqwest::Client;

pub struct ReverseProxy {
    client: Client,
}

impl ReverseProxy {
    fn new() -> Self {
        let client: Client = Client::builder()
            .timeout(Duration::from_secs(30))
            .pool_max_idle_per_host(100) //커넥션 풀
            .build()
            .unwrap();

        ReverseProxy { client }
    }
}
