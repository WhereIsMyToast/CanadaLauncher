#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use reqwest::Error;
#[derive(Serialize, Deserialize)]
pub struct MinecraftProfile {
    pub name: String,
    #[serde(rename = "gameDir")]
    pub game_dir: String,
    #[serde(rename = "lastVersionId")]
    pub version: String,
    #[serde(rename = "javaArgs")]
    pub java_args: Option<String>,
    pub icon: String,
}

#[derive(Deserialize, Debug)]
pub struct MinecraftApiResponse {
    latest: MinecraftLatest,
    pub versions: Vec<MinecraftVersion>,
}

#[derive(Deserialize, Debug)]
pub struct MinecraftLatest {
    release: String,
    snapshot: String,
}

#[derive(Deserialize, Debug)]
pub struct MinecraftVersion {
    pub id: String,
    #[serde(rename = "type")]
    pub version_type: String,
    url: String,
    time: String,
    release_time: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct FabricApiResponse {
    url: String,
    maven: String,
    pub version: String,
    stable: bool,
}

#[derive(PartialEq, Default, Debug, Clone, Serialize, Deserialize)]
pub enum ModLoaders {
    Forge,
    #[default]
    Fabric,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ForgeApiResponse {
    homepage: String,
    pub promos: HashMap<String, String>,
}

pub async fn get_fabric_versions() -> Result<Vec<String>, Error> {
    let api = "https://meta2.fabricmc.net/v2/versions/installer";

    let client = reqwest::Client::new();
    let response = client
        .get(api)
        .header("User-Agent", "reqwest")
        .send()
        .await?;

    let api_response: Vec<FabricApiResponse> = response.json().await?;

    let versions: Vec<String> = api_response
        .into_iter()
        .map(|fabric| fabric.version)
        .collect();
    Ok(versions)
}

pub async fn get_minecraft_versions() -> Result<Vec<String>, reqwest::Error> {
    let api = "https://launchermeta.mojang.com/mc/game/version_manifest.json";
    let response = reqwest::get(api).await?;
    let api_response: MinecraftApiResponse = response.json().await?;
    let release_versions: Vec<String> = api_response
        .versions
        .into_iter()
        .filter(|version| version.version_type == "release")
        .map(|version| version.id)
        .collect();
    Ok(release_versions)
}

pub async fn get_forge_versions() -> Result<HashMap<String, String>, reqwest::Error> {
    let api = "https://files.minecraftforge.net/net/minecraftforge/forge/promotions_slim.json";
    let response = reqwest::get(api).await?;
    let api_response: ForgeApiResponse = response.json().await?;
    Ok(api_response.promos)
}
