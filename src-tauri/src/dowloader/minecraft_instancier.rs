use std::{fs, io::Error, path::PathBuf};

use base64::{engine::general_purpose, Engine};
use chrono::Utc;

use crate::{data_structs::MinecraftProfile, log_to_frontend};

pub struct MinecraftInstancier {
    pub minecraft_canada_directory: PathBuf,
    pub loader_version: String,
    pub launcher_profiles_directory: PathBuf,
}
static ICON: &[u8] = include_bytes!("../../canada.png");

impl MinecraftInstancier {
    pub fn create_minecraft_instance(&self) -> Result<(), Error> {
        log_to_frontend(&format!("{}", self.launcher_profiles_directory.display()));

        if !self.minecraft_canada_directory.exists() {
            fs::create_dir_all(&self.minecraft_canada_directory)?;
            log_to_frontend(&format!(
                "Directorio creado: {}",
                self.minecraft_canada_directory.display()
            ));
        }
        let profiles: serde_json::Value = if self.launcher_profiles_directory.exists() {
            let file_contents = fs::read_to_string(&self.launcher_profiles_directory)?;
            serde_json::from_str(&file_contents)
                .unwrap_or_else(|_| serde_json::json!({"profiles": {}}))
        } else {
            serde_json::json!({"profiles": {}})
        };

        let mut profiles_obj = profiles.clone();

        let new_profile = MinecraftProfile {
            name: "Canada Mods".to_string(),
            game_dir: self
                .minecraft_canada_directory
                .to_string_lossy()
                .to_string(),
            version: self.loader_version.clone(),
            java_args: Some("-Xmx7G".to_string()),
            icon: Self::get_encoded_icon(),
            last_used: Self::get_now_time(),
        };

        println!(
            "Generated Profile JSON: {}",
            serde_json::to_string_pretty(&new_profile)?
        );

        let profile_json = serde_json::to_value(&new_profile)?;

        if let Some(profiles_map) = profiles_obj["profiles"].as_object_mut() {
            profiles_map.insert("Modded Profile".to_string(), profile_json);
        } else {
            profiles_obj["profiles"] = serde_json::json!({
                "Modded Profile": profile_json
            });
        }

        let updated_profiles = serde_json::to_string_pretty(&profiles_obj)?;
        fs::write(&self.launcher_profiles_directory, updated_profiles)?;

        log_to_frontend("Perfil de Minecraft creado exitosamente con una instancia separada.");

        Ok(())
    }

    fn get_encoded_icon() -> String {
        let encoder = general_purpose::STANDARD;
        let base64_encoded_icon = encoder.encode(ICON);
        format!("data:image/png;base64,{}", base64_encoded_icon)
    }

    fn get_now_time() -> String {
        let now = Utc::now();
        now.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()
    }
}
