#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use self_update::cargo_crate_version;
use std::{self, io};
use std::{fs::File, io::copy, sync::Arc};
slint::include_modules!();

use slint::ComponentHandle;
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
    let ui = Arc::new(AppWindow::new()?);
    ui.set_version(cargo_crate_version!().to_string().into());

    let runtime = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");

    ui.on_download_pressed(move || {
        let runtime_handle = runtime.handle().clone();
        runtime_handle.spawn(async move {
            if let Err(e) = Downloader::download_file(
                "https://drive.usercontent.google.com/download?id=1CleGhRN9c5jDhhfcQWIY3Co3eL4nMHHJ&export=download&confirm=t&uuid=1fbc1ac8-aec6-46da-b1d8-f43e78e500ac",
                "C:/mod_pack.zip",
            )
            .await
            {
                eprintln!("Error downloading mod pack: {}", e);
            }
        });
    });

    ui.run()?;
    Ok(())
}

pub struct Downloader;

impl Downloader {
    pub async fn download_file(url: &str, output_path: &str) -> Result<(), std::io::Error> {
        let response = reqwest::get(url).await.map_err(|e| {
            eprintln!("Error making request: {}", e);
            io::Error::new(io::ErrorKind::Other, "Request failed")
        })?;

        if !response.status().is_success() {
            eprintln!("Failed to download the file: {}", response.status());
            return Err(io::Error::new(io::ErrorKind::Other, "HTTP request failed"));
        }

        let mut file = File::create(output_path)?;
        let content = response.bytes().await.map_err(|e| {
            eprintln!("Error reading response bytes: {}", e);
            io::Error::new(io::ErrorKind::Other, "Failed to read response bytes")
        })?;

        copy(&mut content.as_ref(), &mut file)?;
        println!("File downloaded successfully to {:?}", output_path);

        Ok(())
    }
}
