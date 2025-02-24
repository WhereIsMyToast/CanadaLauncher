use data_structs::ModLoaders;
use json_struct_db::JsonConverter;
use self_update::cargo_crate_version;
use serde::Deserialize;
use std::collections::HashMap;
mod data_structs;
mod dowloader;

use std::sync::OnceLock;
use tauri::Emitter;

use tauri::AppHandle;

use serde::Serialize;

#[derive(Clone, Serialize)]
struct LogPayload {
    message: String,
}

static APP_HANDLE: OnceLock<AppHandle> = OnceLock::new();

pub fn set_app_handle(app_handle: AppHandle) {
    APP_HANDLE.set(app_handle).expect("AppHandle already set");
}

pub fn log_to_frontend(message: &str) {
    if let Some(app_handle) = APP_HANDLE.get() {
        app_handle
            .emit(
                "log-event",
                LogPayload {
                    message: message.to_string(),
                },
            )
            .unwrap();
    } else {
        eprintln!("AppHandle no está configurado.");
    }
}
#[derive(Serialize, Deserialize)]
struct Data {
    minecraft_version: String,
    mod_loader: String,
    mod_loader_version: String,
}

impl Data {
    fn new() -> Self {
        Data {
            minecraft_version: String::from(""),
            mod_loader: String::from(""),
            mod_loader_version: String::from(""),
        }
    }
}

impl json_struct_db::JsonConverter for Data {
    fn to_json(&self) -> String {
        serde_json::to_string(self).expect("Failed to serialize MyData")
    }

    fn from_json(json: String) -> Self {
        serde_json::from_str(&json).expect("Failed to deserialize MyData")
    }
}

#[tauri::command]
fn save_data(minecraft_version: String, mod_loader: String, mod_loader_version: String) {
    let data: Data = Data {
        minecraft_version: minecraft_version,
        mod_loader: mod_loader,
        mod_loader_version: mod_loader_version,
    };
    match json_struct_db::save(data, "CanadaLauncher") {
        Ok(path) => {
            println!("Data saved to {}", path)
        }
        Err(e) => {
            println!("{}", e)
        }
    };
}

#[tauri::command]
fn get_data() -> Data {
    match json_struct_db::read("CanadaLauncher") {
        Ok(data) => return Data::from_json(data),
        Err(e) => {
            println!("{}", e.message);
            return Data::new();
        }
    }
}

#[tauri::command]
fn get_version() -> String {
    format!("{}", cargo_crate_version!())
}

#[tauri::command]
async fn get_fabric_versions() -> Vec<String> {
    log_to_frontend("Obteniendo versiones de Fabric... 📡");
    match data_structs::get_fabric_versions().await {
        Err(e) => {
            log_to_frontend(&format!("Error al obtener las versiones de Fabric: {}", e));
            vec![format!("Error cargando versiones {}", e)]
        }
        Ok(version) => {
            log_to_frontend("Versiones de Fabric obtenidas exitosamente! ✅");
            version
        }
    }
}

#[tauri::command]
async fn get_forge_versions() -> HashMap<String, String> {
    log_to_frontend("Obteniendo versiones de Forge... 📡");
    match data_structs::get_forge_versions().await {
        Err(e) => {
            log_to_frontend(&format!("Error al obtener las versiones de Forge: {}", e));
            let mut error_map = HashMap::new();
            error_map.insert(
                "error".to_string(),
                format!("Error cargando versiones: {}", e),
            );
            error_map
        }
        Ok(version) => {
            log_to_frontend("Versiones de Forge obtenidas exitosamente! ✅");
            version
        }
    }
}

#[tauri::command]
async fn start_downloading(minecraft_version: String, mod_type_str: String, mod_version: String) {
    log_to_frontend("Iniciando el proceso de descarga... 📥");
    log_to_frontend(&format!(
        "Versión de Minecraft: {}, Tipo de Mod: {}, Versión de Mod: {}",
        minecraft_version, mod_type_str, mod_version
    ));

    let loader = match mod_type_str.as_str() {
        "forge" => ModLoaders::Forge,
        "fabric" => ModLoaders::Fabric,
        _ => {
            eprintln!("Tipo de mod inválido: {}", mod_type_str);
            return;
        }
    };

    log_to_frontend(&format!("Usando el cargador de mods: {:?}", loader));

    let _ = dowloader::start_install(loader, mod_version, minecraft_version).await;

    log_to_frontend("Proceso de descarga completado. Iniciando Minecraft... 🚀");
    open_minecraft_launcher();
    std::process::exit(0);
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
        .setup(|app| {
            set_app_handle(app.handle().clone());
            Ok(())
        })
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            get_version,
            get_fabric_versions,
            get_forge_versions,
            get_minecraft_versions,
            start_downloading,
            save_data,
            get_data
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
        .bin_name("CanadaLauncher")
        .show_download_progress(true)
        .no_confirm(true)
        .current_version(cargo_crate_version!())
        .build()?
        .update();

    match status {
        Ok(update) => {
            log_to_frontend(&format!(
                "Estado de la actualización: `{}`!",
                update.version()
            ));
            log_to_frontend("¡Actualización exitosa! ✅");
            if update.updated() {
                log_to_frontend("Reiniciando la aplicación... 🔄");
                std::process::Command::new(std::env::current_exe()?).spawn()?;
                std::process::exit(0);
            }
        }
        Err(e) => {
            log_to_frontend(&format!("La actualización falló: {:?}", e));
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

    log_to_frontend(&format!("Iniciando Minecraft: {}", minecraft_launcher_path));
    let _ = std::process::Command::new(minecraft_launcher_path).spawn();
}
