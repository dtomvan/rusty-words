use std::path::PathBuf;

use color_eyre::{eyre::eyre, Result};
use dirs::data_dir;
use uuid::Uuid;

pub fn root_dir() -> Result<PathBuf> {
    data_dir()
        .map(|x| x.join("rusty-words"))
        .ok_or_else(|| eyre!("Could not find root dir"))
}

pub fn index_file() -> Result<PathBuf> {
    Ok(root_dir()?.join("index.ron"))
}

pub fn words_file(uuid: &Uuid) -> Result<PathBuf> {
    Ok(root_dir()?.join(format!("{uuid}.ron")))
}

