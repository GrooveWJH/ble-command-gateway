use super::{command_runner::run_command_with_timeout, map_run_output, SystemExecResult};

#[derive(Debug, Clone, PartialEq)]
pub(super) struct WifiProfileDeletionPlan {
    pub to_delete: Vec<protocol::responses::WifiProfile>,
    pub skipped: Vec<protocol::responses::WifiProfileSkippedItem>,
    pub failed: Vec<protocol::responses::WifiProfileFailedItem>,
}

pub(super) async fn run_wifi_profiles_list() -> SystemExecResult {
    let connections = map_run_output(
        run_command_with_timeout(
            vec![
                "nmcli",
                "-t",
                "-f",
                "NAME,UUID,TYPE,AUTOCONNECT",
                "connection",
                "show",
            ],
            5.0,
        )
        .await,
        5.0,
    );
    if !connections.ok {
        return connections;
    }

    let active = map_run_output(
        run_command_with_timeout(
            vec![
                "nmcli",
                "-t",
                "-f",
                "NAME,UUID,DEVICE",
                "connection",
                "show",
                "--active",
            ],
            5.0,
        )
        .await,
        5.0,
    );
    if !active.ok {
        return active;
    }

    let profiles = parse_nmcli_wifi_profiles(&connections.text, &active.text);
    wifi_profiles_response(profiles)
}

pub(super) async fn run_wifi_profiles_delete(uuids: &[String], force: bool) -> SystemExecResult {
    if uuids.is_empty() {
        return SystemExecResult::error(protocol::codes::CODE_BAD_REQUEST, "uuids cannot be empty");
    }

    let list = run_wifi_profiles_list().await;
    if !list.ok {
        return list;
    }
    let data: protocol::responses::WifiProfilesResponseData = match list
        .data
        .as_ref()
        .and_then(|map| protocol::responses::from_map(map).ok())
    {
        Some(data) => data,
        None => {
            return SystemExecResult::error(
                protocol::codes::CODE_INTERNAL_ERROR,
                "wifi profile list response was not readable",
            )
        }
    };

    let mut plan = plan_wifi_profile_deletions(&data.profiles, uuids, force);
    let mut deleted = Vec::new();
    for profile in &plan.to_delete {
        let result = map_run_output(
            run_command_with_timeout(
                vec![
                    "nmcli",
                    "connection",
                    "delete",
                    "uuid",
                    profile.uuid.as_str(),
                ],
                8.0,
            )
            .await,
            8.0,
        );
        if result.ok {
            deleted.push(protocol::responses::WifiProfileDeleteItem {
                uuid: profile.uuid.clone(),
                name: profile.name.clone(),
                ssid: profile.ssid.clone(),
            });
        } else {
            plan.failed
                .push(protocol::responses::WifiProfileFailedItem {
                    uuid: profile.uuid.clone(),
                    name: profile.name.clone(),
                    ssid: profile.ssid.clone(),
                    error: result.text,
                });
        }
    }

    finalize_profile_delete(deleted, plan.skipped, plan.failed)
}

pub(super) fn parse_nmcli_wifi_profiles(
    connections_output: &str,
    active_output: &str,
) -> Vec<protocol::responses::WifiProfile> {
    let active = active_output
        .lines()
        .filter_map(|line| {
            let parts = super::network::split_nmcli_fields(line);
            if parts.len() < 3 {
                return None;
            }
            Some((parts[1].clone(), parts[2].clone()))
        })
        .collect::<std::collections::HashMap<_, _>>();

    let mut profiles = connections_output
        .lines()
        .filter_map(|line| {
            let parts = super::network::split_nmcli_fields(line);
            if parts.len() < 4 || parts[2] != "802-11-wireless" {
                return None;
            }
            let uuid = parts[1].clone();
            let device = active.get(&uuid).cloned();
            Some(protocol::responses::WifiProfile {
                name: parts[0].clone(),
                ssid: parts[0].clone(),
                uuid,
                active: device.is_some(),
                device,
                autoconnect: matches!(parts[3].as_str(), "yes" | "true"),
            })
        })
        .collect::<Vec<_>>();

    profiles.sort_by(|left, right| {
        right
            .active
            .cmp(&left.active)
            .then_with(|| left.ssid.cmp(&right.ssid))
    });
    profiles
}

pub(super) fn plan_wifi_profile_deletions(
    profiles: &[protocol::responses::WifiProfile],
    uuids: &[String],
    force: bool,
) -> WifiProfileDeletionPlan {
    let mut to_delete = Vec::new();
    let mut skipped = Vec::new();
    let mut failed = Vec::new();

    for uuid in uuids {
        let Some(profile) = profiles.iter().find(|profile| &profile.uuid == uuid) else {
            failed.push(protocol::responses::WifiProfileFailedItem {
                uuid: uuid.clone(),
                name: String::new(),
                ssid: String::new(),
                error: "not found".to_string(),
            });
            continue;
        };

        if profile.active && !force {
            skipped.push(protocol::responses::WifiProfileSkippedItem {
                uuid: profile.uuid.clone(),
                name: profile.name.clone(),
                ssid: profile.ssid.clone(),
                reason: "active_profile".to_string(),
            });
        } else {
            to_delete.push(profile.clone());
        }
    }

    WifiProfileDeletionPlan {
        to_delete,
        skipped,
        failed,
    }
}

fn wifi_profiles_response(profiles: Vec<protocol::responses::WifiProfile>) -> SystemExecResult {
    let data = protocol::responses::WifiProfilesResponseData { profiles };
    SystemExecResult::ok(
        "wifi profiles listed",
        Some(protocol::responses::to_map(&data).expect("wifi profiles response serializes")),
    )
}

fn finalize_profile_delete(
    deleted: Vec<protocol::responses::WifiProfileDeleteItem>,
    skipped: Vec<protocol::responses::WifiProfileSkippedItem>,
    failed: Vec<protocol::responses::WifiProfileFailedItem>,
) -> SystemExecResult {
    let code = if failed.is_empty() && skipped.is_empty() {
        protocol::codes::CODE_OK
    } else if deleted.is_empty() && failed.is_empty() && !skipped.is_empty() {
        protocol::codes::CODE_PROTECTED_PROFILE
    } else {
        protocol::codes::CODE_PARTIAL_SUCCESS
    };
    let ok = failed.is_empty();
    let text = format!(
        "wifi profiles delete finished: deleted={}, skipped={}, failed={}",
        deleted.len(),
        skipped.len(),
        failed.len()
    );
    let data = protocol::responses::WifiProfilesDeleteResponseData {
        deleted,
        skipped,
        failed,
    };
    SystemExecResult::with_code(
        ok,
        code,
        text,
        Some(protocol::responses::to_map(&data).expect("wifi profile delete response serializes")),
    )
}
