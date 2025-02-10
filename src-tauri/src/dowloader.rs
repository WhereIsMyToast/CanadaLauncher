use crate::data_structs::{get_fabric_versions, get_forge_versions, MinecraftProfile, ModLoaders};
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
    path::{Path, PathBuf},
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
    let mut minecraft_dir = get_mods_dir();
    minecraft_dir.pop();
    let _ = create_minecraft_instance(
        minecraft_dir.clone(),
        get_version_format(&minecraft_version, &mod_version, loader.clone()),
    );

    match loader {
        ModLoaders::Forge => {
            println!("Installing Forge...");
            if let Err(e) = install_forge(&mod_version, minecraft_version.clone()).await {
                eprintln!("Forge installation failed: {}", e);
            }
        }
        ModLoaders::Fabric => {
            println!("Installing Fabric...");
            if let Err(e) = install_fabric(&mod_version, &minecraft_dir, &minecraft_version).await {
                eprintln!("Fabric installation failed: {}", e);
            }
        }
    }
    sync_files(&client, &bucket_name).await?;

    Ok(())
}

fn get_version_format(minecraft_version: &str, tool_version: &str, tool: ModLoaders) -> String {
    match tool {
        ModLoaders::Forge => format!("{}-forge-{}", minecraft_version, tool_version),
        ModLoaders::Fabric => format!("fabric-loader-{}-{}", "0.16.5", minecraft_version),
    }
}

async fn sync_files(client: &Client, bucket: &str) -> Result<(), Box<dyn Error>> {
    let resp = client.list_objects_v2().bucket(bucket).send().await?;
    let remote_files = resp.contents();

    if remote_files.is_empty() {
        println!("Bucket is empty or listing is restricted");
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
                println!(
                    "\n Downloading updated file: {} in {}",
                    key, local_file_path
                );

                download_file(client, bucket, key, &local_file_path).await?;
            } else {
                println!("No changes detected for '{}', skipping...", key);
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
    println!(
        "Attempting to download Forge installer from: {}",
        installer_url
    );

    if let Err(e) = download_installer(&installer_url, save_path_str).await {
        eprintln!("Failed to download Forge installer: {}", e);
        return Err(e);
    }

    println!("Forge installer downloaded successfully.");
    println!("Running Forge installer...");

    let mut minecraft_dir = get_mods_dir();
    minecraft_dir.pop();
    if let Err(e) = install_java_package(
        save_path_str,
        "--installClient",
        minecraft_dir.to_str().unwrap(),
    )
    .await
    {
        eprintln!("Forge installation failed: {}", e);
        return Err(e);
    }

    println!("Forge installation completed successfully.");
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
            println!("\n Deleting local file: {}", local_file);
            let mut path = get_mods_dir();
            path.push(local_file);
            match fs::remove_file(path) {
                Err(e) => println!("error,{}", e),
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

    println!("Downloading Fabric installer from {}", installer_url);

    download_installer(&installer_url, save_path_str).await?;

    println!("Installing Fabric...");

    let java_command = if cfg!(target_os = "windows") {
        "java.exe"
    } else {
        "java"
    };

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
        println!("Fabric installation successful.");
        let _ = fs::remove_file(save_path);
    } else {
        eprintln!("Error installing Fabric.");
    }

    Ok(())
}

async fn download_installer(url: &str, save_path: &str) -> Result<(), Box<dyn Error>> {
    println!("Downloading installer: {}", url);
    let response = reqwest::get(url).await?.bytes().await?;
    let mut file = File::create(save_path)?;
    file.write_all(&response)?;
    println!("Downloaded: {}", save_path);
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

    println!(
        "Running Java package installer: {} {}",
        installer_path, args
    );

    let status = Command::new(java_command)
        .arg("-jar")
        .arg(installer_path)
        .arg(args)
        .arg(test)
        .status()?;

    if status.success() {
        println!("Installation successful.");
        let _ = fs::remove_file(installer_path);
    } else {
        eprintln!("Error installing.");
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

    println!("Downloading '{}', size: {} bytes", key, body_bytes.len());

    let mut file = File::create(save_path)?;
    file.write_all(&body_bytes)?;

    println!("Downloaded file saved as '{}'", save_path);
    Ok(())
}

fn get_mods_dir() -> PathBuf {
    if let Some(mut appdata) = config_dir() {
        appdata.push(".minecraftCanada/mods");
        return appdata;
    }
    println!("Error getting appdata");
    PathBuf::new()
}

fn create_minecraft_instance(new_dir: PathBuf, version: String) -> Result<(), std::io::Error> {
    let mut profiles_json = get_mods_dir();
    profiles_json.pop();
    profiles_json.push("launcher_profiles.json");
    println!("{}", profiles_json.display());

    // Ensure the directory exists
    if !new_dir.exists() {
        fs::create_dir_all(&new_dir)?;
        println!("Directory created: {}", new_dir.display());
    }

    // Overwrite the launcher_profiles.json file
    let mut file = fs::File::create(&profiles_json)?; // This truncates the file if it exists
    file.write_all(b"{}")?; // Reset file content to an empty JSON object

    let mut profiles: serde_json::Value = serde_json::from_str("{}")?; // Start fresh

    let new_profile = MinecraftProfile {
        name: "Canada Mods".to_string(),
        game_dir: new_dir.to_string_lossy().to_string(),
        version: version.clone(),
        java_args: Some("-Xmx7G".to_string()),
        icon: get_encoded_incon(),
    };

    let profile_json = serde_json::to_value(&new_profile)?;
    profiles["profiles"]["Modded Profile"] = profile_json;

    let updated_profiles = serde_json::to_string_pretty(&profiles)?;
    fs::write(&profiles_json, updated_profiles)?;

    println!("Successfully created a new Minecraft profile with a separate instance.");

    Ok(())
}

static ICON: &[u8] = include_bytes!("../canada.ico");
fn get_encoded_incon() -> String {
    let encoder = general_purpose::STANDARD;

    let base64_encoded_icon = encoder.encode(ICON);

    format!("data:image/x-icon;base64,{}", base64_encoded_icon)
}
