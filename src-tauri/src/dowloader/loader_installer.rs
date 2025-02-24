use std::{
    env,
    error::Error,
    fs::{self, File},
    io::Write,
    path::PathBuf,
    process::Command,
};

use crate::{data_structs::ModLoaders, log_to_frontend};

pub struct LoaderInstaller {
    pub loader: ModLoaders,
    pub minecraft_version: String,
    pub loader_version: String,
    pub minecraft_directory: PathBuf,
}

impl LoaderInstaller {
    pub async fn install_loader(&self) {
        if self.is_version_installed() {
            log_to_frontend(&format!(
                "{} versión {} ya está instalada, omitiendo instalación.",
                match self.loader {
                    ModLoaders::Forge => "Forge",
                    ModLoaders::Fabric => "Fabric",
                },
                self.loader_version
            ));
            return;
        }
        match self.loader {
            ModLoaders::Forge => {
                log_to_frontend("Instalando Forge... ⚙️");
                if let Err(e) = self.install_forge().await {
                    log_to_frontend(&format!("La instalación de Forge falló: {}", e));
                }
            }
            ModLoaders::Fabric => {
                log_to_frontend("Instalando Fabric... ⚙️");
                if let Err(e) = self.install_fabric().await {
                    log_to_frontend(&format!("La instalación de Fabric falló: {}", e));
                }
            }
        }
    }

    pub fn get_version_format(&self) -> String {
        match self.loader {
            ModLoaders::Forge => {
                format!("{}-forge-{}", self.minecraft_version, self.loader_version)
            }
            ModLoaders::Fabric => format!("fabric-loader-{}-{}", "0.16.10", self.minecraft_version),
        }
    }

    fn is_version_installed(&self) -> bool {
        let mut versions_path = self.minecraft_directory.clone();
        versions_path.push("versions");
        versions_path.push(self.get_version_format());
        versions_path.exists()
    }
    async fn download_installer(&self) -> Result<(), Box<dyn Error>> {
        log_to_frontend(&format!(
            "Descargando instalador: {}",
            self.get_installer_url()
        ));
        let response = reqwest::get(self.get_installer_url())
            .await?
            .bytes()
            .await?;
        let mut file = File::create(Self::get_temp_path())?;
        file.write_all(&response)?;
        log_to_frontend(&format!("Descargado: {}", Self::get_temp_path()));
        Ok(())
    }

    fn get_installer_url(&self) -> String {
        match self.loader {
            ModLoaders::Fabric => {
                format!(
                    "https://maven.fabricmc.net/net/fabricmc/fabric-installer/{0}/fabric-installer-{0}.jar",
                    self.loader_version
                )
            }
            ModLoaders::Forge => {
                let full_version = format!("{}-{}", self.minecraft_version, self.loader_version);
                format!(
                    "https://maven.minecraftforge.net/net/minecraftforge/forge/{}/forge-{}-installer.jar",
                    full_version, full_version
                )
            }
        }
    }

    fn get_temp_path() -> String {
        let save_path = env::temp_dir().join("fabric_installer.jar");
        save_path
            .to_str()
            .unwrap_or("C:/temp/fabric_installer.jar")
            .to_string()
    }

    async fn install_fabric(&self) -> Result<(), Box<dyn Error>> {
        self.download_installer().await?;

        log_to_frontend("Instalando Fabric...");

        let java_command = if cfg!(target_os = "windows") {
            "java.exe"
        } else {
            "java"
        };

        let command_str = format!(
            "{} -jar \"{}\" client -dir \"{}\" -mcversion {}",
            java_command,
            Self::get_temp_path(),
            self.minecraft_directory.display(),
            self.minecraft_version
        );

        log_to_frontend(&format!("Ejecutando comando: {}", command_str));

        let status = Command::new(java_command)
            .arg("-jar")
            .arg(Self::get_temp_path())
            .arg("client")
            .arg("-dir")
            .arg(self.minecraft_directory.clone())
            .arg("-mcversion")
            .arg(self.minecraft_version.clone())
            .status()?;

        if status.success() {
            log_to_frontend("Instalación de Fabric exitosa.");
            let _ = fs::remove_file(Self::get_temp_path());
        } else {
            log_to_frontend("Error al instalar Fabric.");
        }

        Ok(())
    }

    async fn install_forge(&self) -> Result<(), Box<dyn Error>> {
        log_to_frontend(&format!(
            "Intentando descargar el instalador de Forge desde: {}",
            self.get_installer_url()
        ));

        if let Err(e) = self.download_installer().await {
            log_to_frontend(&format!(
                "No se pudo descargar el instalador de Forge: {}",
                e
            ));
            return Err(e);
        }

        log_to_frontend("Instalador de Forge descargado exitosamente.");
        log_to_frontend("Ejecutando el instalador de Forge...");

        let java_command = if cfg!(target_os = "windows") {
            "java.exe"
        } else {
            "java"
        };

        log_to_frontend(&format!(
            "Ejecutando instalador de paquete Java: {} {}",
            Self::get_temp_path(),
            "--installClient"
        ));

        let status = Command::new(java_command)
            .arg("-jar")
            .arg(Self::get_temp_path())
            .arg("--installClient")
            .arg(self.minecraft_directory.clone())
            .status()?;

        if status.success() {
            log_to_frontend("Instalación exitosa.");
            let _ = fs::remove_file(Self::get_temp_path());
        } else {
            log_to_frontend("Error al instalar.");
        }

        log_to_frontend("Instalación de Forge completada exitosamente.");
        Ok(())
    }
}
