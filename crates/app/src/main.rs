fn main() {
    let mut args = std::env::args_os();
    let _binary = args.next();
    let command = args.next();

    match command.as_deref() {
        Some(flag) if flag == "--script-gui" => {
            let repository_root = args.next().unwrap_or_else(|| ".".into());
            if let Err(error) = app::run_script_gui(repository_root) {
                eprintln!("script GUI failed: {error}");
                std::process::exit(1);
            }
        }
        Some(flag) if flag == "--workflow-gui" => {
            if let Err(error) = app::run_workflow_gui() {
                eprintln!("workflow GUI failed: {error}");
                std::process::exit(1);
            }
        }
        Some(flag) if flag == "--job-gui" => {
            if let Err(error) = app::run_job_gui() {
                eprintln!("job GUI failed: {error}");
                std::process::exit(1);
            }
        }
        Some(flag) if flag == "--agent-gui" => {
            if let Err(error) = app::run_agent_gui() {
                eprintln!("agent GUI failed: {error}");
                std::process::exit(1);
            }
        }
        Some(flag) if flag == "--summary" => {
            println!("{}", app::run());
        }
        _ => {
            if let Err(error) = app::run_native() {
                eprintln!("Manual app failed: {error}");
                std::process::exit(1);
            }
        }
    }
}
