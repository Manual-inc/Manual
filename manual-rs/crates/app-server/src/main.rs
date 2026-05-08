use std::io::{self, BufRead, Write};

use app_server::AppServer;

fn main() {
    let server = AppServer::new();
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
}
