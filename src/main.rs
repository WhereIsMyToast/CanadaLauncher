#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use self_update::cargo_crate_version;

slint::include_modules!();

#[cfg(feature = "archive-zip")]
use zip::result::ZipError;

const OWNER: &str = "WhereIsMyToast";
const REPO_NAME: &str = "CanadaLauncher";

fn update_exe() -> Result<(), Box<dyn (::std::error::Error)>> {
    let status = self_update::backends::github::Update::configure()
        .repo_owner(OWNER)
        .repo_name(REPO_NAME)
        .bin_name("CanadaLauncher")
        .show_download_progress(true)
        .no_confirm(true)
        .current_version(cargo_crate_version!())
        .build()?
        .update()?;
    println!("Update status: `{}`!", status.version());
    Ok(())
}

fn main() -> Result<(), slint::PlatformError> {
    let _ = update_exe();
    let ui = AppWindow::new()?;

    ui.set_version(cargo_crate_version!().to_string().into());
    ui.run()?;
    Ok(())
}
