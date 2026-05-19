#[cfg(target_os = "linux")]
use async_trait::async_trait;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterIdentitySnapshot {
    pub system_name: String,
    pub previous_alias: String,
    pub new_alias: String,
    pub changed: bool,
}

pub fn plan_public_identity(
    system_name: impl Into<String>,
    previous_alias: impl Into<String>,
    identity_name: &str,
) -> AdapterIdentitySnapshot {
    let previous_alias = previous_alias.into();
    AdapterIdentitySnapshot {
        system_name: system_name.into(),
        changed: previous_alias != identity_name,
        previous_alias,
        new_alias: identity_name.to_string(),
    }
}

#[cfg(target_os = "linux")]
#[async_trait]
pub trait AdapterIdentity {
    async fn system_name(&self) -> bluer::Result<String>;
    async fn alias(&self) -> bluer::Result<String>;
    async fn set_alias(&self, alias: String) -> bluer::Result<()>;
    fn adapter_name(&self) -> String;
}

#[cfg(target_os = "linux")]
#[async_trait]
impl AdapterIdentity for bluer::Adapter {
    async fn system_name(&self) -> bluer::Result<String> {
        self.system_name().await
    }

    async fn alias(&self) -> bluer::Result<String> {
        self.alias().await
    }

    async fn set_alias(&self, alias: String) -> bluer::Result<()> {
        self.set_alias(alias).await
    }

    fn adapter_name(&self) -> String {
        self.name().to_string()
    }
}

#[cfg(target_os = "linux")]
pub async fn apply_public_identity<A>(
    adapter: &A,
    identity_name: &str,
) -> bluer::Result<AdapterIdentitySnapshot>
where
    A: AdapterIdentity + Sync,
{
    let system_name = adapter.system_name().await?;
    let previous_alias = adapter.alias().await?;
    let snapshot = plan_public_identity(system_name, previous_alias, identity_name);

    if snapshot.changed {
        adapter.set_alias(identity_name.to_string()).await?;
    }

    Ok(snapshot)
}

#[cfg(target_os = "linux")]
pub async fn apply_and_log_public_identity<A>(
    adapter: &A,
    identity_name: &str,
) -> bluer::Result<AdapterIdentitySnapshot>
where
    A: AdapterIdentity + Sync,
{
    let snapshot = apply_public_identity(adapter, identity_name).await?;
    tracing::info!(
        adapter_name = %adapter.adapter_name(),
        identity_name,
        system_name = %snapshot.system_name,
        previous_alias = %snapshot.previous_alias,
        new_alias = %snapshot.new_alias,
        changed = snapshot.changed,
        "ble.adapter.identity_applied"
    );
    crate::log_view::emit_block(&crate::log_view::adapter_identity_block(
        &adapter.adapter_name(),
        &snapshot,
    ));
    Ok(snapshot)
}

#[cfg(test)]
mod tests {
    use super::AdapterIdentitySnapshot;

    #[cfg(target_os = "linux")]
    use super::AdapterIdentity;
    #[cfg(target_os = "linux")]
    use async_trait::async_trait;
    #[cfg(target_os = "linux")]
    use std::sync::{Arc, Mutex};

    #[cfg(target_os = "linux")]
    #[derive(Clone)]
    struct FakeAdapter {
        system_name: String,
        alias: Arc<Mutex<String>>,
        writes: Arc<Mutex<Vec<String>>>,
    }

    #[cfg(target_os = "linux")]
    #[async_trait]
    impl AdapterIdentity for FakeAdapter {
        async fn system_name(&self) -> bluer::Result<String> {
            Ok(self.system_name.clone())
        }

        async fn alias(&self) -> bluer::Result<String> {
            Ok(self.alias.lock().unwrap().clone())
        }

        async fn set_alias(&self, alias: String) -> bluer::Result<()> {
            self.writes.lock().unwrap().push(alias.clone());
            *self.alias.lock().unwrap() = alias;
            Ok(())
        }

        fn adapter_name(&self) -> String {
            "hci0".to_string()
        }
    }

    #[cfg(target_os = "linux")]
    fn fake_adapter(alias: &str) -> FakeAdapter {
        FakeAdapter {
            system_name: "edge-gateway".to_string(),
            alias: Arc::new(Mutex::new(alias.to_string())),
            writes: Arc::new(Mutex::new(Vec::new())),
        }
    }

    #[test]
    fn snapshot_records_changed_alias() {
        let snapshot = AdapterIdentitySnapshot {
            system_name: "edge-gateway".to_string(),
            previous_alias: "edge-gateway".to_string(),
            new_alias: "yundrone-ytcwln".to_string(),
            changed: true,
        };

        assert!(snapshot.changed);
        assert_eq!(snapshot.new_alias, "yundrone-ytcwln");
    }

    #[test]
    fn plans_alias_change_when_current_alias_exposes_system_name() {
        let snapshot =
            super::plan_public_identity("edge-gateway", "edge-gateway", "yundrone-ytcwln");

        assert!(snapshot.changed);
        assert_eq!(snapshot.system_name, "edge-gateway");
        assert_eq!(snapshot.previous_alias, "edge-gateway");
        assert_eq!(snapshot.new_alias, "yundrone-ytcwln");
    }

    #[test]
    fn plans_no_write_when_identity_is_already_applied() {
        let snapshot =
            super::plan_public_identity("edge-gateway", "yundrone-ytcwln", "yundrone-ytcwln");

        assert!(!snapshot.changed);
        assert_eq!(snapshot.previous_alias, "yundrone-ytcwln");
    }

    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn applies_alias_when_current_alias_exposes_system_name() {
        let adapter = fake_adapter("edge-gateway");

        let snapshot = super::apply_public_identity(&adapter, "yundrone-ytcwln")
            .await
            .unwrap();

        assert!(snapshot.changed);
        assert_eq!(snapshot.system_name, "edge-gateway");
        assert_eq!(snapshot.previous_alias, "edge-gateway");
        assert_eq!(snapshot.new_alias, "yundrone-ytcwln");
        assert_eq!(
            adapter.writes.lock().unwrap().as_slice(),
            ["yundrone-ytcwln"]
        );
    }

    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn skips_alias_write_when_identity_is_already_applied() {
        let adapter = fake_adapter("yundrone-ytcwln");

        let snapshot = super::apply_public_identity(&adapter, "yundrone-ytcwln")
            .await
            .unwrap();

        assert!(!snapshot.changed);
        assert!(adapter.writes.lock().unwrap().is_empty());
    }
}
