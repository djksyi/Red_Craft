//! 当前实例的存档、光影、材质包和截图。

use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};

use crate::error::{AppError, AppResult};
use crate::state::AppState;

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ResourceKind {
    Saves,
    Shaderpacks,
    Resourcepacks,
    Screenshots,
}

impl ResourceKind {
    fn folder(self) -> &'static str {
        match self {
            Self::Saves => "saves",
            Self::Shaderpacks => "shaderpacks",
            Self::Resourcepacks => "resourcepacks",
            Self::Screenshots => "screenshots",
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceResource {
    name: String,
    path: PathBuf,
    is_directory: bool,
    size: Option<u64>,
    modified: Option<u64>,
}

fn scan(game_dir: &Path, kind: ResourceKind) -> AppResult<Vec<InstanceResource>> {
    let dir = game_dir.join(kind.folder());
    let entries = match std::fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(launcher_core::CoreError::io(&dir, e).into()),
    };
    let root = game_dir.canonicalize().map_err(|e| launcher_core::CoreError::io(game_dir, e))?;
    let canonical_dir = dir.canonicalize().map_err(|e| launcher_core::CoreError::io(&dir, e))?;
    if !canonical_dir.starts_with(&root) {
        return Err(AppError::msg("资源目录不在当前游戏目录内", "请检查资源目录的链接。"));
    }
    let mut resources = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| launcher_core::CoreError::io(&dir, e))?;
        let path = entry.path();
        let file_type = entry.file_type().map_err(|e| launcher_core::CoreError::io(&path, e))?;
        if file_type.is_symlink() || (!file_type.is_file() && !file_type.is_dir()) {
            continue;
        }
        let is_directory = file_type.is_dir();
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
        let include = match kind {
            ResourceKind::Saves => is_directory,
            ResourceKind::Shaderpacks | ResourceKind::Resourcepacks => is_directory || ext == "zip",
            ResourceKind::Screenshots => !is_directory && matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "webp" | "bmp"),
        };
        if !include {
            continue;
        }
        let target = path.canonicalize().map_err(|e| launcher_core::CoreError::io(&path, e))?;
        if !target.starts_with(&canonical_dir) {
            continue;
        }
        let metadata = entry.metadata().map_err(|e| launcher_core::CoreError::io(&path, e))?;
        resources.push(InstanceResource {
            name: entry.file_name().to_string_lossy().into_owned(),
            path: target,
            is_directory,
            size: (!is_directory).then_some(metadata.len()),
            modified: metadata.modified().ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map(|t| t.as_secs()),
        });
    }
    resources.sort_by(|a, b| {
        if matches!(kind, ResourceKind::Screenshots) {
            b.modified.cmp(&a.modified).then_with(|| a.name.cmp(&b.name))
        } else {
            b.is_directory.cmp(&a.is_directory).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
        }
    });
    Ok(resources)
}

#[tauri::command]
pub async fn instance_resources(
    app: AppHandle,
    state: State<'_, AppState>,
    instance: String,
    kind: ResourceKind,
) -> AppResult<Vec<InstanceResource>> {
    let dir = crate::content::instance_context(&state, &instance)?.game_dir;
    crate::commands::blocking(move || {
        let resources = scan(&dir, kind)?;
        if matches!(kind, ResourceKind::Screenshots) {
            for resource in &resources {
                app.asset_protocol_scope().allow_file(&resource.path)
                    .map_err(|e| AppError::msg(format!("无法显示截图：{e}"), "请检查文件权限。"))?;
            }
        }
        Ok(resources)
    }).await?
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn missing_category_is_empty_without_creating_it() {
        let root = tempfile::tempdir().unwrap();
        assert!(scan(root.path(), ResourceKind::Saves).unwrap().is_empty());
        assert!(!root.path().join("saves").exists());
    }

    #[test]
    fn categories_filter_files_and_keep_pack_directories() {
        let root = tempfile::tempdir().unwrap();
        for folder in ["saves", "shaderpacks", "resourcepacks", "screenshots"] {
            fs::create_dir(root.path().join(folder)).unwrap();
        }
        fs::create_dir(root.path().join("saves/World")).unwrap();
        fs::write(root.path().join("saves/notes.txt"), "ignore").unwrap();
        fs::create_dir(root.path().join("resourcepacks/Unpacked")).unwrap();
        fs::write(root.path().join("resourcepacks/a.ZIP"), "pack").unwrap();
        fs::write(root.path().join("resourcepacks/readme.txt"), "ignore").unwrap();
        fs::write(root.path().join("shaderpacks/shader.zip"), "shader").unwrap();
        fs::write(root.path().join("screenshots/image.PNG"), "image").unwrap();
        fs::write(root.path().join("screenshots/notes.txt"), "ignore").unwrap();
        fs::create_dir(root.path().join("screenshots/folder")).unwrap();

        let saves = scan(root.path(), ResourceKind::Saves).unwrap();
        assert_eq!(saves.len(), 1);
        assert_eq!(saves[0].name, "World");
        assert!(saves[0].is_directory);
        assert_eq!(saves[0].size, None);
        let packs = scan(root.path(), ResourceKind::Resourcepacks).unwrap();
        assert_eq!(packs.iter().map(|r| r.name.as_str()).collect::<Vec<_>>(), ["Unpacked", "a.ZIP"]);
        assert_eq!(packs[1].size, Some(4));
        assert_eq!(scan(root.path(), ResourceKind::Shaderpacks).unwrap().len(), 1);
        let images = scan(root.path(), ResourceKind::Screenshots).unwrap();
        assert_eq!(images.len(), 1);
        assert_eq!(images[0].name, "image.PNG");
        assert!(images[0].path.starts_with(root.path().canonicalize().unwrap()));
    }

    #[test]
    fn invalid_resource_categories_are_rejected() {
        assert!(serde_json::from_str::<ResourceKind>("\"../config\"").is_err());
        assert!(serde_json::from_str::<ResourceKind>("\"mods\"").is_err());
    }
}
