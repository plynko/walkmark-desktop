use std::path::{Path, PathBuf};

use crate::error::Result;
use crate::models::Project;

use super::{ExportOptions, RenderedStep};

/// Writes `<name>.md` plus a sibling `<name>-images/` directory.
///
/// Relative image links are what make the output portable into a wiki, a repo
/// or a static site generator without rewriting anything.
pub fn write(
    project: &Project,
    steps: &[RenderedStep],
    options: &ExportOptions,
    destination: &Path,
) -> Result<PathBuf> {
    let assets = super::assets_dir_for(destination);
    let assets_name = assets
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "images".into());

    if options.include_images {
        std::fs::create_dir_all(&assets)?;
    }

    let mut doc = String::new();
    doc.push_str(&format!("# {}\n\n", project.title.trim()));

    let details = super::detail_parts(project, options);
    if !details.is_empty() {
        doc.push_str(&details.join(super::DETAIL_SEPARATOR));
        doc.push_str("\n\n");
    }

    if options.include_summary && !project.summary.trim().is_empty() {
        doc.push_str(project.summary.trim());
        doc.push_str("\n\n");
    }

    if options.include_prerequisites && !project.prerequisites.is_empty() {
        doc.push_str(&format!("## {}\n\n", options.labels.prerequisites()));
        for item in &project.prerequisites {
            doc.push_str(&format!("- {}\n", item.trim()));
        }
        doc.push('\n');
    }

    if options.include_toc && steps.len() > 2 {
        doc.push_str(&format!("## {}\n\n", options.labels.steps()));
        for step in steps {
            doc.push_str(&format!(
                "{}. [{}](#{})\n",
                step.number,
                escape(&step.title),
                anchor(step)
            ));
        }
        doc.push('\n');
    }

    for step in steps {
        doc.push_str(&format!("## {}. {}\n\n", step.number, escape(&step.title)));

        if !step.body.is_empty() {
            doc.push_str(&step.body);
            doc.push_str("\n\n");
        }

        if let Some(image) = &step.image {
            let file = format!("step-{:02}.png", step.number);
            std::fs::write(assets.join(&file), &image.png)?;
            doc.push_str(&format!(
                "![{}]({}/{})\n\n",
                escape(&step.title),
                assets_name,
                file
            ));
        }
    }

    doc.push_str(&format!("---\n\n_{}._\n", options.labels.generated_with()));
    std::fs::write(destination, doc)?;
    Ok(assets)
}

/// GitHub-style heading anchors, so the table of contents actually works.
fn anchor(step: &RenderedStep) -> String {
    super::slugify(&format!("{} {}", step.number, step.title))
}

/// Only the characters that would break a link label or start a heading.
fn escape(text: &str) -> String {
    text.replace('[', "\\[").replace(']', "\\]")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::DocumentLabels;

    #[test]
    fn headings_and_credit_use_the_languages_wording() {
        let dir = std::env::temp_dir().join(format!("walkmark-md-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let destination = dir.join("guida.md");

        let mut project = Project::new("Guida", "");
        project.prerequisites = vec!["Un account".into()];
        let steps: Vec<RenderedStep> = (1..=3)
            .map(|number| RenderedStep {
                number,
                title: format!("Passo {number}"),
                body: String::new(),
                image: None,
            })
            .collect();
        let options = ExportOptions {
            include_images: false,
            labels: DocumentLabels::built_in("Italian").unwrap(),
            exported_on: "2026-11-23".into(),
            ..Default::default()
        };
        write(&project, &steps, &options, &destination).unwrap();

        let text = std::fs::read_to_string(&destination).unwrap();
        std::fs::remove_dir_all(&dir).ok();
        assert!(text.contains("2026-11-23 · Versione 1.0"));
        assert!(text.contains("## Prima di iniziare"));
        assert!(text.contains("## Passaggi"));
        assert!(text.contains("_Generato con Walkmark._"));
        assert!(!text.contains("Generated with"));
    }
}
