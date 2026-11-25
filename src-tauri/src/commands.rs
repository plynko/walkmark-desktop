use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_dialog::DialogExt;

use crate::ai;
use crate::archive;
use crate::capture::{self, session::Session};
use crate::error::{AppError, Result};
use crate::export::{self, ExportOptions, ExportResult};
use crate::imaging;
use crate::limits;
use crate::local;
use crate::models::*;
use crate::state::AppState;
use crate::storage;
use crate::window;

// ---------------------------------------------------------------------------
// Settings & credentials
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn get_settings(state: State<AppState>) -> Settings {
    state.settings.lock().clone()
}

#[tauri::command]
pub fn save_settings(
    app: AppHandle,
    state: State<AppState>,
    settings: Settings,
) -> Result<Settings> {
    let mut settings = settings;
    settings.concurrency = settings.concurrency.clamp(1, 8);
    settings.capture.sample_interval_ms = settings.capture.sample_interval_ms.clamp(200, 5000);
    settings.capture.sensitivity = settings.capture.sensitivity.clamp(0.0, 1.0);
    settings.capture.min_gap_ms = settings.capture.min_gap_ms.clamp(0, 30_000);
    settings.capture.input_settle_ms = settings.capture.input_settle_ms.clamp(0, 2000);
    settings.capture.max_width = settings.capture.max_width.clamp(800, 4000);
    settings.capture.countdown_secs = settings.capture.countdown_secs.min(10);
    settings.normalize_products();

    storage::save_settings(&app, &settings)?;
    *state.settings.lock() = settings.clone();
    Ok(settings)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionStatus {
    pub granted: bool,
    /// macOS is the only platform where this gate exists today.
    pub required: bool,
    pub input_granted: bool,
    pub input_required: bool,
}

#[tauri::command]
pub fn permission_status() -> PermissionStatus {
    PermissionStatus {
        granted: capture::has_permission(),
        required: cfg!(target_os = "macos"),
        input_granted: capture::input::has_input_permission(),
        input_required: cfg!(target_os = "macos"),
    }
}

#[tauri::command]
pub fn request_permission() -> PermissionStatus {
    capture::request_permission();
    capture::input::request_input_permission();
    permission_status()
}

/// Everything the model picker needs to render itself: who the providers are,
/// which models they offer, and which of them we already hold a key for.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderCatalog {
    pub providers: Vec<ai::catalog::ProviderInfo>,
    /// Provider ids with a credential stored, so the UI can show what's ready.
    pub configured: Vec<&'static str>,
}

#[tauri::command]
pub fn provider_catalog(app: AppHandle) -> ProviderCatalog {
    ProviderCatalog {
        providers: ai::catalog::all_providers(),
        configured: storage::providers_with_keys(&app)
            .into_iter()
            .map(|p| p.id())
            .collect(),
    }
}

fn provider_from(id: &str) -> Result<Provider> {
    Provider::parse(id).ok_or_else(|| AppError::Invalid(format!("Unknown provider `{id}`.")))
}

#[tauri::command]
pub fn set_api_key(app: AppHandle, provider: String, key: String) -> Result<()> {
    storage::set_api_key(&app, provider_from(&provider)?, &key)
}

/// Round-trips a trivial request so Settings can confirm the setup works before
/// the user commits to a long generation run.
///
/// `key` lets the dialog verify something the user just typed without saving it
/// first; without it we check whatever is already stored.
#[tauri::command]
pub async fn verify_provider(
    app: AppHandle,
    state: State<'_, AppState>,
    provider: String,
    model: Option<String>,
    base_url: Option<String>,
    key: Option<String>,
) -> Result<()> {
    let provider = provider_from(&provider)?;
    let settings = state.settings.lock().clone();
    let saved = settings.config_for(provider);

    let key = match key
        .map(|k| limits::clamp_trim(&k, limits::API_KEY))
        .filter(|k| !k.is_empty())
    {
        Some(key) => key,
        None if provider.needs_key() => {
            storage::get_api_key(&app, provider).ok_or(AppError::MissingApiKey)?
        }
        None => String::new(),
    };

    let pick_model = |given: Option<String>, fallback: String| {
        given
            .map(|v| limits::clamp_trim(&v, limits::MODEL_ID))
            .filter(|v| !v.is_empty())
            .unwrap_or(fallback)
    };
    let pick_url = |given: Option<String>, fallback: String| {
        given
            .map(|v| limits::clamp_trim(&v, limits::BASE_URL))
            .filter(|v| !v.is_empty())
            .unwrap_or(fallback)
    };

    ai::provider::Client::new(
        provider,
        pick_model(model, saved.model),
        pick_url(base_url, saved.base_url),
        key,
    )?
    .check()
    .await
}

// ---------------------------------------------------------------------------
// Local models
// ---------------------------------------------------------------------------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalOverview {
    #[serde(flatten)]
    pub status: local::LocalStatus,
    pub catalog: Vec<local::CatalogEntry>,
    /// The model currently being downloaded, if any.
    pub downloading: Option<String>,
}

fn ollama_endpoint(state: &AppState) -> String {
    state.settings.lock().config_for(Provider::Ollama).base_url
}

#[tauri::command]
pub async fn local_status(state: State<'_, AppState>) -> Result<LocalOverview> {
    let endpoint = ollama_endpoint(&state);
    let downloading = state.download.lock().as_ref().map(|d| d.model.clone());
    let status = local::status(&endpoint).await;
    let catalog = local::catalog(&status.models, status.total_memory);
    Ok(LocalOverview {
        status,
        catalog,
        downloading,
    })
}

#[tauri::command]
pub fn download_model(app: AppHandle, state: State<AppState>, model: String) -> Result<()> {
    if let Some(active) = state.download.lock().as_ref() {
        return Err(AppError::Invalid(format!(
            "`{}` is still downloading. Wait for it to finish first.",
            active.model
        )));
    }

    let endpoint = ollama_endpoint(&state);
    let cancel = Arc::new(AtomicBool::new(false));
    *state.download.lock() = Some(crate::state::Download {
        model: model.clone(),
        cancel: Arc::clone(&cancel),
    });

    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        let result = local::pull(handle.clone(), &endpoint, &model, cancel).await;
        handle.state::<AppState>().download.lock().take();
        if let Err(e) = result {
            if !matches!(e, AppError::Cancelled) {
                let payload = serde_json::to_value(&e).unwrap_or_default();
                let _ = handle.emit("local:error", payload);
            }
        }
    });
    Ok(())
}

#[tauri::command]
pub fn cancel_download(state: State<AppState>) {
    if let Some(active) = state.download.lock().as_ref() {
        active.cancel.store(true, Ordering::SeqCst);
    }
}

#[tauri::command]
pub async fn remove_model(state: State<'_, AppState>, model: String) -> Result<()> {
    let endpoint = ollama_endpoint(&state);
    local::remove(&endpoint, &model).await
}

// ---------------------------------------------------------------------------
// Capture sources
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn list_sources(with_thumbnails: bool) -> Result<Vec<CaptureSource>> {
    // Screenshotting every window is slow enough to block the UI thread.
    tauri::async_runtime::spawn_blocking(move || capture::list_sources(with_thumbnails))
        .await
        .map_err(|e| AppError::Other(e.to_string()))?
}

// ---------------------------------------------------------------------------
// Recording
// ---------------------------------------------------------------------------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordingStatus {
    pub active: bool,
    pub state: RecordingState,
    pub step_count: usize,
}

#[tauri::command]
pub fn recording_status(state: State<AppState>) -> RecordingStatus {
    match state.session.lock().as_ref() {
        Some(session) => RecordingStatus {
            active: true,
            state: session.state(),
            step_count: session.step_count(),
        },
        None => RecordingStatus {
            active: false,
            state: RecordingState::Idle,
            step_count: 0,
        },
    }
}

#[tauri::command]
pub async fn start_recording(
    app: AppHandle,
    state: State<'_, AppState>,
    source_id: String,
    product_id: Option<String>,
) -> Result<Project> {
    if state.session.lock().is_some() {
        return Err(AppError::AlreadyRecording);
    }

    let settings = state.settings.lock().clone();
    if let Some(id) = product_id.as_deref() {
        if settings.product(id).is_none() {
            return Err(AppError::Invalid(format!("Unknown product `{id}`.")));
        }
    }

    let mut project = Project::new("Untitled guide", "");
    project.product_id = product_id;
    project.language = settings.language.clone();
    let frames_dir = storage::frames_dir(&app, &project.id)?;

    let session = Session::start(
        app.clone(),
        crate::capture::session::SessionConfig {
            source_id,
            frames_dir,
            settings: settings.capture.clone(),
        },
    )?;

    project.source_label = session.source_label.clone();
    storage::save_project(&app, &project)?;

    *state.project.lock() = Some(project.clone());
    *state.session.lock() = Some(session);

    window::enter_recording_mode(&app, settings.capture.hide_window);
    Ok(project)
}

#[tauri::command]
pub fn pause_recording(state: State<AppState>, paused: bool) -> Result<()> {
    let guard = state.session.lock();
    let session = guard.as_ref().ok_or(AppError::NotRecording)?;
    session.set_paused(paused);
    Ok(())
}

#[tauri::command]
pub fn mark_step(state: State<AppState>) -> Result<()> {
    let guard = state.session.lock();
    guard.as_ref().ok_or(AppError::NotRecording)?.mark();
    Ok(())
}

#[tauri::command]
pub async fn stop_recording(app: AppHandle, state: State<'_, AppState>) -> Result<Project> {
    let session = state.session.lock().take().ok_or(AppError::NotRecording)?;

    // Tell both windows immediately — before the capture thread joins.
    let _ = app.emit(crate::capture::session::EVT_TICK, session.stopping_tick());

    // Sync commands run on the main thread on Windows; joining the capture
    // worker there can deadlock against window and event dispatch, so the
    // blocking wait happens on a worker thread instead.
    let _ = tauri::async_runtime::spawn_blocking(move || session.stop()).await;
    window::leave_recording_mode(&app);

    // Steps arrive as events during the session; the frontend replays them here
    // so the backend's copy is the authoritative, ordered list.
    let project = state
        .project
        .lock()
        .clone()
        .ok_or_else(|| AppError::NotFound("The recording was lost.".into()))?;
    storage::save_project(&app, &project)?;
    let _ = app.emit(crate::capture::session::EVT_STOPPED, &project);
    Ok(project)
}

/// The capture thread emits steps as it finds them; this is how they get into
/// the authoritative project.
#[tauri::command]
pub fn append_step(app: AppHandle, state: State<AppState>, step: Step) -> Result<()> {
    let mut guard = state.project.lock();
    let project = guard
        .as_mut()
        .ok_or_else(|| AppError::NotFound("No document is open.".into()))?;
    if project.steps.iter().any(|s| s.id == step.id) {
        return Ok(());
    }
    project.steps.push(step);
    project.steps.sort_by_key(|s| s.offset_ms);
    project.touch();
    storage::save_project(&app, project)
}

#[tauri::command]
pub fn attach_alternate(
    app: AppHandle,
    state: State<AppState>,
    step_id: String,
    frame: String,
) -> Result<()> {
    let mut guard = state.project.lock();
    let project = guard
        .as_mut()
        .ok_or_else(|| AppError::NotFound("No document is open.".into()))?;
    if let Some(step) = project.steps.iter_mut().find(|s| s.id == step_id) {
        if !step.alternates.contains(&frame) && step.frame != frame {
            step.alternates.push(frame);
        }
    }
    storage::save_project(&app, project)
}

// ---------------------------------------------------------------------------
// Projects
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn list_projects(app: AppHandle) -> Result<Vec<ProjectSummary>> {
    storage::list_projects(&app)
}

/// Swapping the open document while the capture thread is still emitting steps
/// would file them against the wrong project — and their frames live in the
/// recording's directory, so they would come back as broken images.
#[tauri::command]
pub fn open_project(app: AppHandle, state: State<AppState>, id: String) -> Result<Project> {
    if state.session.lock().is_some() {
        return Err(AppError::AlreadyRecording);
    }
    let project = storage::load_project(&app, &id)?;
    *state.project.lock() = Some(project.clone());
    Ok(project)
}

#[tauri::command]
pub fn current_project(state: State<AppState>) -> Option<Project> {
    state.project.lock().clone()
}

#[tauri::command]
pub fn close_project(state: State<AppState>) {
    if state.session.lock().is_some() {
        return;
    }
    *state.project.lock() = None;
}

#[tauri::command]
pub fn delete_project(app: AppHandle, state: State<AppState>, id: String) -> Result<()> {
    if state.project.lock().as_ref().map(|p| p.id.clone()) == Some(id.clone()) {
        *state.project.lock() = None;
    }
    storage::delete_project(&app, &id)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectMeta {
    pub title: Option<String>,
    pub summary: Option<String>,
    pub prerequisites: Option<Vec<String>>,
    pub language: Option<String>,
    pub version: Option<String>,
    pub author: Option<String>,
}

#[tauri::command]
pub fn update_project_meta(
    app: AppHandle,
    state: State<AppState>,
    meta: ProjectMeta,
) -> Result<Project> {
    let mut guard = state.project.lock();
    let project = guard
        .as_mut()
        .ok_or_else(|| AppError::NotFound("No document is open.".into()))?;

    if let Some(title) = meta.title {
        let title = limits::clamp_trim(&title, limits::DOCUMENT_TITLE);
        project.title = if title.is_empty() {
            "Untitled guide".into()
        } else {
            title
        };
    }
    if let Some(summary) = meta.summary {
        project.summary = limits::clamp_trim(&summary, limits::DOCUMENT_SUMMARY);
    }
    if let Some(prerequisites) = meta.prerequisites {
        project.prerequisites = prerequisites
            .into_iter()
            .map(|p| limits::clamp_trim(&p, limits::PREREQUISITE))
            .filter(|p| !p.is_empty())
            .take(limits::PREREQUISITES_MAX)
            .collect();
    }
    if let Some(language) = meta.language {
        project.language = limits::clamp_trim(&language, limits::LANGUAGE);
    }
    if let Some(version) = meta.version {
        project.version = limits::clamp_trim(&version, limits::DOCUMENT_VERSION);
    }
    if let Some(author) = meta.author {
        project.author = limits::clamp_trim(&author, limits::DOCUMENT_AUTHOR);
    }
    project.touch();
    storage::save_project(&app, project)?;
    Ok(project.clone())
}

/// Absolute path for `convertFileSrc`. Serving frames over the asset protocol
/// keeps multi-megabyte screenshots off the IPC bridge entirely.
#[tauri::command]
pub fn frame_path(app: AppHandle, project_id: String, frame: String) -> Result<String> {
    Ok(storage::frame_path(&app, &project_id, &frame)?
        .to_string_lossy()
        .into_owned())
}

// ---------------------------------------------------------------------------
// Steps
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn preview_markdown(text: String) -> String {
    crate::guide_markdown::render(&limits::clamp(&text, limits::STEP_BODY))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportedStep {
    project: Project,
    step_id: String,
}

#[tauri::command]
pub async fn import_step(
    app: AppHandle,
    project_id: String,
    after_step_id: Option<String>,
) -> Result<Option<ImportedStep>> {
    tauri::async_runtime::spawn_blocking(move || {
        let Some(file) = app
            .dialog()
            .file()
            .set_title("Add screenshot to guide")
            .add_filter("Screenshots", &["png", "jpg", "jpeg"])
            .blocking_pick_file()
        else {
            return Ok(None);
        };
        let path = file
            .into_path()
            .map_err(|_| AppError::Invalid("Choose a local screenshot file.".into()))?;
        let image = imaging::load_screenshot(&path)?;
        insert_step_image(&app, &project_id, after_step_id.as_deref(), image).map(Some)
    })
    .await
    .map_err(|error| AppError::Other(error.to_string()))?
}

#[tauri::command]
pub async fn capture_step(
    app: AppHandle,
    project_id: String,
    after_step_id: Option<String>,
    source_id: String,
) -> Result<ImportedStep> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        if state.session.lock().is_some() {
            return Err(AppError::AlreadyRecording);
        }
        if !capture::has_permission() {
            return Err(AppError::PermissionDenied);
        }
        if !state
            .project
            .lock()
            .as_ref()
            .is_some_and(|project| project.id == project_id)
        {
            return Err(AppError::NotFound("That guide is no longer open.".into()));
        }
        let target = capture::resolve(&source_id)?;
        let (countdown, max_width) = {
            let settings = state.settings.lock();
            (
                settings.capture.countdown_secs.clamp(1, 10),
                settings.capture.max_width,
            )
        };
        let main = app
            .get_webview_window(window::MAIN)
            .ok_or_else(|| AppError::NotFound("The Walkmark window is not open.".into()))?;
        main.hide()
            .map_err(|error| AppError::Other(error.to_string()))?;
        std::thread::sleep(std::time::Duration::from_secs(u64::from(countdown)));
        let screenshot = target.grab();
        let _ = main.show();
        let _ = main.set_focus();

        let image = capture::downscale(screenshot?, max_width);
        insert_step_image(&app, &project_id, after_step_id.as_deref(), image)
    })
    .await
    .map_err(|error| AppError::Other(error.to_string()))?
}

#[tauri::command]
pub async fn paste_step(
    app: AppHandle,
    project_id: String,
    after_step_id: Option<String>,
) -> Result<ImportedStep> {
    tauri::async_runtime::spawn_blocking(move || {
        let clipboard = app.clipboard().read_image().map_err(|_| {
            AppError::Invalid("Copy a screenshot or image to the clipboard first.".into())
        })?;
        let image = image::RgbaImage::from_raw(
            clipboard.width(),
            clipboard.height(),
            clipboard.rgba().to_vec(),
        )
        .ok_or_else(|| AppError::Invalid("The clipboard image could not be read.".into()))?;
        insert_step_image(&app, &project_id, after_step_id.as_deref(), image)
    })
    .await
    .map_err(|error| AppError::Other(error.to_string()))?
}

fn insert_step_image(
    app: &AppHandle,
    project_id: &str,
    after_step_id: Option<&str>,
    image: image::RgbaImage,
) -> Result<ImportedStep> {
    let png = imaging::encode_png(&image)?;
    let state = app.state::<AppState>();
    if state.session.lock().is_some() {
        return Err(AppError::AlreadyRecording);
    }
    let mut guard = state.project.lock();
    let project = guard
        .as_mut()
        .filter(|project| project.id == project_id)
        .ok_or_else(|| AppError::NotFound("That guide is no longer open.".into()))?;
    let frame = format!("manual-{}.png", uuid::Uuid::new_v4().simple());
    let frame_path = storage::frame_path(app, project_id, &frame)?;
    let mut updated = project.clone();
    let step_id = updated.insert_manual_step(frame, after_step_id);
    updated.touch();
    if let Err(error) = std::fs::write(&frame_path, png) {
        let _ = std::fs::remove_file(&frame_path);
        return Err(error.into());
    }
    if let Err(error) = storage::save_project(app, &updated) {
        let _ = std::fs::remove_file(&frame_path);
        return Err(error);
    }
    *project = updated;
    Ok(ImportedStep {
        project: project.clone(),
        step_id,
    })
}

fn deserialize_crop_patch<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<Option<Option<Rect>>, D::Error> {
    // An omitted crop leaves it unchanged; an explicit null resets it.
    Option::<Rect>::deserialize(deserializer).map(Some)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StepPatch {
    pub title: Option<String>,
    pub body: Option<String>,
    pub include: Option<bool>,
    pub locked: Option<bool>,
    pub frame: Option<String>,
    pub annotations: Option<Vec<Annotation>>,
    #[serde(default, deserialize_with = "deserialize_crop_patch")]
    pub crop: Option<Option<Rect>>,
}

#[tauri::command]
pub fn update_step(
    app: AppHandle,
    state: State<AppState>,
    id: String,
    patch: StepPatch,
) -> Result<Step> {
    let mut guard = state.project.lock();
    let project = guard
        .as_mut()
        .ok_or_else(|| AppError::NotFound("No document is open.".into()))?;
    let step = project
        .steps
        .iter_mut()
        .find(|s| s.id == id)
        .ok_or_else(|| AppError::NotFound("That step no longer exists.".into()))?;

    let mut text_edited = false;
    if let Some(title) = patch.title {
        step.title = limits::clamp_trim(&title, limits::STEP_TITLE);
        text_edited = true;
    }
    if let Some(body) = patch.body {
        step.body = limits::clamp_trim(&body, limits::STEP_BODY);
        text_edited = true;
    }
    if let Some(include) = patch.include {
        step.include = include;
    }
    if let Some(frame) = patch.frame {
        // Swapping in an alternate puts the old frame back in the pool so the
        // switch stays reversible.
        if step.frame != frame && step.alternates.contains(&frame) {
            let previous = std::mem::replace(&mut step.frame, frame.clone());
            step.alternates.retain(|f| *f != frame);
            step.alternates.push(previous);
            step.alternates.sort();
        }
    }
    if let Some(annotations) = patch.annotations {
        step.annotations = annotations;
    }
    if let Some(crop) = patch.crop {
        step.crop = crop;
    }
    if let Some(locked) = patch.locked {
        step.locked = locked;
    } else if text_edited {
        // Hand-written text is protected from the next "regenerate all".
        step.locked = true;
        if !step.title.is_empty() || !step.body.is_empty() {
            step.status = StepStatus::Ready;
            step.error = None;
        }
    }

    let updated = step.clone();
    project.touch();
    storage::save_project(&app, project)?;
    Ok(updated)
}

#[tauri::command]
pub fn reorder_steps(
    app: AppHandle,
    state: State<AppState>,
    order: Vec<String>,
) -> Result<Project> {
    let mut guard = state.project.lock();
    let project = guard
        .as_mut()
        .ok_or_else(|| AppError::NotFound("No document is open.".into()))?;

    let mut reordered: Vec<Step> = Vec::with_capacity(project.steps.len());
    for id in &order {
        if let Some(pos) = project.steps.iter().position(|s| s.id == *id) {
            reordered.push(project.steps.remove(pos));
        }
    }
    // Anything the frontend didn't mention keeps its relative position at the end
    // rather than silently disappearing.
    reordered.append(&mut project.steps);
    project.steps = reordered;
    project.touch();
    storage::save_project(&app, project)?;
    Ok(project.clone())
}

#[tauri::command]
pub fn delete_steps(app: AppHandle, state: State<AppState>, ids: Vec<String>) -> Result<Project> {
    let recording = state.session.lock().is_some();
    let mut guard = state.project.lock();
    let project = guard
        .as_mut()
        .ok_or_else(|| AppError::NotFound("No document is open.".into()))?;
    let mut updated = project.clone();
    if updated.delete_steps(&ids) {
        updated.touch();
        storage::save_project(&app, &updated)?;
        *project = updated;
        if !recording {
            let _ = storage::prune_frames(&app, project);
        }
    }
    Ok(project.clone())
}

#[tauri::command]
pub fn undo_step_deletion(
    app: AppHandle,
    state: State<AppState>,
    project_id: String,
) -> Result<Project> {
    let mut guard = state.project.lock();
    let project = guard
        .as_mut()
        .filter(|project| project.id == project_id)
        .ok_or_else(|| AppError::NotFound("That document is no longer open.".into()))?;
    let mut updated = project.clone();
    if updated.undo_step_deletion() {
        updated.touch();
        storage::save_project(&app, &updated)?;
        *project = updated;
    }
    Ok(project.clone())
}

/// Folds later steps into the earliest selected one, keeping its screenshot and
/// concatenating the text. Useful when the detector split one action in two.
#[tauri::command]
pub fn merge_steps(app: AppHandle, state: State<AppState>, ids: Vec<String>) -> Result<Project> {
    if ids.len() < 2 {
        return Err(AppError::Invalid(
            "Select at least two steps to merge.".into(),
        ));
    }
    let project = {
        let mut guard = state.project.lock();
        let project = guard
            .as_mut()
            .ok_or_else(|| AppError::NotFound("No document is open.".into()))?;

        let mut positions: Vec<usize> = project
            .steps
            .iter()
            .enumerate()
            .filter(|(_, s)| ids.contains(&s.id))
            .map(|(i, _)| i)
            .collect();
        if positions.len() < 2 {
            return Err(AppError::Invalid("Those steps could not be found.".into()));
        }
        positions.sort_unstable();

        let keep = positions[0];
        let mut bodies = Vec::new();
        let mut alternates = Vec::new();
        for &pos in &positions {
            let step = &project.steps[pos];
            if !step.body.trim().is_empty() {
                bodies.push(step.body.trim().to_string());
            }
            if pos != keep {
                alternates.push(step.frame.clone());
                alternates.extend(step.alternates.iter().cloned());
            }
        }

        let merged_body = bodies.join("\n\n");
        let target = &mut project.steps[keep];
        target.body = merged_body;
        target.alternates.extend(alternates);
        target.alternates.dedup();
        target.locked = true;

        let drop: Vec<String> = positions[1..]
            .iter()
            .map(|&p| project.steps[p].id.clone())
            .collect();
        project.steps.retain(|s| !drop.contains(&s.id));
        project.touch();
        storage::save_project(&app, project)?;
        project.clone()
    };
    Ok(project)
}

// ---------------------------------------------------------------------------
// Generation
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn generate(
    app: AppHandle,
    state: State<AppState>,
    scope: String,
    ids: Vec<String>,
    provider: Option<String>,
    model: Option<String>,
    language: Option<String>,
) -> Result<()> {
    if state.is_generating() {
        return Err(AppError::Invalid("A run is already in progress.".into()));
    }

    let settings = state.settings.lock().clone();
    let run_provider = provider
        .as_deref()
        .and_then(Provider::parse)
        .unwrap_or(settings.provider);

    // Fail here rather than after the first frame has been resized and sent, so
    // a missing key reads as a setup problem and not a generation failure.
    if run_provider.needs_key() && storage::get_api_key(&app, run_provider).is_none() {
        return Err(AppError::MissingApiKey);
    }

    let run_model = model
        .as_ref()
        .map(|m| m.trim().to_string())
        .filter(|m| !m.is_empty())
        .unwrap_or_else(|| settings.config_for(run_provider).model);

    if !ai::catalog::model_has_vision(run_provider, &run_model) {
        return Err(AppError::Invalid(format!(
            "The model `{run_model}` cannot read screenshots. Pick a vision model in the write \
menu — for Mistral, use Small or Large."
        )));
    }

    let scope = match scope.as_str() {
        "all" => ai::Scope::All,
        "only" => ai::Scope::Only(ids),
        _ => ai::Scope::Missing,
    };

    let overrides = ai::GenerationOverrides {
        provider: provider.as_deref().and_then(Provider::parse),
        model,
        language,
    };

    let cancel = Arc::new(AtomicBool::new(false));
    *state.generation.lock() = Some(Arc::clone(&cancel));

    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        let result = ai::run(handle.clone(), scope, cancel, overrides).await;
        handle.state::<AppState>().generation.lock().take();
        if let Err(e) = result {
            // `AppError` isn't `Clone`, which `emit` requires; serialize it once.
            if let Ok(payload) = serde_json::to_value(&e) {
                let _ = handle.emit("ai:error", payload);
            }
        }
    });
    Ok(())
}

/// The fixed wording exports print for `language`, and whether Walkmark ships
/// it or the user supplies it.
#[tauri::command]
pub fn document_labels(state: State<AppState>, language: String) -> ResolvedLabels {
    state.settings.lock().resolve_labels(&language)
}

/// Asks the AI for the fixed wording in `language`. Returns it for the caller
/// to review and save; nothing is stored here.
#[tauri::command]
pub async fn translate_labels(
    app: AppHandle,
    state: State<'_, AppState>,
    provider: Option<String>,
    model: Option<String>,
    language: Option<String>,
) -> Result<DocumentLabels> {
    let language = {
        let project_language = state
            .project
            .lock()
            .as_ref()
            .map(|p| p.language.clone())
            .filter(|l| !l.trim().is_empty());
        language
            .filter(|l| !l.trim().is_empty())
            .or(project_language)
            .unwrap_or_else(|| state.settings.lock().language.clone())
    };
    let overrides = ai::GenerationOverrides {
        provider: provider.as_deref().and_then(Provider::parse),
        model,
        language: None,
    };
    ai::translate_labels(&app, &language, &overrides).await
}

#[tauri::command]
pub fn cancel_generation(state: State<AppState>) {
    if let Some(flag) = state.generation.lock().as_ref() {
        flag.store(true, Ordering::SeqCst);
    }
}

// ---------------------------------------------------------------------------
// Export
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn suggest_export_name(state: State<AppState>, format: ExportFormat) -> String {
    state
        .project
        .lock()
        .as_ref()
        .map(|p| export::default_file_name(p, format))
        .unwrap_or_else(|| format!("walkmark-document.{}", format.extension()))
}

#[tauri::command]
pub async fn export_document(
    app: AppHandle,
    options: ExportOptions,
    destination: String,
) -> Result<ExportResult> {
    let project = app
        .state::<AppState>()
        .project
        .lock()
        .clone()
        .ok_or_else(|| AppError::NotFound("No document is open.".into()))?;

    // Encoding a few dozen PNGs and laying out a PDF is heavy enough that it
    // would visibly stall the window.
    tauri::async_runtime::spawn_blocking(move || {
        export::write(&app, &project, &options, std::path::Path::new(&destination))
    })
    .await
    .map_err(|e| AppError::Other(e.to_string()))?
}

/// Writes a whole document (data and screenshots) to a portable `.zip`.
#[tauri::command]
pub async fn export_archive(app: AppHandle, id: String, destination: String) -> Result<u64> {
    tauri::async_runtime::spawn_blocking(move || {
        let project = storage::load_project(&app, &id)?;
        archive::export(&app, &project, std::path::Path::new(&destination))
    })
    .await
    .map_err(|e| AppError::Other(e.to_string()))?
}

/// Asks for a `.zip` and adds its document to the library. `None` means the
/// user cancelled the picker.
#[tauri::command]
pub async fn import_archive(app: AppHandle) -> Result<Option<ProjectSummary>> {
    tauri::async_runtime::spawn_blocking(move || {
        let Some(file) = app
            .dialog()
            .file()
            .set_title("Import a Walkmark archive")
            .add_filter("Walkmark archive", &["zip"])
            .blocking_pick_file()
        else {
            return Ok(None);
        };
        let path = file
            .into_path()
            .map_err(|_| AppError::Invalid("Choose a local .zip file.".into()))?;
        archive::import(&app, &path).map(Some)
    })
    .await
    .map_err(|e| AppError::Other(e.to_string()))?
}

/// Markdown for the clipboard, without touching the filesystem.
#[tauri::command]
pub fn copy_as_markdown(app: AppHandle, state: State<AppState>) -> Result<String> {
    let project = state
        .project
        .lock()
        .clone()
        .ok_or_else(|| AppError::NotFound("No document is open.".into()))?;

    let options = export::resolve_options(
        &app,
        &project,
        &ExportOptions {
            include_images: false,
            ..Default::default()
        },
    );
    let steps = export::prepare(&app, &project, &options)?;

    let mut out = format!("# {}\n\n", project.title.trim());
    if !project.summary.trim().is_empty() {
        out.push_str(project.summary.trim());
        out.push_str("\n\n");
    }
    for step in &steps {
        out.push_str(&format!("## {}. {}\n\n", step.number, step.title));
        if !step.body.is_empty() {
            out.push_str(&step.body);
            out.push_str("\n\n");
        }
    }
    Ok(out)
}

/// Deep link into the macOS privacy pane, for the (common) case where the
/// system prompt has already been dismissed once and won't reappear.
#[tauri::command]
pub fn open_privacy_settings(app: AppHandle) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        use tauri_plugin_opener::OpenerExt;
        app.opener()
            .open_url(
                "x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture",
                None::<&str>,
            )
            .map_err(|e| AppError::Other(e.to_string()))
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = app;
        Ok(())
    }
}

#[tauri::command]
pub fn open_accessibility_settings(app: AppHandle) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        use tauri_plugin_opener::OpenerExt;
        app.opener()
            .open_url(
                "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility",
                None::<&str>,
            )
            .map_err(|e| AppError::Other(e.to_string()))
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = app;
        Ok(())
    }
}
