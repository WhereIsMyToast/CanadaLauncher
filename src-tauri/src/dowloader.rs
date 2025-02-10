use crate::{
    data_structs::{MinecraftProfile, ModLoaders},
    log_to_frontend,
};
use aws_sdk_s3::{
    config::{Credentials, Region},
    Client, Config,
};
use base64::{engine::general_purpose, Engine};
use dirs::config_dir;
use dotenv::dotenv;
use reqwest;
use std::{
    env,
    error::Error,
    fs::{self, File},
    io::Write,
    path::PathBuf,
    process::Command,
    time::SystemTime,
};

pub async fn dowload_mods(
    loader: ModLoaders,
    mod_version: String,
    minecraft_version: String,
) -> Result<(), Box<dyn Error>> {
    dotenv().ok();

    let access_key = env::var("R2_ACCESS_KEY").expect("Set R2_ACCESS_KEY");
    let secret_key = env::var("R2_SECRET_KEY").expect("Set R2_SECRET_KEY");
    let endpoint_url = env::var("R2_ENDPOINT").expect("Set R2_ENDPOINT");
    let bucket_name = env::var("R2_BUCKET").expect("Set R2_BUCKET");
    let region = Region::new("us-east-1");

    let credentials = Credentials::new(&access_key, &secret_key, None, None, "loaded-from-env");

    let config = Config::builder()
        .credentials_provider(credentials)
        .region(region)
        .endpoint_url(endpoint_url)
        .behavior_version_latest()
        .build();

    let client = Client::from_conf(config);
    let mut minecraft_canada_dir = get_mods_dir();
    minecraft_canada_dir.pop();

    let mut minecraft_dir = get_mods_dir();
    minecraft_dir.pop();
    minecraft_dir.pop();
    minecraft_dir.push(".minecraft");
    minecraft_dir.push("launcher_profiles.json");

    let _ = create_minecraft_instance(
        minecraft_canada_dir.clone(),
        get_version_format(&minecraft_version, &mod_version, loader.clone()),
        minecraft_dir.clone(),
    );

    minecraft_dir.pop();

    if !is_version_installed(&minecraft_version, &mod_version, loader.clone()) {
        match loader {
            ModLoaders::Forge => {
                log_to_frontend("Instalando Forge... ⚙️");
                if let Err(e) = install_forge(&mod_version, minecraft_version.clone()).await {
                    log_to_frontend(&format!("La instalación de Forge falló: {}", e));
                }
            }
            ModLoaders::Fabric => {
                log_to_frontend("Instalando Fabric... ⚙️");
                if let Err(e) =
                    install_fabric(&mod_version, &minecraft_dir, &minecraft_version).await
                {
                    log_to_frontend(&format!("La instalación de Fabric falló: {}", e));
                }
            }
        }
    } else {
        log_to_frontend(&format!(
            "{} versión {} ya está instalada, omitiendo instalación.",
            match loader {
                ModLoaders::Forge => "Forge",
                ModLoaders::Fabric => "Fabric",
            },
            mod_version
        ));
    }

    sync_files(&client, &bucket_name).await?;

    Ok(())
}

fn get_version_format(minecraft_version: &str, tool_version: &str, tool: ModLoaders) -> String {
    match tool {
        ModLoaders::Forge => format!("{}-forge-{}", minecraft_version, tool_version),
        ModLoaders::Fabric => format!("fabric-loader-{}-{}", "0.16.10", minecraft_version),
    }
}

async fn sync_files(client: &Client, bucket: &str) -> Result<(), Box<dyn Error>> {
    let resp = client.list_objects_v2().bucket(bucket).send().await?;
    let remote_files = resp.contents();

    if remote_files.is_empty() {
        log_to_frontend("El bucket está vacío o la lista está restringida.");
        return Ok(());
    }
    let remote_keys: Vec<String> = remote_files
        .iter()
        .filter_map(|obj| obj.key().map(String::from))
        .collect();
    if !fs::exists(&get_mods_dir()).unwrap() {
        let _ = fs::create_dir(&get_mods_dir());
    }
    for obj in remote_files {
        if let Some(key) = obj.key() {
            let remote_size = obj.size;
            let remote_modified = obj
                .last_modified
                .as_ref()
                .map(|t| t.clone().as_secs_f64())
                .unwrap_or(0.0);
            let mut path = get_mods_dir();
            path.push(key);
            let local_file_path = path.to_str().unwrap_or_default();

            if should_download(&local_file_path, remote_size.unwrap(), remote_modified) {
                log_to_frontend(&format!(
                    "\nDescargando archivo actualizado: {} en {}",
                    key, local_file_path
                ));

                download_file(client, bucket, key, &local_file_path).await?;
            } else {
                log_to_frontend(&format!(
                    "No se detectaron cambios para '{}', omitiendo...",
                    key
                ));
            }
        }
    }

    delete_missing_local_files(&remote_keys)?;

    Ok(())
}

async fn install_forge(version: &str, minecraft_version: String) -> Result<(), Box<dyn Error>> {
    let full_version = format!("{}-{}", minecraft_version, version);
    let installer_url = format!(
        "https://maven.minecraftforge.net/net/minecraftforge/forge/{}/forge-{}-installer.jar",
        full_version, full_version
    );
    let save_path = env::temp_dir().join("forge_installer.jar");
    let save_path_str = save_path.to_str().unwrap_or("C:/temp/forge_installer.jar");
    log_to_frontend(&format!(
        "Intentando descargar el instalador de Forge desde: {}",
        installer_url
    ));

    if let Err(e) = download_installer(&installer_url, save_path_str).await {
        log_to_frontend(&format!(
            "No se pudo descargar el instalador de Forge: {}",
            e
        ));
        return Err(e);
    }

    log_to_frontend("Instalador de Forge descargado exitosamente.");
    log_to_frontend("Ejecutando el instalador de Forge...");

    let mut minecraft_dir = get_mods_dir();
    minecraft_dir.pop();
    minecraft_dir.pop();
    minecraft_dir.push(".minecraft");
    if let Err(e) = install_java_package(
        save_path_str,
        "--installClient",
        minecraft_dir.to_str().unwrap(),
    )
    .await
    {
        log_to_frontend(&format!("La instalación de Forge falló: {}", e));
        return Err(e);
    }

    log_to_frontend("Instalación de Forge completada exitosamente.");
    Ok(())
}

fn should_download(local_path: &str, remote_size: i64, remote_modified: f64) -> bool {
    if let Ok(metadata) = fs::metadata(local_path) {
        let local_size = metadata.len() as i64;
        let local_modified = metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0);

        return remote_size != local_size || remote_modified > local_modified;
    }
    true
}

fn delete_missing_local_files(remote_files: &Vec<String>) -> Result<(), Box<dyn Error>> {
    let local_files: Vec<String> = fs::read_dir(get_mods_dir())?
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| entry.file_name().into_string().ok())
        .collect();

    for local_file in &local_files {
        if !remote_files.contains(local_file) {
            log_to_frontend(&format!("\nEliminando archivo local: {}", local_file));
            let mut path = get_mods_dir();
            path.push(local_file);
            match fs::remove_file(path) {
                Err(e) => log_to_frontend(&format!("error,{}", e)),
                _ok => {}
            }
        }
    }

    Ok(())
}

async fn install_fabric(
    version: &str,
    minecraft_dir: &PathBuf,
    minecraft_version: &str,
) -> Result<(), Box<dyn Error>> {
    let installer_url = format!(
        "https://maven.fabricmc.net/net/fabricmc/fabric-installer/{0}/fabric-installer-{0}.jar",
        version
    );

    let save_path = env::temp_dir().join("fabric_installer.jar");
    let save_path_str = save_path.to_str().unwrap_or("C:/temp/fabric_installer.jar");

    log_to_frontend(&format!(
        "Descargando instalador de Fabric desde: {}",
        installer_url
    ));

    download_installer(&installer_url, save_path_str).await?;

    log_to_frontend("Instalando Fabric...");

    let java_command = if cfg!(target_os = "windows") {
        "java.exe"
    } else {
        "java"
    };

    let command_str = format!(
        "{} -jar \"{}\" client -dir \"{}\" -mcversion {}",
        java_command,
        save_path_str,
        minecraft_dir.display(),
        minecraft_version
    );

    log_to_frontend(&format!("Ejecutando comando: {}", command_str));

    let status = Command::new(java_command)
        .arg("-jar")
        .arg(save_path_str)
        .arg("client")
        .arg("-dir")
        .arg(minecraft_dir)
        .arg("-mcversion")
        .arg(minecraft_version)
        .status()?;

    if status.success() {
        log_to_frontend("Instalación de Fabric exitosa.");
        let _ = fs::remove_file(save_path);
    } else {
        log_to_frontend("Error al instalar Fabric.");
    }

    Ok(())
}

async fn download_installer(url: &str, save_path: &str) -> Result<(), Box<dyn Error>> {
    log_to_frontend(&format!("Descargando instalador: {}", url));
    let response = reqwest::get(url).await?.bytes().await?;
    let mut file = File::create(save_path)?;
    file.write_all(&response)?;
    log_to_frontend(&format!("Descargado: {}", save_path));
    Ok(())
}

async fn install_java_package(
    installer_path: &str,
    args: &str,
    test: &str,
) -> Result<(), Box<dyn Error>> {
    let java_command = if cfg!(target_os = "windows") {
        "java.exe"
    } else {
        "java"
    };

    log_to_frontend(&format!(
        "Ejecutando instalador de paquete Java: {} {}",
        installer_path, args
    ));

    let status = Command::new(java_command)
        .arg("-jar")
        .arg(installer_path)
        .arg(args)
        .arg(test)
        .status()?;

    if status.success() {
        log_to_frontend("Instalación exitosa.");
        let _ = fs::remove_file(installer_path);
    } else {
        log_to_frontend("Error al instalar.");
    }
    Ok(())
}

async fn download_file(
    client: &Client,
    bucket: &str,
    key: &str,
    save_path: &str,
) -> Result<(), Box<dyn Error>> {
    let resp = client.get_object().bucket(bucket).key(key).send().await?;
    let body_bytes = resp.body.collect().await?.into_bytes();

    log_to_frontend(&format!(
        "Descargando '{}', tamaño: {} bytes",
        key,
        body_bytes.len()
    ));

    let mut file = File::create(save_path)?;
    file.write_all(&body_bytes)?;

    log_to_frontend(&format!("Archivo descargado guardado como '{}'", save_path));
    Ok(())
}

fn get_mods_dir() -> PathBuf {
    if let Some(mut appdata) = config_dir() {
        appdata.push(".minecraftCanada/mods");
        return appdata;
    }
    log_to_frontend("Error al obtener appdata");
    PathBuf::new()
}

fn create_minecraft_instance(
    new_dir: PathBuf,
    version: String,
    profiles_json: PathBuf,
) -> Result<(), std::io::Error> {
    log_to_frontend(&format!("{}", profiles_json.display()));

    if !new_dir.exists() {
        fs::create_dir_all(&new_dir)?;
        log_to_frontend(&format!("Directorio creado: {}", new_dir.display()));
    }

    let mut file = fs::File::create(&profiles_json)?;
    file.write_all(b"{}")?;

    let mut profiles: serde_json::Value = serde_json::from_str("{}")?;
    let new_profile = MinecraftProfile {
        name: "Canada Mods".to_string(),
        game_dir: new_dir.to_string_lossy().to_string(),
        version: version.clone(),
        java_args: Some("-Xmx7G".to_string()),
        icon: get_encoded_icon(),
    };

    let profile_json = serde_json::to_value(&new_profile)?;
    profiles["profiles"]["Modded Profile"] = profile_json;

    let updated_profiles = serde_json::to_string_pretty(&profiles)?;
    fs::write(&profiles_json, updated_profiles)?;

    log_to_frontend("Perfil de Minecraft creado exitosamente con una instancia separada.");

    Ok(())
}

static ICON: &[u8] = include_bytes!("../canada.png");
fn get_encoded_icon() -> String {
    let encoder = general_purpose::STANDARD;
    let base64_encoded_icon = encoder.encode(ICON);
    format!("data:image/png;base64,{}", base64_encoded_icon)
}

fn is_version_installed(minecraft_version: &str, mod_version: &str, loader: ModLoaders) -> bool {
    let mut versions_path = get_mods_dir();
    versions_path.pop();
    versions_path.pop();
    versions_path.push(".minecraft");
    versions_path.push("versions");

    let version_format = get_version_format(minecraft_version, mod_version, loader);
    let mut version_dir = versions_path.clone();
    version_dir.push(version_format);

    version_dir.exists()
}
