#[cfg(target_os = "linux")]
use async_trait::async_trait;
#[cfg(target_os = "linux")]
use std::time::Duration;

#[cfg(target_os = "linux")]
const PAIRING_GUARD_INTERVAL: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairingPolicySnapshot {
    pub previous_pairable: bool,
    pub new_pairable: bool,
    pub changed: bool,
}

pub fn plan_pairing_policy(previous_pairable: bool) -> PairingPolicySnapshot {
    PairingPolicySnapshot {
        previous_pairable,
        new_pairable: false,
        changed: previous_pairable,
    }
}

#[cfg(target_os = "linux")]
#[async_trait]
pub trait AdapterPairingPolicy {
    async fn is_pairable(&self) -> bluer::Result<bool>;
    async fn set_pairable(&self, pairable: bool) -> bluer::Result<()>;
    fn adapter_name(&self) -> String;
}

#[cfg(target_os = "linux")]
#[async_trait]
impl AdapterPairingPolicy for bluer::Adapter {
    async fn is_pairable(&self) -> bluer::Result<bool> {
        self.is_pairable().await
    }

    async fn set_pairable(&self, pairable: bool) -> bluer::Result<()> {
        self.set_pairable(pairable).await
    }

    fn adapter_name(&self) -> String {
        self.name().to_string()
    }
}

#[cfg(target_os = "linux")]
pub async fn disable_pairing<A>(adapter: &A) -> bluer::Result<PairingPolicySnapshot>
where
    A: AdapterPairingPolicy + Sync,
{
    let previous_pairable = adapter.is_pairable().await?;
    let snapshot = plan_pairing_policy(previous_pairable);

    if snapshot.changed {
        adapter.set_pairable(false).await?;
    }

    Ok(snapshot)
}

#[cfg(target_os = "linux")]
pub async fn disable_and_log_pairing<A>(adapter: &A) -> bluer::Result<PairingPolicySnapshot>
where
    A: AdapterPairingPolicy + Sync,
{
    let snapshot = disable_pairing(adapter).await?;
    tracing::info!(
        adapter_name = %adapter.adapter_name(),
        previous_pairable = snapshot.previous_pairable,
        new_pairable = snapshot.new_pairable,
        changed = snapshot.changed,
        "ble.adapter.pairing_disabled"
    );
    crate::log_view::emit_block(&crate::log_view::adapter_pairing_block(
        &adapter.adapter_name(),
        &snapshot,
    ));
    Ok(snapshot)
}

#[cfg(target_os = "linux")]
pub fn spawn_pairing_guard(adapter: bluer::Adapter) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(PAIRING_GUARD_INTERVAL);
        loop {
            interval.tick().await;
            match disable_pairing(&adapter).await {
                Ok(snapshot) if snapshot.changed => {
                    tracing::warn!(
                        adapter_name = %adapter.name(),
                        previous_pairable = snapshot.previous_pairable,
                        new_pairable = snapshot.new_pairable,
                        "ble.adapter.pairing_reasserted"
                    );
                    crate::log_view::emit_block(&crate::log_view::adapter_pairing_block(
                        adapter.name(),
                        &snapshot,
                    ));
                }
                Ok(_) => {}
                Err(err) => {
                    tracing::warn!(
                        adapter_name = %adapter.name(),
                        error = %err,
                        "ble.adapter.pairing_guard_failed"
                    );
                }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    #[cfg(target_os = "linux")]
    use super::AdapterPairingPolicy;
    #[cfg(target_os = "linux")]
    use async_trait::async_trait;
    #[cfg(target_os = "linux")]
    use std::sync::{Arc, Mutex};

    #[cfg(target_os = "linux")]
    #[derive(Clone)]
    struct FakeAdapter {
        pairable: Arc<Mutex<bool>>,
        writes: Arc<Mutex<Vec<bool>>>,
    }

    #[cfg(target_os = "linux")]
    #[async_trait]
    impl AdapterPairingPolicy for FakeAdapter {
        async fn is_pairable(&self) -> bluer::Result<bool> {
            Ok(*self.pairable.lock().unwrap())
        }

        async fn set_pairable(&self, pairable: bool) -> bluer::Result<()> {
            self.writes.lock().unwrap().push(pairable);
            *self.pairable.lock().unwrap() = pairable;
            Ok(())
        }

        fn adapter_name(&self) -> String {
            "hci0".to_string()
        }
    }

    #[cfg(target_os = "linux")]
    fn fake_adapter(pairable: bool) -> FakeAdapter {
        FakeAdapter {
            pairable: Arc::new(Mutex::new(pairable)),
            writes: Arc::new(Mutex::new(Vec::new())),
        }
    }

    #[test]
    fn plans_pairing_disable_when_adapter_is_pairable() {
        let snapshot = super::plan_pairing_policy(true);

        assert!(snapshot.changed);
        assert!(snapshot.previous_pairable);
        assert!(!snapshot.new_pairable);
    }

    #[test]
    fn plans_no_write_when_pairing_is_already_disabled() {
        let snapshot = super::plan_pairing_policy(false);

        assert!(!snapshot.changed);
        assert!(!snapshot.previous_pairable);
        assert!(!snapshot.new_pairable);
    }

    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn disables_pairing_when_adapter_is_pairable() {
        let adapter = fake_adapter(true);

        let snapshot = super::disable_pairing(&adapter).await.unwrap();

        assert!(snapshot.changed);
        assert!(!*adapter.pairable.lock().unwrap());
        assert_eq!(adapter.writes.lock().unwrap().as_slice(), [false]);
    }

    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn skips_write_when_pairing_is_already_disabled() {
        let adapter = fake_adapter(false);

        let snapshot = super::disable_pairing(&adapter).await.unwrap();

        assert!(!snapshot.changed);
        assert!(adapter.writes.lock().unwrap().is_empty());
    }

    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn disables_pairing_again_when_external_state_reopens_it() {
        let adapter = fake_adapter(false);

        let first = super::disable_pairing(&adapter).await.unwrap();
        assert!(!first.changed);

        *adapter.pairable.lock().unwrap() = true;
        let second = super::disable_pairing(&adapter).await.unwrap();

        assert!(second.changed);
        assert!(!*adapter.pairable.lock().unwrap());
        assert_eq!(adapter.writes.lock().unwrap().as_slice(), [false]);
    }
}
