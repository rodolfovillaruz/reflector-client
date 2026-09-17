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

fn run_ssh(config: &Config, forwards: &[String], remote_command: Option<&str>) {
    let ip = fetch_ip(config);

    let mut cmd = Command::new("ssh");
    cmd.arg("-t")
        .arg("-o")
        .arg("StrictHostKeyChecking=no")
        .arg("-o")
        .arg("UserKnownHostsFile=/dev/null");

    for spec in forwards {
        cmd.arg("-L").arg(spec);
    }

    cmd.arg(format!("ubuntu@{ip}"));
    if let Some(remote_command) = remote_command {
        cmd.arg(remote_command);
    }

    let status = cmd.status().unwrap_or_else(|e| {
        eprintln!("failed to run ssh: {e}");
        std::process::exit(1);
    });

    std::process::exit(status.code().unwrap_or(1));
}

/// Validates a `-L` forward spec of the form `[bind_address:]port:host:hostport`
/// and returns it unchanged for handing off to ssh.
fn validate_forward_spec(spec: &str) -> &str {
    if spec.splitn(4, ':').count() < 3 {
        eprintln!("invalid -L argument: {spec}");
        eprintln!(
            "expected format: [bind_address:]port:host:hostport (e.g. -L 5000:127.0.0.1:5000)"
        );
        std::process::exit(2);
    }
    spec
}

/// Validates a tmux session name, restricting it to characters that are safe
/// to interpolate into the remote shell command and that tmux itself allows
/// (tmux uses `:` and `.` as target-spec separators).
fn validate_session_name(name: &str) -> &str {
    let valid = !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if !valid {
        eprintln!("invalid -s argument: {name}");
        eprintln!("session names may only contain letters, digits, '-' and '_'");
        std::process::exit(2);
    }
    name
}

struct Args {
    command: Option<String>,
    forwards: Vec<String>,
    session: Option<String>,
}

fn parse_args(raw: Vec<String>) -> Args {
    let mut command = None;
    let mut forwards = Vec::new();
    let mut session = None;
    let mut iter = raw.into_iter();

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "-L" | "--local-forward" => {
                let spec = iter.next().unwrap_or_else(|| {
                    eprintln!("{arg} requires an argument, e.g. {arg} 5000:127.0.0.1:5000");
                    std::process::exit(2);
                });
                forwards.push(validate_forward_spec(&spec).to_string());
            }
            "-s" | "--session" => {
                let name = iter.next().unwrap_or_else(|| {
                    eprintln!("{arg} requires an argument, e.g. {arg} work");
                    std::process::exit(2);
                });
                session = Some(validate_session_name(&name).to_string());
            }
            other if command.is_none() => command = Some(other.to_string()),
            other => {
                eprintln!("unexpected argument: {other}");
                std::process::exit(2);
            }
        }
    }

    Args {
        command,
        forwards,
        session,
    }
}

fn main() {
    let raw_args: Vec<String> = std::env::args().skip(1).collect();

    if raw_args.iter().any(|a| a == "--version" || a == "-V") {
        println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        return;
    }

    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("failed to install rustls crypto provider");

    let config = load_config();
    let args = parse_args(raw_args);

    match args.command.as_deref() {
        None | Some("connect") => {
            let session = args.session.as_deref().unwrap_or("default");
            run_ssh(
                &config,
                &args.forwards,
                Some(&format!("tmux new -As {session}")),
            )
        }
        Some("ssh") => {
            if args.session.is_some() {
                eprintln!("-s is only supported with the connect command");
                std::process::exit(2);
            }
            run_ssh(&config, &args.forwards, None)
        }
        Some("status") => {
            if !args.forwards.is_empty() {
                eprintln!("-L is only supported with the connect and ssh commands");
                std::process::exit(2);
            }
            if args.session.is_some() {
                eprintln!("-s is only supported with the connect command");
                std::process::exit(2);
            }
            run_status(&config)
        }
        Some(other) => {
            eprintln!("unknown command: {other}");
            eprintln!(
                "usage: reflector [connect|ssh|status] [-L bind_address:port:host:hostport]... [-s session]"
            );
            std::process::exit(2);
        }
    }
}
