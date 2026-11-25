//! Portable project archives: a `.zip` holding `project.json` and `frames/*`.
//!
//! The archive mirrors the on-disk layout, so it can be moved between
//! machines or kept as a backup. Importing always assigns a fresh id, which
//! means an archive never overwrites an existing document.

use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;

use tauri::AppHandle;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::error::{AppError, Result};
use crate::models::{Project, ProjectSummary};
use crate::storage;

const PROJECT_ENTRY: &str = "project.json";
const FRAMES_PREFIX: &str = "frames/";
/// Guards against zip bombs: no single frame or manifest should exceed this.
const MAX_ENTRY_BYTES: u64 = 256 * 1024 * 1024;

fn zip_err(e: zip::result::ZipError) -> AppError {
    AppError::Other(format!("The archive could not be processed: {e}"))
}

pub fn export(app: &AppHandle, project: &Project, destination: &Path) -> Result<u64> {
    let frames_dir = storage::frames_dir(app, &project.id)?;
    let file = File::create(destination)?;
    let mut zip = ZipWriter::new(file);
    // PNG is already compressed; storing avoids burning CPU for no gain.
    let stored = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    let deflated = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

    zip.start_file(PROJECT_ENTRY, deflated).map_err(zip_err)?;
    zip.write_all(&serde_json::to_vec_pretty(project)?)?;

    for entry in fs::read_dir(&frames_dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        zip.start_file(format!("{FRAMES_PREFIX}{name}"), stored)
            .map_err(zip_err)?;
        let mut source = File::open(entry.path())?;
        std::io::copy(&mut source, &mut zip)?;
    }

    zip.finish().map_err(zip_err)?;
    Ok(fs::metadata(destination)?.len())
}

pub fn import(app: &AppHandle, source: &Path) -> Result<ProjectSummary> {
    let mut archive = ZipArchive::new(File::open(source)?)
        .map_err(|_| AppError::Invalid("That file is not a Walkmark archive (.zip).".into()))?;

    let mut project: Project = {
        let mut entry = archive.by_name(PROJECT_ENTRY).map_err(|_| {
            AppError::Invalid(
                "That zip has no project.json, so it is not a Walkmark archive.".into(),
            )
        })?;
        if entry.size() > MAX_ENTRY_BYTES {
            return Err(AppError::Invalid(
                "The archive's project file is too large.".into(),
            ));
        }
        let mut raw = String::new();
        entry.read_to_string(&mut raw)?;
        serde_json::from_str(&raw)
            .map_err(|_| AppError::Invalid("The archive's project.json is not valid.".into()))?
    };

    // A new id keeps the import from clobbering an existing document.
    project.id = crate::models::new_id();
    let frames_dir = storage::frames_dir(app, &project.id)?;

    let result = (|| -> Result<()> {
        for index in 0..archive.len() {
            let mut entry = archive.by_index(index).map_err(zip_err)?;
            if !entry.is_file() {
                continue;
            }
            let Some(name) = entry.name().strip_prefix(FRAMES_PREFIX).map(str::to_owned) else {
                continue;
            };
            // Same rule as `storage::frame_path`: never escape the frames dir.
            if !is_safe_frame_name(&name) {
                continue;
            }
            if entry.size() > MAX_ENTRY_BYTES {
                return Err(AppError::Invalid(
                    "The archive contains an oversized file.".into(),
                ));
            }
            let mut out = File::create(frames_dir.join(&name))?;
            std::io::copy(&mut entry.take(MAX_ENTRY_BYTES), &mut out)?;
        }
        storage::save_project(app, &project)
    })();

    if let Err(e) = result {
        let _ = storage::delete_project(app, &project.id);
        return Err(e);
    }

    storage::list_projects(app)?
        .into_iter()
        .find(|p| p.id == project.id)
        .ok_or_else(|| AppError::Other("The imported document could not be found.".into()))
}

/// Same rule as `storage::frame_path`: never escape the frames directory.
fn is_safe_frame_name(name: &str) -> bool {
    !name.is_empty() && !name.contains(['/', '\\']) && !name.contains("..")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_names_that_escape_are_rejected() {
        for name in ["../x.png", "a/b.png", "a\\b.png", ""] {
            assert!(!is_safe_frame_name(name), "{name}");
        }
        assert!(is_safe_frame_name("frame-0001.png"));
    }
}
