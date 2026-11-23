pub mod html;
pub mod markdown;
pub mod pdf;

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use crate::error::{AppError, Result};
use crate::imaging;
use crate::models::{DocumentLabels, ExportFormat, Project};
use crate::state::AppState;
use crate::storage;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportOptions {
    pub format: ExportFormat,
    pub include_images: bool,
    /// Longest edge of images inside the document.
    pub image_width: u32,
    pub include_toc: bool,
    pub include_summary: bool,
    pub include_prerequisites: bool,
    /// The line under the title with the export date, version and author.
    #[serde(default = "default_true")]
    pub include_details: bool,
    /// Filled in by `write` when the document is produced; ignored on input.
    #[serde(default)]
    pub exported_on: String,
    /// Fixed wording in the guide's language. Filled in by `write` from
    /// Settings; ignored on input.
    #[serde(skip)]
    pub labels: DocumentLabels,
    /// HTML only. "light" | "dark" | "auto"
    pub theme: String,
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            format: ExportFormat::Html,
            include_images: true,
            image_width: 1400,
            include_toc: true,
            include_summary: true,
            include_prerequisites: true,
            include_details: true,
            exported_on: String::new(),
            labels: DocumentLabels::default(),
            theme: "auto".into(),
        }
    }
}

fn default_true() -> bool {
    true
}

/// The parts of the line under the title, e.g. `["Exported 2026-11-23",
/// "Version 1.0", "Author Erik"]`. Version and author are left out while empty.
/// The labels come from Settings for the guide's language.
pub(crate) fn detail_parts(project: &Project, options: &ExportOptions) -> Vec<String> {
    if !options.include_details {
        return Vec::new();
    }
    let labels = &options.labels;
    let mut parts = Vec::new();
    if !options.exported_on.is_empty() {
        parts.push(format!("{} {}", labels.exported(), options.exported_on));
    }
    if !project.version.trim().is_empty() {
        parts.push(format!("{} {}", labels.version(), project.version.trim()));
    }
    if !project.author.trim().is_empty() {
        parts.push(format!("{} {}", labels.author(), project.author.trim()));
    }
    parts
}

pub(crate) const DETAIL_SEPARATOR: &str = " · ";

/// A step flattened into exactly what a writer needs: final text, final pixels.
pub struct RenderedStep {
    pub number: usize,
    pub title: String,
    pub body: String,
    pub image: Option<RenderedImage>,
}

pub struct RenderedImage {
    pub png: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportResult {
    pub path: String,
    /// Present for Markdown, which writes images alongside the document.
    pub assets_dir: Option<String>,
    pub bytes: u64,
}

/// Bake annotations, resize, and fill in the blanks that the user left empty.
///
/// Every writer goes through here so a Markdown export and a PDF export can
/// never disagree about what the document contains.
pub fn prepare(
    app: &AppHandle,
    project: &Project,
    options: &ExportOptions,
) -> Result<Vec<RenderedStep>> {
    let frames = storage::frames_dir(app, &project.id)?;
    let mut out = Vec::new();

    for step in project.steps.iter().filter(|s| s.include) {
        let number = out.len() + 1;
        let image = if options.include_images {
            let path = frames.join(&step.frame);
            match imaging::load(&path) {
                Ok(img) => {
                    let img = imaging::render_frame(img, &step.annotations, step.crop);
                    let img = imaging::fit_width(img, options.image_width);
                    Some(RenderedImage {
                        width: img.width(),
                        height: img.height(),
                        png: imaging::encode_png(&img)?,
                    })
                }
                // A missing frame shouldn't sink the whole export; the step's
                // text is still worth having.
                Err(_) => None,
            }
        } else {
            None
        };

        out.push(RenderedStep {
            number,
            title: if step.title.trim().is_empty() {
                format!("Step {number}")
            } else {
                step.title.trim().to_string()
            },
            body: step.body.trim().to_string(),
            image,
        });
    }

    if out.is_empty() {
        return Err(AppError::Invalid(
            "This document has no steps to export. Include at least one step first.".into(),
        ));
    }
    Ok(out)
}

pub fn write(
    app: &AppHandle,
    project: &Project,
    options: &ExportOptions,
    destination: &Path,
) -> Result<ExportResult> {
    let steps = prepare(app, project, options)?;
    let mut options = options.clone();
    options.exported_on = chrono::Local::now().format("%Y-%m-%d").to_string();
    options.labels = {
        let settings = app.state::<AppState>().settings.lock().clone();
        let language = if project.language.trim().is_empty() {
            settings.language.clone()
        } else {
            project.language.clone()
        };
        settings.resolve_labels(&language).labels
    };
    let options = &options;
    if let Some(parent) = destination.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let assets_dir = match options.format {
        ExportFormat::Markdown => Some(markdown::write(project, &steps, options, destination)?),
        ExportFormat::Html => {
            std::fs::write(destination, html::render(project, &steps, options))?;
            None
        }
        ExportFormat::Pdf => {
            std::fs::write(destination, pdf::render(project, &steps, options)?)?;
            None
        }
    };

    Ok(ExportResult {
        path: destination.to_string_lossy().into_owned(),
        assets_dir: assets_dir.map(|p| p.to_string_lossy().into_owned()),
        bytes: std::fs::metadata(destination).map(|m| m.len()).unwrap_or(0),
    })
}

/// Suggested file name, e.g. "Creating a new invoice" -> "creating-a-new-invoice".
pub fn slugify(input: &str) -> String {
    let mut out = String::new();
    let mut last_dash = true;
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    let trimmed = out.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "walkmark-document".into()
    } else {
        trimmed
    }
}

pub fn default_file_name(project: &Project, format: ExportFormat) -> String {
    format!("{}.{}", slugify(&project.title), format.extension())
}

pub(crate) fn assets_dir_for(destination: &Path) -> PathBuf {
    let stem = destination
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "document".into());
    destination.with_file_name(format!("{stem}-images"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs_are_url_safe() {
        assert_eq!(slugify("Creating a New Invoice"), "creating-a-new-invoice");
        assert_eq!(slugify("  Spaces  &  symbols!! "), "spaces-symbols");
        assert_eq!(slugify("!!!"), "walkmark-document");
    }

    fn dated() -> ExportOptions {
        ExportOptions {
            exported_on: "2026-11-23".into(),
            ..Default::default()
        }
    }

    #[test]
    fn details_use_the_languages_labels() {
        let mut project = Project::new("Guida", "");
        project.author = "Erik".into();
        assert_eq!(
            detail_parts(&project, &dated()),
            ["Exported 2026-11-23", "Version 1.0", "Author Erik"]
        );

        project.version = "2.1".into();
        let italian = ExportOptions {
            labels: DocumentLabels::built_in("Italian").unwrap(),
            ..dated()
        };
        assert_eq!(
            detail_parts(&project, &italian),
            ["Esportato il 2026-11-23", "Versione 2.1", "Autore Erik"]
        );
    }

    #[test]
    fn details_skip_empty_fields_and_can_be_turned_off() {
        let mut project = Project::new("Guide", "");
        project.version = "  ".into();
        assert_eq!(detail_parts(&project, &dated()), ["Exported 2026-11-23"]);

        let off = ExportOptions {
            include_details: false,
            ..dated()
        };
        assert!(detail_parts(&project, &off).is_empty());
    }

    #[test]
    fn empty_labels_fall_back_to_english() {
        let labels = DocumentLabels {
            guide: "  ".into(),
            prerequisites: "Prima di iniziare".into(),
            ..Default::default()
        };
        assert_eq!(labels.guide(), "Step-by-step guide");
        assert_eq!(labels.prerequisites(), "Prima di iniziare");
    }

    #[test]
    fn older_projects_get_default_details() {
        let json = r#"{"id":"a","title":"Old","createdAt":"2026-01-01T00:00:00Z","updatedAt":"2026-01-01T00:00:00Z"}"#;
        let project: Project = serde_json::from_str(json).unwrap();
        assert_eq!(project.version, "1.0");
        assert_eq!(project.author, "");
    }
}
