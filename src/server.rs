#![forbid(unsafe_code)]

use crate::config::ApiConfig;
use crate::routes;

pub fn run(config: &ApiConfig) {
    println!("api bind {}", config.bind);
    match serde_json::to_string(&routes::health::body()) {
        Ok(body) => println!("{body}"),
        Err(_) => {
            eprintln!("cannot serialize API health payload");
            std::process::exit(1);
        }
    }
}

