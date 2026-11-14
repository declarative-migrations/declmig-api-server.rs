#![forbid(unsafe_code)]

use declmig_api_server::{config::ApiConfig, flags, server};

fn main() {
    let environment = match flags::resolve() {
        Ok(environment) => environment,
        Err(_) => {
            eprintln!("invalid API flags or environment configuration");
            std::process::exit(2);
        }
    };
    let cfg = ApiConfig::from_map(&environment);
    if let Err(error) = cfg.validate() {
        eprintln!("invalid API transport configuration: {error}");
        std::process::exit(2);
    }
    server::run(&cfg);
}
