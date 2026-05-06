use sandbox::{NetworkMode, SandboxPolicy};
use sandbox_gui::run_native;
use sandbox_registry::{SandboxDefinition, SandboxRegistry};

fn main() -> eframe::Result {
    run_native(sample_registry())
}

fn sample_registry() -> SandboxRegistry {
    let mut registry = SandboxRegistry::new();
    let mut networked_write = SandboxPolicy::workspace_write("/Users/leejs/github/Manual");
    networked_write.network.mode = NetworkMode::Enabled;

    registry
        .insert(
            SandboxDefinition::new(
                "read-only",
                SandboxPolicy::read_only("/Users/leejs/github/Manual"),
            )
            .expect("sample read-only sandbox should be valid"),
        )
        .expect("sample read-only sandbox should be inserted");
    registry
        .insert(
            SandboxDefinition::new("networked-write", networked_write)
                .expect("sample networked sandbox should be valid"),
        )
        .expect("sample networked sandbox should be inserted");
    registry
        .insert(
            SandboxDefinition::new("danger-full-access", SandboxPolicy::danger_full_access())
                .expect("sample danger sandbox should be valid"),
        )
        .expect("sample danger sandbox should be inserted");

    registry
}
