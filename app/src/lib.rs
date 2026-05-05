use manual_core::workspace_descriptor;

pub fn run() -> String {
    let descriptor = workspace_descriptor();

    format!(
        "{} app is ready with {} workspace packages",
        descriptor.name,
        descriptor.packages.len()
    )
}
