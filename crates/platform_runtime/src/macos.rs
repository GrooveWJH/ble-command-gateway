use anyhow::Result;
#[cfg(any(target_os = "macos", test))]
use anyhow::{anyhow, Context};
#[cfg(target_os = "macos")]
use std::ffi::OsStr;
#[cfg(target_os = "macos")]
use std::path::{Path, PathBuf};
#[cfg(target_os = "macos")]
use std::process::Command;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeMode {
    Gui,
    Cli,
}

#[derive(Clone, Copy, Debug)]
pub struct AppRuntime {
    pub bundle_name: &'static str,
    pub executable_name: &'static str,
    pub info_plist: &'static [u8],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuntimeLaunchOutcome {
    Continue,
    Relaunched,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RelaunchCommand {
    pub program: String,
    pub args: Vec<String>,
}

pub fn prepare_gui_runtime(app: &AppRuntime) -> Result<RuntimeLaunchOutcome> {
    prepare_runtime(app, RuntimeMode::Gui, &[])
}

pub fn prepare_cli_runtime(app: &AppRuntime, args: &[String]) -> Result<RuntimeLaunchOutcome> {
    prepare_runtime(app, RuntimeMode::Cli, args)
}

fn prepare_runtime(
    app: &AppRuntime,
    mode: RuntimeMode,
    args: &[String],
) -> Result<RuntimeLaunchOutcome> {
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (app, mode, args);
        Ok(RuntimeLaunchOutcome::Continue)
    }

    #[cfg(target_os = "macos")]
    {
        let current_exe = std::env::current_exe().context("resolve current executable")?;
        if is_running_inside_app_bundle(&current_exe) {
            return Ok(RuntimeLaunchOutcome::Continue);
        }

        let bundle_root = app_bundle_root(&current_exe, app.bundle_name)?;
        stage_app_bundle(app, &bundle_root, &current_exe)?;
        sign_app_bundle(&bundle_root)?;
        run_relaunch_command(build_relaunch_command(
            mode,
            &bundle_root.display().to_string(),
            args,
        ))?;

        tracing::info!(
            executable = %current_exe.display(),
            bundle_root = %bundle_root.display(),
            runtime_mode = ?mode,
            "platform_runtime.relaunched"
        );

        Ok(RuntimeLaunchOutcome::Relaunched)
    }
}

#[cfg(any(target_os = "macos", test))]
fn build_relaunch_command(
    mode: RuntimeMode,
    bundle_root: &str,
    args: &[String],
) -> RelaunchCommand {
    let mut open_args = vec!["-n".to_string()];
    if mode == RuntimeMode::Cli {
        open_args.push("-W".to_string());
    }
    open_args.push(bundle_root.to_string());
    if mode == RuntimeMode::Cli && !args.is_empty() {
        open_args.push("--args".to_string());
        open_args.extend(args.iter().cloned());
    }

    RelaunchCommand {
        program: "/usr/bin/open".to_string(),
        args: open_args,
    }
}

#[cfg(target_os = "macos")]
fn is_running_inside_app_bundle(executable: &Path) -> bool {
    executable
        .ancestors()
        .any(|path| path.extension() == Some(OsStr::new("app")))
}

#[cfg(target_os = "macos")]
fn app_bundle_root(executable: &Path, bundle_name: &str) -> Result<PathBuf> {
    let parent = executable
        .parent()
        .ok_or_else(|| anyhow!("missing parent directory for {}", executable.display()))?;
    Ok(parent.join(bundle_name))
}

#[cfg(target_os = "macos")]
fn stage_app_bundle(app: &AppRuntime, bundle_root: &Path, executable: &Path) -> Result<()> {
    let executable_path = bundle_root.join("Contents/MacOS").join(app.executable_name);
    let info_plist_path = bundle_root.join("Contents/Info.plist");

    std::fs::create_dir_all(
        executable_path
            .parent()
            .ok_or_else(|| anyhow!("missing bundle executable parent"))?,
    )
    .with_context(|| format!("create {}", bundle_root.display()))?;
    std::fs::copy(executable, &executable_path).with_context(|| {
        format!(
            "copy executable {} -> {}",
            executable.display(),
            executable_path.display()
        )
    })?;
    std::fs::write(&info_plist_path, app.info_plist)
        .with_context(|| format!("write {}", info_plist_path.display()))?;

    Ok(())
}

#[cfg(target_os = "macos")]
fn sign_app_bundle(bundle_root: &Path) -> Result<()> {
    let status = Command::new("/usr/bin/codesign")
        .args(["--force", "--deep", "--sign", "-"])
        .arg(bundle_root)
        .status()
        .context("run codesign")?;

    if status.success() {
        Ok(())
    } else {
        Err(anyhow!("codesign failed for {}", bundle_root.display()))
    }
}

#[cfg(target_os = "macos")]
fn run_relaunch_command(command: RelaunchCommand) -> Result<()> {
    let status = Command::new(&command.program)
        .args(&command.args)
        .status()
        .with_context(|| format!("launch {}", command.program))?;

    if status.success() {
        Ok(())
    } else {
        Err(anyhow!(
            "{} failed with args {:?}",
            command.program,
            command.args
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::{build_relaunch_command, RelaunchCommand, RuntimeMode};

    #[cfg(target_os = "macos")]
    use super::{app_bundle_root, is_running_inside_app_bundle};
    #[cfg(target_os = "macos")]
    use std::path::Path;

    #[test]
    fn gui_relaunch_uses_open_without_cli_args() {
        let command =
            build_relaunch_command(RuntimeMode::Gui, "/tmp/YunDrone BLE Gateway.app", &[]);

        assert_eq!(
            command,
            RelaunchCommand {
                program: "/usr/bin/open".to_string(),
                args: vec![
                    "-n".to_string(),
                    "/tmp/YunDrone BLE Gateway.app".to_string()
                ],
            }
        );
    }

    #[test]
    fn cli_relaunch_passes_through_original_args() {
        let command = build_relaunch_command(
            RuntimeMode::Cli,
            "/tmp/YunDrone BLE Client.app",
            &["--target".to_string(), "Yundrone_UAV".to_string()],
        );

        assert_eq!(
            command,
            RelaunchCommand {
                program: "/usr/bin/open".to_string(),
                args: vec![
                    "-n".to_string(),
                    "-W".to_string(),
                    "/tmp/YunDrone BLE Client.app".to_string(),
                    "--args".to_string(),
                    "--target".to_string(),
                    "Yundrone_UAV".to_string(),
                ],
            }
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn detects_paths_inside_app_bundles() {
        let executable = Path::new("/tmp/gui.app/Contents/MacOS/gui");
        assert!(is_running_inside_app_bundle(executable));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn derives_bundle_root_next_to_debug_executable() {
        let executable = Path::new("/tmp/target/debug/gui");
        let bundle_root = app_bundle_root(executable, "YunDrone BLE Gateway.app").unwrap();

        assert_eq!(
            bundle_root,
            Path::new("/tmp/target/debug/YunDrone BLE Gateway.app")
        );
    }
}
