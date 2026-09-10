//! Shared, revisioned live observations. HTTP clients cannot write this store.
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};

pub const PROVIDERS: [&str; 4] = ["codex", "claude", "copilot", "antigravity"];

#[derive(Clone)]
pub struct LiveDashboard(Arc<Mutex<(u64, Vec<Value>)>>);

impl Default for LiveDashboard {
    fn default() -> Self {
        Self(Arc::new(Mutex::new((1, PROVIDERS.into_iter().map(|provider| json!({
            "provider":provider,"provider_account":null,"setup":"required","paused":false,
            "availability":"unknown",
            "collection_state":"idle","freshness":"unknown","failure_code":null,
            "data_quality":"unknown","collector_maturity":"experimental",
            "quota_windows":[],"source_usage":[],"observed_at":null,"collected_at":null
        })).collect()))))
    }
}

impl LiveDashboard {
    pub fn snapshot(&self) -> (u64, Vec<Value>) {
        self.0.lock().unwrap().clone()
    }

    /// Accept only dashboard fields, never entire raw collector responses.
    pub fn publish(&self, provider: &str, view: &Value) -> Result<(), &'static str> {
        let index = PROVIDERS
            .iter()
            .position(|id| *id == provider)
            .ok_or("unknown_provider")?;
        let mut safe = json!({"provider":provider,"provider_account":null,"paused":false,"collector_maturity":"experimental"});
        for key in [
            "setup",
            "availability",
            "collection_state",
            "freshness",
            "failure_code",
            "data_quality",
            "collected_at",
            "checked_at",
        ] {
            safe[key] = view.get(key).cloned().unwrap_or(Value::Null);
        }
        safe["quota_windows"] = json!(
            view["quota_windows"]
                .as_array()
                .map(|items| items
                    .iter()
                    .map(|item| {
                        let mut window = json!({});
                        for key in [
                            "bucket_key",
                            "label",
                            "remaining_percent",
                            "used_percent",
                            "unit",
                            "resets_at",
                            "reset_display",
                            "entitlement",
                            "used",
                        ] {
                            window[key] = item.get(key).cloned().unwrap_or(Value::Null);
                        }
                        window
                    })
                    .collect::<Vec<_>>())
                .unwrap_or_default()
        );
        safe["source_usage"] = json!(view["source_usage"].as_array().map(|items| items.iter().map(|item| json!({
            "source":"native-windows","used":item["used"],"unit":item["unit"],"model":item["model"],"label":item["label"]
        })).collect::<Vec<_>>()).unwrap_or_default());
        let mut state = self.0.lock().unwrap();
        if state.1[index] != safe {
            state.1[index] = safe;
            state.0 += 1;
        }
        Ok(())
    }

    pub fn failure(&self, provider: &str) {
        let Some(index) = PROVIDERS.iter().position(|id| *id == provider) else {
            return;
        };
        let mut state = self.0.lock().unwrap();
        state.1[index]["collection_state"] = json!("error");
        state.1[index]["freshness"] = json!("stale");
        state.1[index]["failure_code"] = json!("refresh_failed");
        state.0 += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn empty_sources_never_have_mock_quota_and_updates_are_complete_and_deduplicated() {
        let store = LiveDashboard::default();
        let initial = store.snapshot();
        assert_eq!(initial.1.len(), 4);
        assert!(initial.1.iter().all(|p| p["quota_windows"] == json!([])));
        let value = json!({"quota_windows":[{"remaining_percent":42,"unit":"percent","secret":"PRIVATE"}],"token":"PRIVATE","path":"PRIVATE"});
        store.publish("codex", &value).unwrap();
        let changed = store.snapshot();
        assert!(changed.0 > initial.0);
        assert_eq!(changed.1[0]["quota_windows"][0]["remaining_percent"], 42);
        assert!(
            !serde_json::to_string(&changed.1)
                .unwrap()
                .contains("PRIVATE")
        );
        store.publish("codex", &value).unwrap();
        assert_eq!(store.snapshot().0, changed.0);
        store.failure("codex");
        assert_eq!(
            store.snapshot().1[0]["quota_windows"][0]["remaining_percent"],
            42
        );
        assert_eq!(store.snapshot().1[0]["freshness"], "stale");
    }
}
