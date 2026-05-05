pub const WORKSPACE_NAME: &str = "manual";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkspaceDescriptor {
    pub name: &'static str,
    pub packages: &'static [&'static str],
}

pub fn workspace_descriptor() -> WorkspaceDescriptor {
    WorkspaceDescriptor {
        name: WORKSPACE_NAME,
        packages: &["core", "app", "cli", "skill"],
    }
}

#[cfg(test)]
mod tests {
    use super::workspace_descriptor;

    #[test]
    fn descriptor_lists_expected_packages() {
        let descriptor = workspace_descriptor();

        assert_eq!(descriptor.name, "manual");
        assert_eq!(descriptor.packages, ["core", "app", "cli", "skill"]);
    }
}
