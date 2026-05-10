use std::io::{self, BufRead, Write};
use std::net::TcpListener;
use std::path::PathBuf;

use app_server::{AppServer, HttpServerConfig, serve_http_listener};

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let server = AppServer::new();

    if let Some(listen) = args.listen {
        let auth_token = args
            .auth_token
            .filter(|token| !token.is_empty())
            .ok_or("--auth-token is required when --listen is used")?;
        let listener = TcpListener::bind(&listen)?;
        let address = listener.local_addr()?;

        if let Some(discovery_file) = args.discovery_file {
            write_discovery_file(&discovery_file, address.to_string(), &auth_token)?;
        }

        return serve_http_listener(listener, server, HttpServerConfig { auth_token })
            .map_err(Into::into);
    }

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(line) => line,
            Err(error) => {
                eprintln!("failed to read JSON-RPC request: {error}");
                break;
            }
        };

        if line.trim().is_empty() {
            continue;
        }

        let response = server.handle_json(&line);
        if writeln!(stdout, "{response}")
            .and_then(|_| stdout.flush())
            .is_err()
        {
            break;
        }
    }

    Ok(())
}

struct Args {
    listen: Option<String>,
    auth_token: Option<String>,
    discovery_file: Option<PathBuf>,
}

impl Args {
    fn parse() -> Self {
        let mut listen = None;
        let mut auth_token = None;
        let mut discovery_file = None;
        let mut args = std::env::args().skip(1);

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--listen" => listen = args.next(),
                "--auth-token" => auth_token = args.next(),
                "--discovery-file" => discovery_file = args.next().map(PathBuf::from),
                "--stdio" => {}
                _ => {}
            }
        }

        Self {
            listen,
            auth_token,
            discovery_file,
        }
    }
}

fn write_discovery_file(
    path: &PathBuf,
    address: String,
    auth_token: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let payload = serde_json::json!({
        "pid": std::process::id(),
        "url": format!("http://{address}"),
        "auth_token": auth_token,
    });
    std::fs::write(path, serde_json::to_vec_pretty(&payload)?)?;
    restrict_discovery_file_permissions(path)?;
    Ok(())
}

#[cfg(unix)]
fn restrict_discovery_file_permissions(path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = std::fs::metadata(path)?.permissions();
    permissions.set_mode(0o600);
    std::fs::set_permissions(path, permissions)?;
    Ok(())
}

#[cfg(not(unix))]
fn restrict_discovery_file_permissions(_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}
