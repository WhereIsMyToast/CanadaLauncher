use crate::{data_structs::ModLoaders, log_to_frontend};
use aws_sdk_s3::{
    config::{Credentials, Region},
    Client, Config,
};
use dirs::config_dir;
use dotenv::dotenv;
use dotenv_codegen::dotenv;
mod loader_installer;
use loader_installer::LoaderInstaller;
use minecraft_instancier::MinecraftInstancier;
mod minecraft_instancier;
use std::{
    error::Error,
    fs::{self, File},
    io::Write,
    path::PathBuf,
    time::SystemTime,
};

fn get_env_var(key: &str, value: &str) -> String {
    if value.is_empty() {
        log_to_frontend(&format!("Error: {} no está configurado o está vacío.", key));
    }
    value.to_string()
}

pub async fn start_install(loader: ModLoaders, loader_version: String, minecraft_version: String) {
    dowload_mods().await;
    let loader_installer = LoaderInstaller {
        loader,
        loader_version: loader_version.clone(),
        minecraft_version,
        minecraft_directory: get_minecraft_directory(),
    };

    let mut launcher_profiles_directory = get_minecraft_directory();
    launcher_profiles_directory.push("launcher_profiles.json");

    let minecraft_instancier = MinecraftInstancier {
        minecraft_canada_directory: get_minecraft_canada_directory(),
        launcher_profiles_directory,
        loader_version: loader_installer.get_version_format(),
    };

    loader_installer.install_loader().await;
    match minecraft_instancier.create_minecraft_instance() {
        Err(e) => {
            log_to_frontend(&format!("Error creating minecraft instance files: {}", e));
        }
        Ok(()) => (),
    };
}

fn get_clients() -> (Vec<Client>, Vec<String>, Vec<PathBuf>) {
    let mut clients = Vec::<Client>::new();
    let mut buckets = Vec::<String>::new();
    let mut directories = Vec::<PathBuf>::new();

    let mut access_key;
    let mut secret_key;
    let mut endpoint_url;
    let mut bucket_name;
    let region = Region::new("us-east-1");
    for i in 0..2 {
        match i {
            0 => {
                access_key = get_env_var("MODS_R2_ACCESS_KEY", dotenv!("MODS_R2_ACCESS_KEY"));
                secret_key = get_env_var("MODS_R2_SECRET_KEY", dotenv!("MODS_R2_SECRET_KEY"));
                endpoint_url = get_env_var("MODS_R2_ENDPOINT", dotenv!("MODS_R2_ENDPOINT"));
                bucket_name = get_env_var("MODS_R2_BUCKET", dotenv!("MODS_R2_BUCKET"));
                buckets.push(bucket_name);
                let mut dir = get_minecraft_canada_directory();
                dir.push("mods");
                directories.push(dir);
            }
            1 => {
                access_key = get_env_var("RESP_R2_ACCESS_KEY", dotenv!("RESP_R2_ACCESS_KEY"));
                secret_key = get_env_var("RESP_R2_SECRET_KEY", dotenv!("RESP_R2_SECRET_KEY"));
                endpoint_url = get_env_var("RESP_R2_ENDPOINT", dotenv!("RESP_R2_ENDPOINT"));
                bucket_name = get_env_var("RESP_R2_BUCKET", dotenv!("RESP_R2_BUCKET"));
                buckets.push(bucket_name);
                let mut dir = get_minecraft_canada_directory();
                dir.push("resourcepacks");
                directories.push(dir);
            }
            _ => panic!("Error in fetching clients"),
        }
        let credentials = Credentials::new(&access_key, &secret_key, None, None, "loaded-from-env");

        let config = Config::builder()
            .credentials_provider(credentials)
            .region(region.clone())
            .endpoint_url(endpoint_url.clone())
            .behavior_version_latest()
            .build();

        clients.push(Client::from_conf(config));
    }
    (clients, buckets, directories)
}

async fn dowload_mods() {
    dotenv().ok();
    let (clients, bucket_names, directories) = get_clients();
    for ((client, bucket_name), directory) in clients
        .into_iter()
        .zip(bucket_names.into_iter())
        .zip(directories.into_iter())
    {
        match sync_files(&client, &bucket_name, directory).await {
            Err(e) => {
                log_to_frontend(&format!("Error to sync files: {}", e));
            }
            Ok(()) => (),
        };
    }
}

async fn sync_files(
    client: &Client,
    bucket: &str,
    directory: PathBuf,
) -> Result<(), Box<dyn Error>> {
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
    if !fs::exists(&directory).unwrap() {
        let _ = fs::create_dir(&directory);
    }
    for obj in remote_files {
        if let Some(key) = obj.key() {
            let remote_size = obj.size;
            let remote_modified = obj
                .last_modified
                .as_ref()
                .map(|t| t.clone().as_secs_f64())
                .unwrap_or(0.0);
            let mut path = directory.clone();
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

    delete_missing_local_files(&remote_keys, directory)?;

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

fn delete_missing_local_files(
    remote_files: &Vec<String>,
    directory: PathBuf,
) -> Result<(), Box<dyn Error>> {
    let local_files: Vec<String> = fs::read_dir(&directory)?
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| entry.file_name().into_string().ok())
        .collect();

    for local_file in &local_files {
        if !remote_files.contains(local_file) {
            log_to_frontend(&format!("\nEliminando archivo local: {}", local_file));
            let mut path = directory.clone();
            path.push(local_file);
            match fs::remove_file(path) {
                Err(e) => log_to_frontend(&format!("error,{}", e)),
                _ok => {}
            }
        }
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

fn get_minecraft_canada_directory() -> PathBuf {
    if let Some(mut appdata) = config_dir() {
        appdata.push(".minecraftCanada");
        return appdata;
    }
    log_to_frontend("Error al obtener appdata");
    PathBuf::new()
}

fn get_minecraft_directory() -> PathBuf {
    if let Some(mut appdata) = config_dir() {
        appdata.push(".minecraft");
        return appdata;
    }
    log_to_frontend("Error al obtener appdata");
    PathBuf::new()
}
