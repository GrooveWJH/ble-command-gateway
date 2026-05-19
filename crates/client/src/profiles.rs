use anyhow::Result;
use client::BleSession;
use comfy_table::modifiers::UTF8_ROUND_CORNERS;
use comfy_table::Table;
use protocol::requests::CommandPayload;
use protocol::responses::WifiProfilesResponseData;
use std::fmt;

use crate::cli_text::Lang;

pub(crate) async fn run_wifi_profiles(session: &mut BleSession, lang: &Lang) -> Result<()> {
    println!(">> Requesting saved Wi-Fi profiles...");
    let response =
        crate::interactive::execute_request(session, CommandPayload::WifiProfilesList, 10).await?;
    let data: WifiProfilesResponseData = response.decode_data()?;

    if data.profiles.is_empty() {
        println!("{}", lang.t("profiles_empty"));
        return Ok(());
    }

    print_profiles_table(&data);
    let choices = deletable_profile_choices(&data);
    if choices.is_empty() {
        println!("{}", lang.t("profiles_no_deletable"));
        return Ok(());
    }

    let selected = inquire::MultiSelect::new(lang.t("profiles_select_delete"), choices).prompt()?;
    if selected.is_empty() {
        println!("{}", lang.t("profiles_delete_skipped"));
        return Ok(());
    }

    let response = crate::interactive::execute_request(
        session,
        CommandPayload::WifiProfilesDelete {
            uuids: selected.into_iter().map(|choice| choice.uuid).collect(),
            force: false,
        },
        30,
    )
    .await?;
    println!("{}", response.text);
    Ok(())
}

fn print_profiles_table(data: &WifiProfilesResponseData) {
    let mut table = Table::new();
    table.apply_modifier(UTF8_ROUND_CORNERS);
    table.set_header(vec!["UUID", "SSID", "Active", "Device", "Autoconnect"]);
    for profile in &data.profiles {
        table.add_row(vec![
            profile.uuid.as_str(),
            profile.ssid.as_str(),
            if profile.active { "yes" } else { "no" },
            profile.device.as_deref().unwrap_or("-"),
            if profile.autoconnect { "yes" } else { "no" },
        ]);
    }
    println!("\n{table}");
}

fn deletable_profile_choices(data: &WifiProfilesResponseData) -> Vec<ProfileChoice> {
    data.profiles
        .iter()
        .filter(|profile| !profile.active)
        .map(|profile| ProfileChoice {
            uuid: profile.uuid.clone(),
            label: format!("{} ({})", profile.ssid, profile.uuid),
        })
        .collect()
}

#[derive(Clone)]
struct ProfileChoice {
    uuid: String,
    label: String,
}

impl fmt::Display for ProfileChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.label)
    }
}
