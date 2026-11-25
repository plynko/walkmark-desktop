import { test as base, expect, type Page } from "@playwright/test";

/** One recorded Tauri command, in call order. */
export type Call = { cmd: string; args: Record<string, unknown> };

export interface App {
  page: Page;
  /** Every command the UI sent to the mocked backend. */
  calls(cmd?: string): Promise<Call[]>;
  /** Load the library and open the sample guide in the editor. */
  openGuide(): Promise<void>;
}

/**
 * Installs a stateful stand-in for the Tauri backend. It implements just enough
 * of the commands the editor uses (projects, steps, markdown preview) to
 * exercise the real Svelte UI in a plain browser. Native behaviour — screen
 * capture, the clipboard and macOS permissions — is not covered here.
 */
function installTauriMock(language: string) {
  const settings = {
    provider: "gemini",
    providers: {},
    audience: "a colleague",
    tone: "neutral",
    language,
    labels: {} as Record<string, Record<string, string>>,
    concurrency: 3,
    capture: {
      sampleIntervalMs: 600,
      sensitivity: 0.55,
      minGapMs: 1200,
      settle: true,
      maxWidth: 1800,
      countdownSecs: 3,
      hideWindow: true,
    },
    theme: "light",
    onboarded: true,
    products: [{ id: "general", name: "General", vocabulary: [] }],
    defaultProductId: "general",
  };

  const provider = (id: string, name: string, local: boolean) => ({
    id,
    name,
    blurb: "",
    local,
    needsKey: !local,
    keyUrl: "https://example.com/key",
    keyHint: "",
    keyGuide: [],
    baseUrlEditable: local,
    baseUrlRequired: false,
    defaultBaseUrl: local ? "http://127.0.0.1:11434" : "",
    defaultModel: local ? "qwen3-vl:8b" : id === "anthropic" ? "claude-sonnet-5-5" : "gemini-3.6-flash",
    models: local
      ? []
      : id === "anthropic"
        ? [{ id: "claude-sonnet-5-5", name: "Claude Sonnet 5.5", note: "", tier: "recommended" }]
        : [{ id: "gemini-3.6-flash", name: "Gemini 3.6 Flash", note: "", tier: "recommended" }],
  });
  const catalog = {
    providers: [
      provider("gemini", "Google Gemini", false),
      provider("anthropic", "Claude", false),
      provider("ollama", "On this Mac", true),
    ],
    // Keys saved for two cloud providers, as a user with both would have.
    configured: ["gemini", "anthropic"],
  };
  const local = {
    running: true,
    version: "0.6.2",
    endpoint: "http://127.0.0.1:11434",
    downloadUrl: "https://ollama.com/download",
    totalMemory: 16_000_000_000,
    downloading: false,
    models: [],
    catalog: [],
  };

  const makeStep = (id: string, title: string, frame: string, alternates: string[] = []) => ({
    id,
    title,
    body: "",
    offsetMs: 0,
    frame,
    alternates,
    annotations: [],
    include: true,
    locked: false,
    status: "draft",
    error: null,
    manual: false,
  });

  const state = {
    project: {
      id: "guide-1",
      title: "Sample guide",
      summary: "",
      prerequisites: ["An admin account"],
      version: "1.0",
      author: "",
          createdAt: "2026-01-01T00:00:00Z",
      updatedAt: "2026-01-01T00:00:00Z",
      sourceLabel: "Window",
      language,
      steps: [
        { ...makeStep("s1", "Open the menu", "frame-b.png", ["frame-a.png"]), body: "Click **File**." },
        makeStep("s2", "Save the file", "frame-c.png"),
      ],
    },
    added: 0,
  };

  const calls: { cmd: string; args: Record<string, unknown> }[] = [];
  (window as any).__calls = calls;

  const summary = () => ({
    id: state.project.id,
    title: state.project.title,
    updatedAt: state.project.updatedAt,
    stepCount: state.project.steps.length,
    readyCount: 0,
    cover: null,
  });

  const addStep = (args: any) => {
    const step = makeStep(`added-${++state.added}`, "", `manual-${state.added}.png`);
    const steps = state.project.steps;
    const at = args.afterStepId
      ? steps.findIndex((s) => s.id === args.afterStepId) + 1
      : steps.length;
    steps.splice(at, 0, step);
    return { project: structuredClone(state.project), stepId: step.id };
  };

  // Just enough Markdown to see that the preview is rendered, not echoed.
  const markdown = (text: string) =>
    `<p>${text.replace(/\*\*(.+?)\*\*/g, "<strong>$1</strong>")}</p>`;

  function mockInvoke(cmd: string, args: any) {
    switch (cmd) {
      case "plugin:event|listen":
        return 1;
      case "plugin:event|unlisten":
        return null;
      case "get_settings":
        return settings;
      case "save_settings":
        Object.assign(settings, args.settings);
        return settings;
      case "permission_status":
        return { granted: true, required: false };
      case "provider_catalog":
        return catalog;
      case "local_status":
        return local;
      case "recording_status":
        return { active: false, state: "idle", stepCount: 0 };
      case "list_projects":
        return [summary()];
      case "import_archive":
        return null;
      case "export_archive":
        return 0;
      case "current_project":
        return null;
      case "open_project":
        return structuredClone(state.project);
      case "frame_path":
        return args.frame;
      case "suggest_export_name":
        return `sample-guide.${args.format === "docx" ? "docx" : args.format}`;
      case "list_sources":
        return [
          {
            id: "monitor-1",
            kind: "monitor",
            name: "Built-in display",
            detail: "1440 × 900",
            width: 1440,
            height: 900,
            isPrimary: true,
            thumbnail: null,
          },
        ];
      case "document_labels": {
        const english = {
          guide: "Step-by-step guide",
          prerequisites: "Before you start",
          version: "Version",
          author: "Author",
          steps: "Steps",
          onThisPage: "On this page",
          step: "Step",
          stepOne: "step",
          stepMany: "steps",
          generatedWith: "Generated with Walkmark",
        };
        const italian = {
          guide: "Guida passo passo",
          prerequisites: "Prima di iniziare",
          version: "Versione",
          author: "Autore",
          steps: "Passaggi",
          onThisPage: "In questa pagina",
          step: "Passaggio",
          stepOne: "passaggio",
          stepMany: "passaggi",
          generatedWith: "Generato con Walkmark",
        };
        const name = String(args.language).trim().toLowerCase();
        const defaults = name === "italian" ? italian : english;
        const custom = Object.entries(settings.labels).find(([k]) => k.toLowerCase() === name)?.[1];
        const labels: Record<string, string> = { ...defaults };
        for (const [key, value] of Object.entries(custom ?? {})) if (value) labels[key] = value;
        return { labels, defaults, localized: name === "english" || name === "italian", customized: !!custom };
      }
      case "update_project_meta":
        Object.assign(state.project, args.meta);
        return structuredClone(state.project);
      case "translate_labels":
        return {
          guide: "Pamoka žingsnis po žingsnio",
          prerequisites: "Prieš pradedant",
          version: "Versija",
          author: "Autorius",
          steps: "Žingsniai",
          onThisPage: "Šiame puslapyje",
          step: "Žingsnis",
          stepOne: "žingsnis",
          stepMany: "žingsniai",
          generatedWith: "Sukurta su Walkmark",
        };
      case "update_step": {
        const step = state.project.steps.find((s) => s.id === args.id)!;
        const { frame, ...rest } = args.patch;
        Object.assign(step, rest);
        if (frame && step.alternates.includes(frame)) {
          step.alternates = [...step.alternates.filter((f) => f !== frame), step.frame];
          step.frame = frame;
        }
        return structuredClone(step);
      }
      case "import_step":
      case "paste_step":
      case "capture_step":
        return addStep(args);
      case "preview_markdown":
        return markdown(args.text);
      default:
        return null;
    }
  }

  let nextCallback = 1;
  (window as any).__TAURI_INTERNALS__ = {
    invoke(cmd: string, args?: Record<string, unknown>) {
      calls.push({ cmd, args: args ?? {} });
      return Promise.resolve(mockInvoke(cmd, args ?? {}));
    },
    transformCallback: () => nextCallback++,
    unregisterCallback() {},
    // Each frame name becomes its own inline picture, so tests can tell which
    // frame a step is showing from the <img src>.
    convertFileSrc: (path: string) =>
      `data:image/svg+xml,${encodeURIComponent(
        `<svg xmlns="http://www.w3.org/2000/svg" width="1600" height="1000"><rect width="1600" height="1000" fill="#ccd"/><text x="40" y="120" font-size="96">${path}</text></svg>`,
      )}`,
    metadata: { currentWindow: { label: "main" } },
  };
  (window as any).__TAURI_EVENT_PLUGIN_INTERNALS__ = {
    registerListener() {},
    unregisterListener() {},
  };
}

export const test = base.extend<{ app: App; language: string }>({
  /** Writing language in Settings; picks which fixed wording the editor shows. */
  language: ["English", { option: true }],
  app: async ({ page, language }, use) => {
    const errors: string[] = [];
    page.on("pageerror", (error) => errors.push(String(error)));

    await page.addInitScript(installTauriMock, language);
    // Match the desktop app: external origins (icon APIs, fonts) are unavailable.
    await page.route("**/*", (route) =>
      new URL(route.request().url()).origin === "http://localhost:1420"
        ? route.continue()
        : route.abort(),
    );
    await page.goto("/", { waitUntil: "networkidle" });

    await use({
      page,
      calls: (cmd) =>
        page.evaluate(
          (name) => (window as any).__calls.filter((call: Call) => !name || call.cmd === name),
          cmd,
        ),
      openGuide: async () => {
        await page.getByRole("button", { name: /^Sample guide/ }).click();
        await expect(page.getByRole("textbox", { name: "Step description" }).first()).toBeVisible();
      },
    });

    expect(errors).toEqual([]);
  },
});

export { expect };
