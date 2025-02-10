use data_structs::ModLoaders;
use self_update::cargo_crate_version;
use std::{collections::HashMap, fmt::format};
mod data_structs;
mod dowloader;
#[cfg(feature = "archive-zip")]
use zip::result::ZipError;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn get_version() -> String {
    format!("{}", cargo_crate_version!())
}

#[tauri::command]
async fn get_fabric_versions() -> Vec<String> {
    println!("Fetching fabric versions...");
    match data_structs::get_fabric_versions().await {
        Err(e) => {
            println!("Error fetching Fabric versions: {}", e);
            vec![format!("Error cargando versiones {}", e)]
        }
        Ok(version) => {
            println!("Fabric versions fetched successfully.");
            version
        }
    }
}

#[tauri::command]
async fn get_forge_versions() -> HashMap<String, String> {
    println!("Fetching Forge versions...");
    match data_structs::get_forge_versions().await {
        Err(e) => {
            println!("Error fetching Forge versions: {}", e);
            let mut error_map = HashMap::new();
            error_map.insert(
                "error".to_string(),
                format!("Error cargando versiones: {}", e),
            );
            error_map
        }
        Ok(version) => {
            println!("Forge versions fetched successfully.");
            version
        }
    }
}

#[tauri::command]
async fn start_downloading(minecraft_version: String, mod_type_str: String, mod_version: String) {
    println!("Starting download process...");
    println!(
        "Minecraft Version: {}, Mod Type: {}, Mod Version: {}",
        minecraft_version, mod_type_str, mod_version
    );

    let loader = match mod_type_str.as_str() {
        "forge" => ModLoaders::Forge,
        "fabric" => ModLoaders::Fabric,
        _ => {
            eprintln!("Invalid mod type: {}", mod_type_str);
            return;
        }
    };

    println!("Using mod loader: {:?}", loader);

    let _ = dowloader::dowload_mods(loader, mod_version, minecraft_version).await;

    println!("Download process completed. Launching Minecraft...");
    open_minecraft_launcher();
}

#[tauri::command]
async fn get_minecraft_versions() -> Vec<String> {
    match data_structs::get_minecraft_versions().await {
        Err(e) => vec![format!("Error cargando versiones {}", e)],
        Ok(version) => version,
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let _ = update_exe();
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            get_version,
            get_fabric_versions,
            get_forge_versions,
            get_minecraft_versions,
            start_downloading
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

const OWNER: &str = "WhereIsMyToast";
const REPO_NAME: &str = "CanadaLauncher";

fn update_exe() -> Result<(), Box<dyn (::std::error::Error)>> {
    let status = self_update::backends::github::Update::configure()
        .repo_owner(OWNER)
        .repo_name(REPO_NAME)
        .bin_name("CanadaLauncher") // Ensure this is correct
        .show_download_progress(true)
        .no_confirm(true)
        .current_version(cargo_crate_version!())
        .build()?
        .update();

    match status {
        Ok(update) => {
            println!("Update status: `{}`!", update.version());
            println!("Update successful!");
            if update.updated() {
                println!("Restarting application...");
                std::process::Command::new(std::env::current_exe()?).spawn()?;
                std::process::exit(0);
            }
        }
        Err(e) => {
            println!("Update failed: {:?}", e);
        }
    }
    Ok(())
}

fn open_minecraft_launcher() {
    let minecraft_launcher_path = if cfg!(target_os = "windows") {
        "C:/XboxGames/Minecraft Launcher/Content/Minecraft.exe"
    } else {
        "/Applications/Minecraft.app/Contents/MacOS/launcher"
    };

    println!("Launching Minecraft: {}", minecraft_launcher_path);
    let _ = std::process::Command::new(minecraft_launcher_path).spawn();
}
