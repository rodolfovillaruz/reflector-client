use std::path::PathBuf;
use std::process::Command;

use serde::Deserialize;

#[derive(Deserialize)]
struct Config {
    url: String,
    auth_token: String,
}

#[derive(Deserialize)]
struct ReflectorResponse {
    ip: Option<String>,
    error: Option<String>,
}

fn config_path() -> PathBuf {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .expect("HOME or USERPROFILE environment variable is not set");
    PathBuf::from(home).join(".config").join("reflector.json")
}

fn load_config() -> Config {
    let path = config_path();
    let contents = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        eprintln!("failed to read config file {}: {e}", path.display());
        std::process::exit(1);
    });
    serde_json::from_str(&contents).unwrap_or_else(|e| {
        eprintln!("failed to parse config file {}: {e}", path.display());
        std::process::exit(1);
    })
}

fn fetch_ip(config: &Config) -> String {
    let client = reqwest::blocking::Client::new();
    let res = client
        .get(&config.url)
        .header("X-Auth-Token", &config.auth_token)
        .send()
        .unwrap_or_else(|e| {
            eprintln!("request to {} failed: {e}", config.url);
            std::process::exit(1);
        });

    let status = res.status();
    let body: ReflectorResponse = res.json().unwrap_or_else(|e| {
        eprintln!("failed to parse response from {}: {e}", config.url);
        std::process::exit(1);
    });

    if !status.is_success() {
        let msg = body.error.unwrap_or_else(|| status.to_string());
        eprintln!("reflector returned an error: {msg}");
        std::process::exit(1);
    }

    body.ip.unwrap_or_else(|| {
        eprintln!("reflector response did not include an ip");
        std::process::exit(1);
    })
}

fn main() {
    let config = load_config();
    let ip = fetch_ip(&config);

    let status = Command::new("ssh")
        .arg("-t")
        .arg("-o")
        .arg("StrictHostKeyChecking=no")
        .arg("-o")
        .arg("UserKnownHostsFile=/dev/null")
        .arg(format!("ubuntu@{ip}"))
        .arg("tmux new -As default")
        .status()
        .unwrap_or_else(|e| {
            eprintln!("failed to run ssh: {e}");
            std::process::exit(1);
        });

    std::process::exit(status.code().unwrap_or(1));
}
