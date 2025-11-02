// Сериализация данных для сохранения и передачи

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Сериализует данные в JSON файл
pub fn save_json<T: Serialize>(data: &T, path: impl AsRef<Path>) -> Result<(), String> {
    let json = serde_json::to_string_pretty(data)
        .map_err(|e| format!("Failed to serialize to JSON: {}", e))?;

    fs::write(path, json).map_err(|e| format!("Failed to write file: {}", e))?;

    Ok(())
}

/// Десериализует данные из JSON файла
pub fn load_json<T: for<'de> Deserialize<'de>>(path: impl AsRef<Path>) -> Result<T, String> {
    let contents = fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;

    serde_json::from_str(&contents).map_err(|e| format!("Failed to deserialize from JSON: {}", e))
}

/// Сериализует данные в бинарный формат (bincode)
pub fn save_binary<T: Serialize>(data: &T, path: impl AsRef<Path>) -> Result<(), String> {
    let bytes =
        bincode::serialize(data).map_err(|e| format!("Failed to serialize to binary: {}", e))?;

    fs::write(path, bytes).map_err(|e| format!("Failed to write file: {}", e))?;

    Ok(())
}

/// Десериализует данные из бинарного формата (bincode)
pub fn load_binary<T: for<'de> Deserialize<'de>>(path: impl AsRef<Path>) -> Result<T, String> {
    let bytes = fs::read(path).map_err(|e| format!("Failed to read file: {}", e))?;

    bincode::deserialize(&bytes).map_err(|e| format!("Failed to deserialize from binary: {}", e))
}

/// Сериализует данные в TOML файл
pub fn save_toml<T: Serialize>(data: &T, path: impl AsRef<Path>) -> Result<(), String> {
    let toml =
        toml::to_string_pretty(data).map_err(|e| format!("Failed to serialize to TOML: {}", e))?;

    fs::write(path, toml).map_err(|e| format!("Failed to write file: {}", e))?;

    Ok(())
}

/// Десериализует данные из TOML файла
pub fn load_toml<T: for<'de> Deserialize<'de>>(path: impl AsRef<Path>) -> Result<T, String> {
    let contents = fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;

    toml::from_str(&contents).map_err(|e| format!("Failed to deserialize from TOML: {}", e))
}

/// Сериализует данные в формат, определяемый расширением файла
pub fn save_auto<T: Serialize>(data: &T, path: impl AsRef<Path>) -> Result<(), String> {
    let path = path.as_ref();

    if let Some(ext) = path.extension() {
        match ext.to_str() {
            Some("json") => save_json(data, path),
            Some("toml") => save_toml(data, path),
            Some("bin") | Some("bincode") => save_binary(data, path),
            _ => Err(format!("Unsupported file extension: {:?}", ext)),
        }
    } else {
        // По умолчанию используем JSON
        save_json(data, path)
    }
}

/// Десериализует данные из формата, определяемого расширением файла
pub fn load_auto<T: for<'de> Deserialize<'de>>(path: impl AsRef<Path>) -> Result<T, String> {
    let path = path.as_ref();

    if let Some(ext) = path.extension() {
        match ext.to_str() {
            Some("json") => load_json(path),
            Some("toml") => load_toml(path),
            Some("bin") | Some("bincode") => load_binary(path),
            _ => Err(format!("Unsupported file extension: {:?}", ext)),
        }
    } else {
        // По умолчанию используем JSON
        load_json(path)
    }
}
