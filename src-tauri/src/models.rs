use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

fn now() -> DateTime<Utc> {
    Utc::now()
}

fn new_id() -> String {
    uuid::Uuid::new_v4().simple().to_string()[..12].to_string()
}

// ---------------------------------------------------------------------------
// Capture sources
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SourceKind {
    Monitor,
    Window,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureSource {
    /// Opaque handle of the form `monitor:<id>` or `window:<id>`.
    pub id: String,
    pub kind: SourceKind,
    pub name: String,
    /// Secondary line in the picker: app name for windows, resolution for monitors.
    pub detail: String,
    pub width: u32,
    pub height: u32,
    pub is_primary: bool,
    /// Small PNG data URL used for the live preview grid.
    pub thumbnail: Option<String>,
}

// ---------------------------------------------------------------------------
// Annotations
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AnnotationKind {
    /// Mosaic the region — reversible-looking but destructive in exports.
    Blur,
    /// Paint the region solid so nothing can be recovered.
    Redact,
    /// Draw an accent outline to draw the eye.
    Highlight,
    /// Tint a rectangle without covering the pixels underneath.
    Tint,
    Arrow,
    /// Mark a click with a pointer and a translucent halo.
    Mouse,
}

/// Rectangle in normalized (0..1) image space so annotations survive
/// re-framing and any downscaling we apply on export.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    /// Clamp to the image and convert to integer pixel bounds.
    pub fn to_pixels(self, img_w: u32, img_h: u32) -> (u32, u32, u32, u32) {
        let x0 = (self.x.clamp(0.0, 1.0) * img_w as f32).round() as u32;
        let y0 = (self.y.clamp(0.0, 1.0) * img_h as f32).round() as u32;
        let x1 = ((self.x + self.w).clamp(0.0, 1.0) * img_w as f32).round() as u32;
        let y1 = ((self.y + self.h).clamp(0.0, 1.0) * img_h as f32).round() as u32;
        (
            x0.min(img_w),
            y0.min(img_h),
            x1.saturating_sub(x0).min(img_w - x0.min(img_w)),
            y1.saturating_sub(y0).min(img_h - y0.min(img_h)),
        )
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum AnnotationStroke {
    #[default]
    Medium,
    Thin,
    Thick,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArrowDirection {
    pub reverse_x: bool,
    pub reverse_y: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Annotation {
    #[serde(default = "new_id")]
    pub id: String,
    pub kind: AnnotationKind,
    pub rect: Rect,
    /// `#rrggbb` — applied by tools that support colour.
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub stroke: Option<AnnotationStroke>,
    /// The start corner of the arrow's bounding rectangle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arrow: Option<ArrowDirection>,
}

// ---------------------------------------------------------------------------
// Steps
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum StepStatus {
    /// Captured, no description yet.
    Draft,
    Queued,
    Generating,
    Ready,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Step {
    pub id: String,
    pub title: String,
    pub body: String,
    /// Milliseconds from the start of the recording session.
    pub offset_ms: u64,
    /// File name (not path) of the chosen frame inside the project's `frames/` dir.
    pub frame: String,
    /// Neighbouring frames the user can swap in when the capture was mistimed.
    #[serde(default)]
    pub alternates: Vec<String>,
    #[serde(default)]
    pub annotations: Vec<Annotation>,
    /// Visible region in the original frame. The original file stays intact.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crop: Option<Rect>,
    #[serde(default = "default_true")]
    pub include: bool,
    /// Set once a human edits the text — regeneration then skips this step
    /// unless it is explicitly targeted.
    #[serde(default)]
    pub locked: bool,
    pub status: StepStatus,
    #[serde(default)]
    pub error: Option<String>,
    /// True when the step was pinned by the user rather than auto-detected.
    #[serde(default)]
    pub manual: bool,
}

fn default_true() -> bool {
    true
}

impl Step {
    pub fn new(frame: String, offset_ms: u64, manual: bool) -> Self {
        Self {
            id: new_id(),
            title: String::new(),
            body: String::new(),
            offset_ms,
            frame,
            alternates: Vec::new(),
            annotations: Vec::new(),
            crop: None,
            include: true,
            locked: false,
            status: StepStatus::Draft,
            error: None,
            manual,
        }
    }
}

// ---------------------------------------------------------------------------
// Product vocabulary
// ---------------------------------------------------------------------------

/// One preferred term and what it means in this product.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VocabularyTerm {
    #[serde(default = "new_id")]
    pub id: String,
    #[serde(default)]
    pub term: String,
    #[serde(default)]
    pub explanation: String,
}

fn parse_vocabulary_string(raw: &str) -> Vec<VocabularyTerm> {
    raw.lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() {
                return None;
            }
            Some(VocabularyTerm {
                id: new_id(),
                term: line.to_string(),
                explanation: String::new(),
            })
        })
        .collect()
}

fn deserialize_vocabulary<'de, D>(deserializer: D) -> Result<Vec<VocabularyTerm>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::{self, Visitor};
    use std::fmt;

    struct VocabVisitor;

    impl<'de> Visitor<'de> for VocabVisitor {
        type Value = Vec<VocabularyTerm>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a vocabulary string or array of terms")
        }

        fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
            Ok(parse_vocabulary_string(value))
        }

        fn visit_seq<A: de::SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let mut terms = Vec::new();
            while let Some(term) = seq.next_element::<VocabularyTerm>()? {
                terms.push(term);
            }
            Ok(terms)
        }
    }

    deserializer.deserialize_any(VocabVisitor)
}

/// Named vocabulary profile for a product the user documents.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Product {
    #[serde(default = "new_id")]
    pub id: String,
    pub name: String,
    /// Preferred terms and what they mean in this product.
    #[serde(default, deserialize_with = "deserialize_vocabulary")]
    pub vocabulary: Vec<VocabularyTerm>,
}

impl Product {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: new_id(),
            name: name.into(),
            vocabulary: Vec::new(),
        }
    }

    pub fn normalize_vocabulary(&mut self) {
        for term in &mut self.vocabulary {
            if term.id.is_empty() {
                term.id = new_id();
            }
        }
    }

    /// Rendered for the model prompt.
    pub fn format_vocabulary(&self) -> String {
        self.vocabulary
            .iter()
            .filter(|t| !t.term.trim().is_empty())
            .map(|t| {
                let term = t.term.trim();
                let explanation = t.explanation.trim();
                if explanation.is_empty() {
                    format!("- {term}")
                } else {
                    format!("- {term} — {explanation}")
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

// ---------------------------------------------------------------------------
// Project
// ---------------------------------------------------------------------------

fn default_language() -> String {
    "English".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeletedStep {
    pub index: usize,
    pub step: Step,
}

fn default_document_version() -> String {
    "1.0".into()
}

fn pick_text(mine: &str, theirs: &str) -> String {
    let mine = mine.trim();
    if mine.is_empty() {
        theirs.trim().to_owned()
    } else {
        mine.to_owned()
    }
}

/// Declares `DocumentLabels` once, so adding a label is one line here.
macro_rules! document_labels {
    ($($field:ident => $english:literal),+ $(,)?) => {
        /// Fixed wording that exports print around the guide's own text. It
        /// belongs to a language, not to a guide, and lives in Settings. An
        /// empty value falls back to the language's built-in wording, then to
        /// English.
        #[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
        #[serde(rename_all = "camelCase", default)]
        pub struct DocumentLabels {
            $(pub $field: String,)+
        }

        impl DocumentLabels {
            /// The English wording, written out in full.
            pub fn english() -> Self {
                Self { $($field: $english.into(),)+ }
            }

            /// Builds labels from values in declaration order.
            fn from_values(values: [&str; [$(stringify!($field)),+].len()]) -> Self {
                let mut values = values.into_iter();
                Self { $($field: values.next().unwrap_or_default().into(),)+ }
            }

            fn is_blank(&self) -> bool {
                true $(&& self.$field.trim().is_empty())+
            }

            /// `self` where it says something, `base` where it does not.
            fn over(&self, base: &Self) -> Self {
                Self { $($field: pick_text(&self.$field, &base.$field),)+ }
            }

            /// Applies `f` to every label.
            pub fn map(&self, f: impl Fn(&str) -> String) -> Self {
                Self { $($field: f(&self.$field),)+ }
            }

            $(
                pub fn $field(&self) -> &str {
                    let text = self.$field.trim();
                    if text.is_empty() { $english } else { text }
                }
            )+
        }
    };
}

document_labels! {
    guide => "Step-by-step guide",
    prerequisites => "Before you start",
    exported => "Exported",
    version => "Version",
    author => "Author",
    steps => "Steps",
    on_this_page => "On this page",
    step => "Step",
    step_one => "step",
    step_many => "steps",
    generated_with => "Generated with Walkmark",
}

impl DocumentLabels {
    /// Wording that ships with Walkmark. Languages not listed here are
    /// translated by the user, or by the AI on request.
    pub fn built_in(language: &str) -> Option<Self> {
        // Order: guide, prerequisites, exported, version, author, steps,
        // on this page, step, step (one), steps (many), generated with.
        let words = match language.trim().to_lowercase().as_str() {
            "english" => return Some(Self::english()),
            "italian" => [
                "Guida passo passo",
                "Prima di iniziare",
                "Esportato il",
                "Versione",
                "Autore",
                "Passaggi",
                "In questa pagina",
                "Passaggio",
                "passaggio",
                "passaggi",
                "Generato con Walkmark",
            ],
            "spanish" => [
                "Guía paso a paso",
                "Antes de empezar",
                "Exportado el",
                "Versión",
                "Autor",
                "Pasos",
                "En esta página",
                "Paso",
                "paso",
                "pasos",
                "Generado con Walkmark",
            ],
            "french" => [
                "Guide pas à pas",
                "Avant de commencer",
                "Exporté le",
                "Version",
                "Auteur",
                "Étapes",
                "Sur cette page",
                "Étape",
                "étape",
                "étapes",
                "Généré avec Walkmark",
            ],
            "german" => [
                "Schritt-für-Schritt-Anleitung",
                "Bevor Sie beginnen",
                "Exportiert am",
                "Version",
                "Autor",
                "Schritte",
                "Auf dieser Seite",
                "Schritt",
                "Schritt",
                "Schritte",
                "Erstellt mit Walkmark",
            ],
            "portuguese" => [
                "Guia passo a passo",
                "Antes de começar",
                "Exportado em",
                "Versão",
                "Autor",
                "Passos",
                "Nesta página",
                "Passo",
                "passo",
                "passos",
                "Gerado com Walkmark",
            ],
            "dutch" => [
                "Stapsgewijze handleiding",
                "Voordat je begint",
                "Geëxporteerd op",
                "Versie",
                "Auteur",
                "Stappen",
                "Op deze pagina",
                "Stap",
                "stap",
                "stappen",
                "Gemaakt met Walkmark",
            ],
            _ => return None,
        };
        Some(Self::from_values(words))
    }

    /// "1 step", "3 steps".
    pub fn step_count(&self, count: usize) -> String {
        let word = if count == 1 {
            self.step_one()
        } else {
            self.step_many()
        };
        format!("{count} {word}")
    }

    /// Language code for `<html lang>`, so readers and browsers treat the
    /// document in the right language. Unknown languages fall back to English.
    pub fn html_lang(language: &str) -> &'static str {
        match language.trim().to_lowercase().as_str() {
            "spanish" => "es",
            "french" => "fr",
            "german" => "de",
            "portuguese" => "pt",
            "italian" => "it",
            "dutch" => "nl",
            "polish" => "pl",
            "russian" => "ru",
            "ukrainian" => "uk",
            "turkish" => "tr",
            "arabic" => "ar",
            "hindi" => "hi",
            "japanese" => "ja",
            "korean" => "ko",
            "lithuanian" => "lt",
            "swedish" => "sv",
            "czech" => "cs",
            "romanian" => "ro",
            "vietnamese" => "vi",
            "indonesian" => "id",
            "greek" => "el",
            "hebrew" => "he",
            "chinese (simplified)" => "zh-Hans",
            "chinese (traditional)" => "zh-Hant",
            _ => "en",
        }
    }
}

/// Version of the `project.json` layout. Projects written before the field
/// existed read back as version 1.
pub const PROJECT_SCHEMA_VERSION: u32 = 1;

fn default_schema_version() -> u32 {
    PROJECT_SCHEMA_VERSION
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub prerequisites: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(default)]
    pub source_label: String,
    /// Which product vocabulary applies to this guide.
    #[serde(default)]
    pub product_id: Option<String>,
    /// Language used when AI writes this document.
    #[serde(default = "default_language")]
    pub language: String,
    /// Revision of the guide itself, shown under the title in exports.
    #[serde(default = "default_document_version")]
    pub version: String,
    /// Who made the guide, shown under the title in exports.
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub steps: Vec<Step>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub deleted_steps: Vec<DeletedStep>,
}

impl Project {
    pub fn new(title: impl Into<String>, source_label: impl Into<String>) -> Self {
        Self {
            schema_version: PROJECT_SCHEMA_VERSION,
            id: new_id(),
            title: title.into(),
            summary: String::new(),
            prerequisites: Vec::new(),
            created_at: now(),
            updated_at: now(),
            source_label: source_label.into(),
            product_id: None,
            language: default_language(),
            version: default_document_version(),
            author: String::new(),
            steps: Vec::new(),
            deleted_steps: Vec::new(),
        }
    }

    pub fn touch(&mut self) {
        self.updated_at = now();
    }

    pub fn insert_manual_step(&mut self, frame: String, after_step_id: Option<&str>) -> String {
        let index = after_step_id
            .and_then(|id| self.steps.iter().position(|step| step.id == id))
            .map_or(self.steps.len(), |index| index + 1);
        let offset = if index == 0 {
            0
        } else {
            self.steps[index - 1].offset_ms
        };
        let step = Step::new(frame, offset, true);
        let id = step.id.clone();
        self.steps.insert(index, step);
        id
    }

    pub fn delete_steps(&mut self, ids: &[String]) -> bool {
        let mut index = 0;
        let mut deleted = Vec::new();
        self.steps.retain(|step| {
            let remove = ids.contains(&step.id);
            if remove {
                deleted.push(DeletedStep {
                    index,
                    step: step.clone(),
                });
            }
            index += 1;
            !remove
        });
        if deleted.is_empty() {
            return false;
        }
        self.deleted_steps = deleted;
        true
    }

    pub fn undo_step_deletion(&mut self) -> bool {
        if self.deleted_steps.is_empty() {
            return false;
        }
        for DeletedStep { index, mut step } in std::mem::take(&mut self.deleted_steps) {
            if matches!(step.status, StepStatus::Queued | StepStatus::Generating) {
                step.status = if step.body.is_empty() {
                    StepStatus::Draft
                } else {
                    StepStatus::Ready
                };
            }
            self.steps.insert(index.min(self.steps.len()), step);
        }
        true
    }
}

#[cfg(test)]
mod project_step_tests {
    use super::*;

    #[test]
    fn projects_without_a_schema_version_read_as_version_one() {
        let project = Project::new("Guide", "Window");
        let mut value = serde_json::to_value(&project).unwrap();
        assert_eq!(value["schemaVersion"], PROJECT_SCHEMA_VERSION);

        value.as_object_mut().unwrap().remove("schemaVersion");
        let loaded: Project = serde_json::from_value(value).unwrap();
        assert_eq!(loaded.schema_version, 1);
    }

    #[test]
    fn inserts_a_manual_draft_after_the_anchor_or_at_the_end() {
        let mut project = Project::new("Guide", "Window");
        let first = Step::new("first.png".into(), 100, false);
        let last = Step::new("last.png".into(), 200, false);
        let first_id = first.id.clone();
        let last_id = last.id.clone();
        project.steps.extend([first, last]);

        let inserted = project.insert_manual_step("import.png".into(), Some(&first_id));
        let appended = project.insert_manual_step("end.png".into(), None);
        let order: Vec<&str> = project.steps.iter().map(|step| step.id.as_str()).collect();
        assert_eq!(order, [&first_id, &inserted, &last_id, &appended]);
        let step = &project.steps[1];
        assert!(step.manual);
        assert_eq!(step.status, StepStatus::Draft);
        assert_eq!(step.frame, "import.png");
        assert_eq!(step.offset_ms, 100);
    }

    #[test]
    fn restores_the_latest_deletion_in_order_after_reloading() {
        let mut project = Project::new("Guide", "Window");
        project.steps = (0..5)
            .map(|i| {
                let mut step = Step::new(format!("{i}.png"), i, false);
                step.title = format!("Step {i}");
                step.alternates.push(format!("{i}-alternate.png"));
                step
            })
            .collect();
        let original = project.steps.clone();

        assert!(project.delete_steps(&[original[0].id.clone()]));
        assert!(project.delete_steps(&[original[2].id.clone(), original[4].id.clone()]));
        assert!(!project.delete_steps(&["missing".into()]));
        let saved = serde_json::to_string(&project).unwrap();
        let mut reopened: Project = serde_json::from_str(&saved).unwrap();

        assert!(reopened.undo_step_deletion());
        assert_eq!(
            serde_json::to_value(&reopened.steps).unwrap(),
            serde_json::to_value(&original[1..]).unwrap()
        );
        assert!(!reopened.undo_step_deletion());
    }
}

/// Lightweight row for the project library, so we never load every frame list
/// just to render the home screen.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSummary {
    pub id: String,
    pub title: String,
    pub updated_at: DateTime<Utc>,
    pub step_count: usize,
    pub ready_count: usize,
    /// Absolute path to the cover frame, for `convertFileSrc`.
    pub cover: Option<String>,
}

// ---------------------------------------------------------------------------
// Settings
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Tone {
    Neutral,
    Friendly,
    Formal,
    Playful,
}

impl Tone {
    pub fn describe(self) -> &'static str {
        match self {
            Tone::Neutral => "plain, matter-of-fact and neutral",
            Tone::Friendly => "warm and encouraging, but never chatty",
            Tone::Formal => "formal and precise, suitable for a compliance manual",
            Tone::Playful => "light and personable, with the occasional bit of character",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureSettings {
    /// How often we grab a frame while recording.
    pub sample_interval_ms: u64,
    /// 0 = only huge changes become steps, 1 = almost every change does.
    /// Used only when [`Self::visual_fallback`] is enabled.
    pub sensitivity: f32,
    /// Never emit two steps closer together than this.
    pub min_gap_ms: u64,
    /// Wait for the screen to stop changing before committing a step, so we
    /// don't capture half-open menus or mid-flight animations.
    /// Used only when [`Self::visual_fallback`] is enabled.
    pub settle: bool,
    /// Fall back to visual change detection when input monitoring is unavailable
    /// or as a supplement to catch transitions input misses.
    #[serde(default)]
    pub visual_fallback: bool,
    /// After a click, key press or scroll, wait this long before taking the
    /// screenshot so menus and typed text have time to appear.
    #[serde(default = "default_input_settle_ms")]
    pub input_settle_ms: u64,
    /// Longest edge of stored frames. Keeps retina captures from ballooning.
    pub max_width: u32,
    pub countdown_secs: u32,
    /// Hide the main window while recording and show the floating HUD instead.
    pub hide_window: bool,
}

fn default_input_settle_ms() -> u64 {
    300
}

impl Default for CaptureSettings {
    fn default() -> Self {
        Self {
            sample_interval_ms: 200,
            sensitivity: 0.55,
            min_gap_ms: 800,
            settle: true,
            visual_fallback: false,
            input_settle_ms: 300,
            max_width: 1800,
            countdown_secs: 3,
            hide_window: true,
        }
    }
}

/// Where the writing happens. Everything behind this enum speaks a different
/// wire protocol, but the rest of the app only ever sees `Provider`.
#[derive(
    Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
pub enum Provider {
    #[default]
    #[serde(rename = "gemini")]
    Gemini,
    #[serde(rename = "openai")]
    OpenAi,
    #[serde(rename = "anthropic")]
    Anthropic,
    #[serde(rename = "mistral")]
    Mistral,
    /// One API key, many cloud models through openrouter.ai.
    #[serde(rename = "openrouter")]
    OpenRouter,
    /// Models running on this machine, managed through Ollama.
    #[serde(rename = "ollama")]
    Ollama,
    /// Any server that speaks the OpenAI chat-completions protocol — LM Studio,
    /// llama.cpp, vLLM, a corporate gateway.
    #[serde(rename = "compatible")]
    Compatible,
}

impl Provider {
    pub const ALL: [Provider; 7] = [
        Provider::Gemini,
        Provider::OpenAi,
        Provider::Anthropic,
        Provider::Mistral,
        Provider::OpenRouter,
        Provider::Ollama,
        Provider::Compatible,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Provider::Gemini => "gemini",
            Provider::OpenAi => "openai",
            Provider::Anthropic => "anthropic",
            Provider::Mistral => "mistral",
            Provider::OpenRouter => "openrouter",
            Provider::Ollama => "ollama",
            Provider::Compatible => "compatible",
        }
    }

    pub fn parse(id: &str) -> Option<Self> {
        Provider::ALL.into_iter().find(|p| p.id() == id)
    }

    /// Human name, used in error messages so they read naturally.
    pub fn label(self) -> &'static str {
        match self {
            Provider::Gemini => "Gemini",
            Provider::OpenAi => "OpenAI",
            Provider::Anthropic => "Claude",
            Provider::Mistral => "Mistral",
            Provider::OpenRouter => "OpenRouter",
            Provider::Ollama => "Ollama",
            Provider::Compatible => "The server",
        }
    }

    /// Local providers never need a key and never send anything off the machine.
    pub fn is_local(self) -> bool {
        matches!(self, Provider::Ollama)
    }

    pub fn needs_key(self) -> bool {
        !matches!(self, Provider::Ollama)
    }

    /// A custom endpoint is required for `Compatible` and optional elsewhere.
    pub fn default_base_url(self) -> &'static str {
        match self {
            Provider::Gemini => "https://generativelanguage.googleapis.com/v1beta",
            Provider::OpenAi => "https://api.openai.com/v1",
            Provider::Anthropic => "https://api.anthropic.com/v1",
            Provider::Mistral => "https://api.mistral.ai/v1",
            Provider::OpenRouter => "https://openrouter.ai/api/v1",
            Provider::Ollama => "http://127.0.0.1:11434",
            Provider::Compatible => "",
        }
    }
}

/// The model and endpoint chosen for one provider. Kept per provider so that
/// switching to Ollama to try something and switching back doesn't lose the
/// cloud model you had picked.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderConfig {
    #[serde(default)]
    pub model: String,
    /// Empty means "use the provider's default endpoint".
    #[serde(default)]
    pub base_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    /// Which provider generation runs against right now.
    #[serde(default)]
    pub provider: Provider,
    /// Saved selection per provider, keyed by `Provider::id`.
    #[serde(default)]
    pub providers: BTreeMap<Provider, ProviderConfig>,
    /// Free-text description of who will read the doc.
    pub audience: String,
    pub tone: Tone,
    pub language: String,
    /// Parallel requests. Higher is faster but hits rate limits sooner, and
    /// local models generally want this at 1.
    pub concurrency: usize,
    pub capture: CaptureSettings,
    pub theme: String,
    pub onboarded: bool,
    /// Named vocabulary profiles. At least one is always present.
    #[serde(default = "default_products")]
    pub products: Vec<Product>,
    /// Last product picked when starting a recording.
    #[serde(default)]
    pub default_product_id: Option<String>,
    /// Wording the user changed, keyed by language. Anything left empty uses
    /// the built-in wording for that language.
    #[serde(default)]
    pub labels: BTreeMap<String, DocumentLabels>,
}

/// The fixed wording for one language, and where it came from.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedLabels {
    /// What exports will print.
    pub labels: DocumentLabels,
    /// What it would be without the user's changes.
    pub defaults: DocumentLabels,
    /// Walkmark ships wording for this language. When false, the user types
    /// it or asks the AI.
    pub localized: bool,
    /// The user has changed something for this language.
    pub customized: bool,
}

fn default_products() -> Vec<Product> {
    vec![Product::new("General")]
}

impl Default for Settings {
    fn default() -> Self {
        let general = Product::new("General");
        let default_id = general.id.clone();
        Self {
            provider: Provider::default(),
            providers: BTreeMap::new(),
            audience: "a colleague who has never used this tool before".into(),
            tone: Tone::Neutral,
            language: "English".into(),
            concurrency: 3,
            capture: CaptureSettings::default(),
            theme: "system".into(),
            onboarded: false,
            products: vec![general],
            default_product_id: Some(default_id),
            labels: BTreeMap::new(),
        }
    }
}

impl Settings {
    /// Guarantee a usable product list after loading older settings files.
    pub fn clamp_text(&mut self) {
        self.audience = crate::limits::clamp(&self.audience, crate::limits::AUDIENCE);
        self.language = crate::limits::clamp(&self.language, crate::limits::LANGUAGE);
        for product in &mut self.products {
            product.name = crate::limits::clamp(&product.name, crate::limits::PRODUCT_NAME);
            for term in &mut product.vocabulary {
                term.term = crate::limits::clamp(&term.term, crate::limits::VOCABULARY_TERM);
                term.explanation =
                    crate::limits::clamp(&term.explanation, crate::limits::VOCABULARY_EXPLANATION);
            }
        }
        self.clamp_labels();
        for config in self.providers.values_mut() {
            config.model = crate::limits::clamp(&config.model, crate::limits::MODEL_ID);
            config.base_url = crate::limits::clamp(&config.base_url, crate::limits::BASE_URL);
        }
    }

    /// The user's own wording for `language`, matched without regard to case.
    fn custom_labels(&self, language: &str) -> Option<&DocumentLabels> {
        let wanted = language.trim().to_lowercase();
        self.labels
            .iter()
            .find(|(name, _)| name.trim().to_lowercase() == wanted)
            .map(|(_, labels)| labels)
    }

    pub fn resolve_labels(&self, language: &str) -> ResolvedLabels {
        let built_in = DocumentLabels::built_in(language);
        let localized = built_in.is_some();
        let defaults = built_in.unwrap_or_else(DocumentLabels::english);
        let custom = self.custom_labels(language).filter(|c| !c.is_blank());
        ResolvedLabels {
            labels: custom.map_or_else(|| defaults.clone(), |c| c.over(&defaults)),
            customized: custom.is_some(),
            localized,
            defaults,
        }
    }

    /// Keep saved wording bounded and drop entries that say nothing.
    fn clamp_labels(&mut self) {
        use crate::limits::{clamp_trim, DOCUMENT_LABEL, LANGUAGE};
        let cleaned: BTreeMap<String, DocumentLabels> = std::mem::take(&mut self.labels)
            .into_iter()
            .map(|(language, labels)| {
                (
                    clamp_trim(&language, LANGUAGE),
                    labels.map(|text| clamp_trim(text, DOCUMENT_LABEL)),
                )
            })
            .filter(|(language, labels)| !language.is_empty() && !labels.is_blank())
            .take(crate::limits::LABEL_LANGUAGES_MAX)
            .collect();
        self.labels = cleaned;
    }

    pub fn normalize_products(&mut self) {
        if self.products.is_empty() {
            let general = Product::new("General");
            self.default_product_id = Some(general.id.clone());
            self.products.push(general);
        }
        for product in &mut self.products {
            product.normalize_vocabulary();
        }
        self.clamp_text();
        if self.default_product_id.is_none()
            || self
                .default_product_id
                .as_ref()
                .is_some_and(|id| self.product(id).is_none())
        {
            self.default_product_id = self.products.first().map(|p| p.id.clone());
        }
    }

    pub fn product(&self, id: &str) -> Option<&Product> {
        self.products.iter().find(|p| p.id == id)
    }

    pub fn resolve_product(&self, id: Option<&str>) -> Option<&Product> {
        id.and_then(|id| self.product(id))
    }

    pub fn config_for(&self, provider: Provider) -> ProviderConfig {
        let saved = self.providers.get(&provider);
        let model = saved
            .map(|c| c.model.trim())
            .filter(|m| !m.is_empty())
            .unwrap_or_else(|| crate::ai::catalog::default_model(provider))
            .to_string();
        let base_url = saved
            .map(|c| c.base_url.trim())
            .filter(|u| !u.is_empty())
            .unwrap_or_else(|| provider.default_base_url())
            .trim_end_matches('/')
            .to_string();
        ProviderConfig { model, base_url }
    }
}

// ---------------------------------------------------------------------------
// Events emitted to the frontend
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RecordingState {
    Idle,
    Counting,
    Recording,
    Paused,
    Stopping,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordingTick {
    pub state: RecordingState,
    pub elapsed_ms: u64,
    pub step_count: usize,
    /// 0..1 measure of how much the screen just changed.
    pub activity: f32,
    pub countdown: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerationProgress {
    pub done: usize,
    pub total: usize,
    pub running: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    Markdown,
    Html,
    Pdf,
}

impl ExportFormat {
    pub fn extension(self) -> &'static str {
        match self {
            ExportFormat::Markdown => "md",
            ExportFormat::Html => "html",
            ExportFormat::Pdf => "pdf",
        }
    }
}

#[cfg(test)]
mod label_tests {
    use super::*;

    #[test]
    fn english_and_built_in_languages_are_localized() {
        let settings = Settings::default();
        let english = settings.resolve_labels("English");
        assert!(english.localized && !english.customized);
        assert_eq!(english.labels.prerequisites, "Before you start");

        let italian = settings.resolve_labels(" italian ");
        assert!(italian.localized);
        assert_eq!(italian.labels.prerequisites, "Prima di iniziare");
        assert_eq!(italian.labels.exported, "Esportato il");
    }

    #[test]
    fn other_languages_start_from_english_and_say_they_need_translating() {
        let lithuanian = Settings::default().resolve_labels("Lithuanian");
        assert!(!lithuanian.localized);
        assert_eq!(lithuanian.labels, DocumentLabels::english());
    }

    #[test]
    fn the_users_wording_wins_field_by_field() {
        let mut settings = Settings::default();
        settings.labels.insert(
            "ITALIAN".into(),
            DocumentLabels {
                prerequisites: "Requisiti".into(),
                ..Default::default()
            },
        );
        let italian = settings.resolve_labels("Italian");
        assert!(italian.customized);
        assert_eq!(italian.labels.prerequisites, "Requisiti");
        assert_eq!(italian.labels.guide, "Guida passo passo");
        assert_eq!(italian.defaults.prerequisites, "Prima di iniziare");
    }

    #[test]
    fn saving_drops_empty_entries_and_trims_the_rest() {
        let mut settings = Settings::default();
        settings
            .labels
            .insert("Italian".into(), DocumentLabels::default());
        settings.labels.insert(
            "Lithuanian".into(),
            DocumentLabels {
                guide: format!("  {}  ", "x".repeat(200)),
                ..Default::default()
            },
        );
        settings.normalize_products();
        assert!(!settings.labels.contains_key("Italian"));
        assert_eq!(
            settings.labels["Lithuanian"].guide.chars().count(),
            crate::limits::DOCUMENT_LABEL
        );
    }

    #[test]
    fn older_settings_files_have_no_custom_wording() {
        let mut value = serde_json::to_value(Settings::default()).unwrap();
        value.as_object_mut().unwrap().remove("labels");
        let loaded: Settings = serde_json::from_value(value).unwrap();
        assert!(loaded.labels.is_empty());
    }

    #[test]
    fn every_built_in_language_fills_every_label() {
        for language in [
            "English",
            "Italian",
            "Spanish",
            "French",
            "German",
            "Portuguese",
            "Dutch",
        ] {
            let labels = DocumentLabels::built_in(language).unwrap();
            assert!(!labels.is_blank());
            let json = serde_json::to_value(&labels).unwrap();
            for (key, value) in json.as_object().unwrap() {
                assert!(
                    !value.as_str().unwrap().trim().is_empty(),
                    "{language}: {key} is empty"
                );
            }
        }
    }

    #[test]
    fn built_in_words_land_in_the_right_fields() {
        let italian = DocumentLabels::built_in("Italian").unwrap();
        assert_eq!(italian.step_one, "passaggio");
        assert_eq!(italian.step_many, "passaggi");
        assert_eq!(italian.generated_with, "Generato con Walkmark");
        assert_eq!(italian.step_count(1), "1 passaggio");
        assert_eq!(italian.step_count(4), "4 passaggi");
        let german = DocumentLabels::built_in("German").unwrap();
        assert_eq!(
            (german.steps.as_str(), german.step.as_str()),
            ("Schritte", "Schritt")
        );
    }

    #[test]
    fn languages_map_to_html_codes() {
        assert_eq!(DocumentLabels::html_lang("Italian"), "it");
        assert_eq!(DocumentLabels::html_lang("Chinese (Simplified)"), "zh-Hans");
        assert_eq!(DocumentLabels::html_lang("Klingon"), "en");
    }
}
