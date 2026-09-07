pub use codex_prompts::PermissionsInstructions;

use codex_protocol::models::PermissionProfile;
use codex_protocol::protocol::AskForApproval;
use std::path::Path;

/// Compact permission reminder for lite agent context. The actual profile is
/// still enforced by tools and represented in the environment context.
pub(crate) fn lite_permissions_instructions(
    permission_profile: &PermissionProfile,
    approval_policy: AskForApproval,
    cwd: &Path,
) -> String {
    let filesystem = permission_profile.file_system_sandbox_policy();
    let sandbox = if filesystem.has_full_disk_write_access() {
        "full-access"
    } else if filesystem.get_writable_roots_with_cwd(cwd).is_empty() {
        "read-only"
    } else {
        "workspace-write"
    };
    let network = if permission_profile.network_sandbox_policy().is_enabled() {
        "enabled"
    } else {
        "restricted"
    };
    format!(
        "<permissions instructions>Sandbox={sandbox}; network={network}; approvals={approval_policy:?}. Tool-enforced permissions are authoritative; request escalation only through the provided tool flow.</permissions instructions>"
    )
}
