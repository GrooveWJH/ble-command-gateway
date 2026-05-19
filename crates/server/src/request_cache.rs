use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

const REQUEST_CACHE_TTL: Duration = Duration::from_secs(120);

pub(crate) struct RequestCache {
    entries: HashMap<String, CachedRequestState>,
}

struct CachedRequestState {
    command_name: String,
    created_at: Instant,
    final_response: Option<protocol::CommandResponse>,
}

impl RequestCache {
    pub(crate) fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    pub(crate) fn duplicate_response(
        &mut self,
        request_id: &str,
    ) -> Option<protocol::CommandResponse> {
        self.prune_expired();
        let entry = self.entries.get(request_id)?;
        match &entry.final_response {
            Some(response) => Some(response.clone()),
            None => Some(protocol::CommandResponse::accepted(
                request_id.to_string(),
                entry.command_name.clone(),
                "accepted",
                None,
            )),
        }
    }

    pub(crate) fn mark_started(&mut self, request_id: &str, command_name: &str) {
        self.prune_expired();
        self.entries.insert(
            request_id.to_string(),
            CachedRequestState {
                command_name: command_name.to_string(),
                created_at: Instant::now(),
                final_response: None,
            },
        );
    }

    pub(crate) fn mark_final(&mut self, response: &protocol::CommandResponse) {
        if let Some(entry) = self.entries.get_mut(&response.id) {
            entry.final_response = Some(response.clone());
        }
    }

    fn prune_expired(&mut self) {
        let now = Instant::now();
        self.entries
            .retain(|_, entry| now.duration_since(entry.created_at) <= REQUEST_CACHE_TTL);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn duplicate_in_progress_replays_accepted() {
        let mut cache = super::RequestCache::new();
        cache.mark_started("req-1", "wifi.scan");

        let response = cache.duplicate_response("req-1").unwrap();

        assert_eq!(response.id, "req-1");
        assert_eq!(response.cmd.as_deref(), Some("wifi.scan"));
        assert!(!response.final_flag);
    }

    #[test]
    fn duplicate_completed_request_replays_final_response() {
        let mut cache = super::RequestCache::new();
        cache.mark_started("req-2", "system.status");
        let response = protocol::CommandResponse::result(
            "req-2",
            Some("system.status".to_string()),
            true,
            protocol::codes::CODE_OK,
            "ok",
            None,
        );
        cache.mark_final(&response);

        assert_eq!(cache.duplicate_response("req-2"), Some(response));
    }
}
