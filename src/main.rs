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

#[derive(Deserialize)]
struct StatusResponse {
    state: Option<String>,
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

fn build_client() -> reqwest::blocking::Client {
    // reqwest 0.13's default rustls verifier is rustls-platform-verifier, which
    // panics on Android/Termux because it needs a JNI Context we don't have.
    // Verify against the bundled webpki roots instead.
    let roots = rustls::RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let tls = rustls::ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();

    reqwest::blocking::Client::builder()
        .tls_backend_preconfigured(tls)
        .build()
        .unwrap_or_else(|e| {
            eprintln!("failed to build HTTP client: {e}");
            std::process::exit(1);
        })
}

fn with_action(base: &str, action: &str) -> String {
    let sep = if base.contains('?') { '&' } else { '?' };
    format!("{base}{sep}action={action}")
}

fn fetch_ip(config: &Config) -> String {
    let client = build_client();
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

fn run_status(config: &Config) {
    let client = build_client();
    let url = with_action(&config.url, "status");
    let res = client
        .get(&url)
        .header("X-Auth-Token", &config.auth_token)
        .send()
        .unwrap_or_else(|e| {
            eprintln!("request to {url} failed: {e}");
            std::process::exit(1);
        });

    let status = res.status();
    let body: StatusResponse = res.json().unwrap_or_else(|e| {
        eprintln!("failed to parse response from {url}: {e}");
        std::process::exit(1);
    });

    if !status.is_success() {
        let msg = body.error.unwrap_or_else(|| status.to_string());
        eprintln!("reflector returned an error: {msg}");
        std::process::exit(1);
    }

    println!("state: {}", body.state.as_deref().unwrap_or("unknown"));
    println!("ip: {}", body.ip.as_deref().unwrap_or("-"));
}

fn run_connect(config: &Config) {
    let ip = fetch_ip(config);

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

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.iter().any(|a| a == "--version" || a == "-V") {
        println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        return;
    }

    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("failed to install rustls crypto provider");

    let config = load_config();

    match args.first().map(String::as_str) {
        None | Some("connect") => run_connect(&config),
        Some("status") => run_status(&config),
        Some(other) => {
            eprintln!("unknown command: {other}");
            eprintln!("usage: reflector-client [connect|status]");
            std::process::exit(2);
        }
    }
}
