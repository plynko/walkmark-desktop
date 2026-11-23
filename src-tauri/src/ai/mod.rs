pub mod catalog;
pub mod prompt;
pub mod provider;

use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::Semaphore;

use crate::error::{AppError, Result};
use crate::imaging;
use crate::models::{
    DocumentLabels, GenerationProgress, Project, Rect, Settings, Step, StepStatus,
};
use crate::state::AppState;
use crate::storage;

pub const EVT_PROGRESS: &str = "ai:progress";
pub const EVT_STEP: &str = "ai:step";
pub const EVT_OUTLINE: &str = "ai:outline";
pub const EVT_DONE: &str = "ai:done";

/// Frames sent for the whole-task outline are small on purpose: we only need
/// enough detail to tell the steps apart, and 24 large images would be slow
/// and expensive for no benefit.
const OUTLINE_IMAGE_WIDTH: u32 = 640;
const OUTLINE_MAX_IMAGES: usize = 24;

/// Detail frames. Every provider downsamples somewhere above this, and 1280
/// keeps small UI labels legible.
const STEP_IMAGE_WIDTH: u32 = 1280;

/// Local models are slower and have far less context to spend, so they get
/// smaller pictures and fewer of them. Sending a 1280px screenshot to a 4B
/// model on a laptop is how you wait two minutes for a worse sentence.
const LOCAL_STEP_IMAGE_WIDTH: u32 = 896;
const LOCAL_OUTLINE_MAX_IMAGES: usize = 10;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct OutlineEvent {
    title: String,
    summary: String,
    prerequisites: Vec<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DoneEvent {
    cancelled: bool,
    succeeded: usize,
    failed: usize,
}

/// Which steps a run should touch.
pub enum Scope {
    /// Everything that has never been written, leaving edited steps alone.
    Missing,
    /// Everything included in the document, overwriting existing text.
    All,
    /// Specific steps, regardless of their lock state.
    Only(Vec<String>),
}

/// Optional provider/model for a single generation run without changing settings.
#[derive(Debug, Clone, Default)]
pub struct GenerationOverrides {
    pub provider: Option<crate::models::Provider>,
    pub model: Option<String>,
    pub language: Option<String>,
}

fn effective_settings(
    settings: &Settings,
    project: &Project,
    overrides: &GenerationOverrides,
) -> Settings {
    let mut effective = settings.clone();
    if let Some(language) = overrides
        .language
        .as_ref()
        .map(|l| crate::limits::clamp_trim(l, crate::limits::LANGUAGE))
        .filter(|l| !l.is_empty())
    {
        effective.language = language;
    } else if !project.language.trim().is_empty() {
        effective.language = crate::limits::clamp(&project.language, crate::limits::LANGUAGE);
    }
    effective
}

/// Builds a client from whatever the user has configured, so no caller needs to
/// know which provider is active or where its credentials live.
pub fn client_for_overrides(
    app: &AppHandle,
    settings: &Settings,
    overrides: &GenerationOverrides,
) -> Result<provider::Client> {
    let provider = overrides.provider.unwrap_or(settings.provider);
    let config = settings.config_for(provider);
    let model = overrides
        .model
        .as_ref()
        .map(|m| m.trim().to_string())
        .filter(|m| !m.is_empty())
        .unwrap_or(config.model);
    let api_key = if provider.needs_key() {
        storage::get_api_key(app, provider).ok_or(AppError::MissingApiKey)?
    } else {
        String::new()
    };
    provider::Client::new(provider, model, config.base_url, api_key)
}

pub async fn run(
    app: AppHandle,
    scope: Scope,
    cancel: Arc<AtomicBool>,
    overrides: GenerationOverrides,
) -> Result<()> {
    let state = app.state::<AppState>();
    let settings: Settings = state.settings.lock().clone();
    let provider = overrides.provider.unwrap_or(settings.provider);
    let local = provider.is_local();

    let project: Project = state
        .project
        .lock()
        .clone()
        .ok_or_else(|| AppError::NotFound("No document is open.".into()))?;

    let settings = effective_settings(&settings, &project, &overrides);

    let targets = select_targets(&project, &scope);
    if targets.is_empty() {
        emit_done(&app, false, 0, 0);
        return Ok(());
    }

    let client = Arc::new(client_for_overrides(&app, &settings, &overrides)?);
    let vocabulary = settings
        .resolve_product(project.product_id.as_deref())
        .map(|p| p.format_vocabulary())
        .filter(|text| !text.is_empty());
    let system = prompt::system_instruction(&settings, vocabulary.as_deref());
    let frames_dir = storage::frames_dir(&app, &project.id)?;

    let total = targets.len();
    emit_progress(&app, 0, total, true, Some("Reviewing the recording".into()));

    // --- Pass 1: understand the whole task -------------------------------
    let write_outline = matches!(scope, Scope::All | Scope::Missing) && total > 1;
    let outline = if write_outline {
        // Dropping the request future closes the connection, so a stop request
        // ends a long outline call straight away instead of after it finishes.
        let built = tokio::select! {
            built = build_outline(&client, &system, &project, &frames_dir, local) => built,
            () = stop_requested(&cancel) => {
                emit_progress(&app, 0, total, false, None);
                emit_done(&app, true, 0, 0);
                return Ok(());
            }
        };
        match built {
            Ok(o) => Some(o),
            Err(e) => {
                // An outline is a quality boost, not a requirement — a failure
                // here shouldn't cost the user the whole run.
                emit_progress(
                    &app,
                    0,
                    total,
                    true,
                    Some(format!("Continuing without an overview ({e})")),
                );
                None
            }
        }
    } else {
        None
    };

    let included: Vec<String> = project
        .steps
        .iter()
        .filter(|s| s.include)
        .map(|s| s.id.clone())
        .collect();

    if let Some(outline) = &outline {
        let mut guard = state.project.lock();
        if let Some(p) = guard.as_mut() {
            if !outline.title.trim().is_empty() {
                p.title = outline.title.trim().to_string();
            }
            p.summary = outline.summary.trim().to_string();
            p.prerequisites = outline
                .prerequisites
                .iter()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            p.touch();
            let _ = storage::save_project(&app, p);
            let _ = app.emit(
                EVT_OUTLINE,
                OutlineEvent {
                    title: p.title.clone(),
                    summary: p.summary.clone(),
                    prerequisites: p.prerequisites.clone(),
                },
            );
        }
    }

    // Planned titles give every parallel worker the same view of what comes
    // before it, which is what keeps the finished document coherent.
    let planned: Vec<(String, String)> = match &outline {
        Some(o) => included
            .iter()
            .cloned()
            .zip(o.steps.iter().cloned())
            .collect(),
        None => Vec::new(),
    };
    let planned_title = |id: &str| -> String {
        planned
            .iter()
            .find(|(sid, _)| sid == id)
            .map(|(_, t)| t.clone())
            .unwrap_or_default()
    };

    let task_title = state
        .project
        .lock()
        .as_ref()
        .map(|p| p.title.clone())
        .unwrap_or_default();

    // --- Pass 2: write each step ----------------------------------------
    mark_queued(&app, &targets);
    let target_ids = targets.clone();

    // One local model serves one request at a time no matter how many we send,
    // so parallelism there only buys queueing and memory pressure.
    let concurrency = if local {
        1
    } else {
        settings.concurrency.clamp(1, 8)
    };
    let permits = Arc::new(Semaphore::new(concurrency));
    let mut set = tokio::task::JoinSet::new();

    let mut step_inputs = std::collections::HashMap::new();
    for step_id in targets {
        let Some((position, step)) = position_of(&state, &step_id) else {
            continue;
        };

        let earlier: Vec<String> = included
            .iter()
            .take_while(|id| **id != step_id)
            .map(|id| {
                let t = planned_title(id);
                if t.is_empty() {
                    title_of(&state, id)
                } else {
                    t
                }
            })
            .filter(|t| !t.is_empty())
            .collect();
        // Only the nearest few matter; sending twenty makes the prompt long
        // without making the writing better.
        let preceding: Vec<String> = earlier
            .iter()
            .skip(earlier.len().saturating_sub(4))
            .cloned()
            .collect();

        let draft = (matches!(scope, Scope::Only(_))
            && (!step.title.trim().is_empty() || !step.body.trim().is_empty()))
        .then_some((step.title.as_str(), step.body.as_str()));
        let text_prompt = prompt::step_prompt(
            position,
            included.len().max(1),
            &task_title,
            &planned_title(&step_id),
            &preceding,
            draft,
        );
        step_inputs.insert(step_id.clone(), (step.title, step.body));

        let client = Arc::clone(&client);
        let permits = Arc::clone(&permits);
        let cancel = Arc::clone(&cancel);
        let system = system.clone();
        let frame_path = frames_dir.join(&step.frame);
        let annotations = step.annotations;
        let crop = step.crop;

        set.spawn(async move {
            let _permit = permits.acquire().await;
            if cancel.load(Ordering::SeqCst) {
                return (step_id, Err(AppError::Cancelled));
            }

            let width = if local {
                LOCAL_STEP_IMAGE_WIDTH
            } else {
                STEP_IMAGE_WIDTH
            };
            let image = match prepare_image(&frame_path, &annotations, crop, width) {
                Ok(bytes) => bytes,
                Err(e) => return (step_id, Err(e)),
            };

            let result = client.describe(&system, &text_prompt, image).await;
            (step_id, result)
        });
    }

    let mut done = 0usize;
    let mut failed = 0usize;
    let mut finished: HashSet<String> = HashSet::new();
    let mut stopping = false;
    loop {
        let joined = tokio::select! {
            joined = set.join_next() => joined,
            // Abort in-flight requests the moment a stop is asked for, rather
            // than waiting for the slowest one (a local model can take minutes).
            () = stop_requested(&cancel), if !stopping => {
                stopping = true;
                set.abort_all();
                continue;
            }
        };
        let Some(joined) = joined else {
            break;
        };
        let Ok((step_id, result)) = joined else {
            continue;
        };
        finished.insert(step_id.clone());
        done += 1;

        let updated = {
            let mut guard = state.project.lock();
            let Some(project) = guard.as_mut() else {
                break;
            };
            let Some(step) = project.steps.iter_mut().find(|s| s.id == step_id) else {
                continue;
            };
            match result {
                Ok(text) => {
                    if let Some(source) = step_inputs.remove(&step_id) {
                        apply_written_step(step, source, text);
                    }
                }
                Err(AppError::Cancelled) => restore_unwritten(step),
                Err(e) => {
                    failed += 1;
                    step.status = StepStatus::Failed;
                    step.error = Some(e.to_string());
                }
            }
            let updated = step.clone();
            project.touch();
            updated
        };

        let _ = app.emit(EVT_STEP, &updated);
        emit_progress(&app, done, total, true, None);

        // Persist as we go: a crash or a quit halfway through a long run should
        // never throw away work the user already paid for.
        if let Some(project) = state.project.lock().as_ref() {
            let _ = storage::save_project(&app, project);
        }
    }

    let cancelled = cancel.load(Ordering::SeqCst);
    if cancelled {
        // Aborted requests never report back, so put their steps back
        // ourselves; otherwise they would stay "queued" forever.
        release_unfinished(&app, &target_ids, &finished);
    }
    emit_progress(&app, done, total, false, None);
    emit_done(&app, cancelled, done.saturating_sub(failed), failed);
    Ok(())
}

/// The guide's fixed labels in `language`. English needs no request: empty
/// labels already mean the English defaults.
pub async fn translate_labels(
    app: &AppHandle,
    language: &str,
    overrides: &GenerationOverrides,
) -> Result<DocumentLabels> {
    let language = crate::limits::clamp_trim(language, crate::limits::LANGUAGE);
    if language.is_empty() || language.eq_ignore_ascii_case("english") {
        return Ok(DocumentLabels::default());
    }

    let settings: Settings = app.state::<AppState>().settings.lock().clone();
    let client = client_for_overrides(app, &settings, overrides)?;
    let translated = client.translate_labels(&language).await?;
    Ok(translated.map(|text| crate::limits::clamp_trim(text, crate::limits::DOCUMENT_LABEL)))
}

/// Resolves once the run has been asked to stop.
async fn stop_requested(cancel: &AtomicBool) {
    while !cancel.load(Ordering::SeqCst) {
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

/// A step whose request was abandoned goes back to what it was before the run:
/// untouched if it never had text, otherwise still holding the text it had.
fn restore_unwritten(step: &mut Step) {
    step.status = if step.title.trim().is_empty() && step.body.trim().is_empty() {
        StepStatus::Draft
    } else {
        StepStatus::Ready
    };
    step.error = None;
}

fn release_unfinished(app: &AppHandle, targets: &[String], finished: &HashSet<String>) {
    let state = app.state::<AppState>();
    let mut guard = state.project.lock();
    let Some(project) = guard.as_mut() else {
        return;
    };
    let mut changed = false;
    for step in project.steps.iter_mut() {
        if targets.contains(&step.id)
            && !finished.contains(&step.id)
            && matches!(step.status, StepStatus::Queued | StepStatus::Generating)
        {
            restore_unwritten(step);
            let _ = app.emit(EVT_STEP, &step.clone());
            changed = true;
        }
    }
    if changed {
        project.touch();
        let _ = storage::save_project(app, project);
    }
}

fn apply_written_step(step: &mut Step, source: (String, String), text: provider::StepText) {
    // Leave edits made after the request started in place.
    if step.title == source.0 && step.body == source.1 {
        let title = text.title.trim().trim_end_matches('.').trim_end();
        let body = text.body.trim();
        if !title.is_empty() {
            step.title = title.to_owned();
        }
        if !body.is_empty() {
            step.body = body.to_owned();
        }
    }
    step.status = StepStatus::Ready;
    step.error = None;
}

fn select_targets(project: &Project, scope: &Scope) -> Vec<String> {
    match scope {
        Scope::Only(ids) => project
            .steps
            .iter()
            .filter(|s| ids.contains(&s.id))
            .map(|s| s.id.clone())
            .collect(),
        Scope::All => project
            .steps
            .iter()
            .filter(|s| s.include && !s.locked)
            .map(|s| s.id.clone())
            .collect(),
        Scope::Missing => project
            .steps
            .iter()
            .filter(|s| s.include && !s.locked && s.status != StepStatus::Ready)
            .map(|s| s.id.clone())
            .collect(),
    }
}

async fn build_outline(
    client: &provider::Client,
    system: &str,
    project: &Project,
    frames_dir: &std::path::Path,
    local: bool,
) -> Result<provider::Outline> {
    let included: Vec<&Step> = project.steps.iter().filter(|s| s.include).collect();
    let budget = if local {
        LOCAL_OUTLINE_MAX_IMAGES
    } else {
        OUTLINE_MAX_IMAGES
    };
    // Long recordings get an evenly spread sample rather than just the first N,
    // so the outline still reflects the end of the task.
    let stride = (included.len() as f32 / budget as f32).ceil().max(1.0) as usize;
    let sampled: Vec<&Step> = included.iter().copied().step_by(stride).collect();

    let mut images = Vec::with_capacity(sampled.len());
    for step in &sampled {
        images.push(prepare_image(
            &frames_dir.join(&step.frame),
            &step.annotations,
            step.crop,
            OUTLINE_IMAGE_WIDTH,
        )?);
    }

    let mut outline = client
        .outline(system, &prompt::outline_prompt(images.len()), images)
        .await?;

    // When we sampled, the returned titles line up with the sample, not with
    // every step; stretch them back out so downstream indexing stays honest.
    if stride > 1 {
        let mut expanded = Vec::with_capacity(included.len());
        for i in 0..included.len() {
            expanded.push(outline.steps.get(i / stride).cloned().unwrap_or_default());
        }
        outline.steps = expanded;
    }
    outline.steps.resize(included.len(), String::new());
    Ok(outline)
}

fn prepare_image(
    path: &std::path::Path,
    annotations: &[crate::models::Annotation],
    crop: Option<Rect>,
    width: u32,
) -> Result<provider::InlineImage> {
    let image = imaging::load(path)?;
    // Annotations are burned in first so redacted regions never leave the
    // machine, then the result is shrunk for the request.
    let image = imaging::render_frame(image, annotations, crop);
    let image = imaging::fit_width(image, width);
    Ok(provider::InlineImage(imaging::encode_jpeg(&image, 82)?))
}

fn position_of(state: &AppState, step_id: &str) -> Option<(usize, Step)> {
    let guard = state.project.lock();
    let project = guard.as_ref()?;
    let included: Vec<&Step> = project.steps.iter().filter(|s| s.include).collect();
    let position = included.iter().position(|s| s.id == step_id)?;
    Some((position, included[position].clone()))
}

fn title_of(state: &AppState, step_id: &str) -> String {
    state
        .project
        .lock()
        .as_ref()
        .and_then(|p| p.steps.iter().find(|s| s.id == step_id))
        .map(|s| s.title.clone())
        .unwrap_or_default()
}

/// Flip every target to "queued" up front so the list shows the full run
/// immediately instead of revealing it one row at a time.
fn mark_queued(app: &AppHandle, targets: &[String]) {
    let state = app.state::<AppState>();
    let mut guard = state.project.lock();
    let Some(project) = guard.as_mut() else {
        return;
    };
    for step in project.steps.iter_mut() {
        if targets.contains(&step.id) {
            step.status = StepStatus::Queued;
            step.error = None;
            let _ = app.emit(EVT_STEP, &step.clone());
        }
    }
}

fn emit_progress(
    app: &AppHandle,
    done: usize,
    total: usize,
    running: bool,
    message: Option<String>,
) {
    let _ = app.emit(
        EVT_PROGRESS,
        GenerationProgress {
            done,
            total,
            running,
            message,
        },
    );
}

fn emit_done(app: &AppHandle, cancelled: bool, succeeded: usize, failed: usize) {
    let _ = app.emit(
        EVT_DONE,
        DoneEvent {
            cancelled,
            succeeded,
            failed,
        },
    );
}

#[cfg(test)]
mod step_polish_tests {
    use super::*;

    #[test]
    fn polishing_keeps_locked_steps_and_does_not_clear_an_empty_response_field() {
        let mut step = Step::new("frame.png".into(), 0, true);
        step.title = "Job settings".into();
        step.body = "Use Q4-EMEA and leave retries off.".into();
        step.locked = true;
        let source = (step.title.clone(), step.body.clone());

        apply_written_step(
            &mut step,
            source,
            provider::StepText {
                title: "Configure the job.".into(),
                body: String::new(),
            },
        );
        assert_eq!(step.title, "Configure the job");
        assert_eq!(step.body, "Use Q4-EMEA and leave retries off.");
        assert!(step.locked);
    }

    #[test]
    fn an_ai_response_does_not_overwrite_more_recent_edits() {
        let mut step = Step::new("frame.png".into(), 0, true);
        step.title = "Configure the job".into();
        let source = (step.title.clone(), step.body.clone());
        step.body = "New note: keep retries off.".into();
        apply_written_step(
            &mut step,
            source,
            provider::StepText {
                title: "Configure retries".into(),
                body: "Enable retries.".into(),
            },
        );
        assert_eq!(step.title, "Configure the job");
        assert_eq!(step.body, "New note: keep retries off.");
    }

    #[test]
    fn a_stopped_step_keeps_its_text_or_returns_to_draft() {
        let mut empty = Step::new("a.png".into(), 0, false);
        empty.status = StepStatus::Queued;
        restore_unwritten(&mut empty);
        assert_eq!(empty.status, StepStatus::Draft);

        let mut written = Step::new("b.png".into(), 0, false);
        written.body = "Click Save.".into();
        written.status = StepStatus::Generating;
        written.error = Some("old".into());
        restore_unwritten(&mut written);
        assert_eq!(written.status, StepStatus::Ready);
        assert_eq!(written.error, None);
        assert_eq!(written.body, "Click Save.");
    }

    #[tokio::test]
    async fn a_stop_request_interrupts_a_pending_call() {
        let cancel = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&cancel);
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            flag.store(true, Ordering::SeqCst);
        });

        let outcome = tokio::time::timeout(Duration::from_secs(5), async {
            tokio::select! {
                () = std::future::pending::<()>() => "finished",
                () = stop_requested(&cancel) => "stopped",
            }
        })
        .await;
        assert_eq!(outcome, Ok("stopped"));
    }
}
