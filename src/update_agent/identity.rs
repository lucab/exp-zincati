use crate::config::IdentityInput;
use failure::{Fallible, ResultExt};
use uuid::Uuid;

/// Default group for reboot management.
static DEFAULT_GROUP: &str = "default";

#[derive(Clone, Debug, Serialize)]
pub(crate) struct Identity {
    pub(crate) arch: String,
    pub(crate) current_version: String,
    pub(crate) group: String,
    pub(crate) node_uuid: Uuid,
    pub(crate) platform: String,
    pub(crate) stream: String,
    /// Throttle level, 0 (never) to 1000 (unlimited).
    pub(crate) throttle_permille: Option<u16>,
}

impl Identity {
    pub(crate) fn try_from_config(cfg: IdentityInput) -> Fallible<Self> {
        let group = if cfg.group.is_empty() {
            String::from(DEFAULT_GROUP)
        } else {
            cfg.group
        };

        let node_uuid = if cfg.node_uuid.is_empty() {
            compute_node_uuid()?
        } else {
            Uuid::parse_str(&cfg.node_uuid).context("failed to parse uuid")?
        };

        // TODO(lucab): populate these.
        let arch = String::from("amd64");
        let stream = String::from("stable");
        let platform = String::from("metal-bios");
        let throttle_permille = if cfg.throttle_permille.is_empty() {
            None
        } else {
            Some(cfg.throttle_permille.parse()?)
        };

        let current_version = read_os_release().context("failed to get current os-release")?;
        let identity = Self {
            arch,
            stream,
            platform,
            current_version,
            group,
            node_uuid,
            throttle_permille,
        };
        Ok(identity)
    }
}

fn read_os_release() -> Fallible<String> {
    // TODO(lucab): read os-release.
    let ver = "FCOS-01".to_string();
    Ok(ver)
}

fn compute_node_uuid() -> Fallible<Uuid> {
    // TODO(lucab): hash machine-id.
    let node_uuid = Uuid::from_u128(0);
    Ok(node_uuid)
}
