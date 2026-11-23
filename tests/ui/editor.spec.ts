import { test, expect } from "./fixtures";

test.describe("tooltips", () => {
  test("show the action and shortcut on hover", async ({ app }) => {
    const settings = app.page.getByRole("button", { name: "Settings", exact: true });
    await settings.hover();
    await expect(app.page.getByRole("tooltip", { name: /^Settings \(/ })).toBeVisible();
  });

  test("show on keyboard focus and close with Escape", async ({ app }) => {
    const settings = app.page.getByRole("button", { name: "Settings", exact: true });
    await settings.focus();
    await expect(app.page.getByRole("tooltip", { name: /^Settings \(/ })).toBeVisible();
    await app.page.keyboard.press("Escape");
    await expect(app.page.getByRole("tooltip")).toHaveCount(0);
  });
});

test.describe("choose another frame", () => {
  test("swaps the screenshot and leaves the text alone", async ({ app }) => {
    await app.openGuide();
    const first = app.page.locator("#step-s1");
    await expect(first.locator("img[alt^='Screenshot']")).toHaveAttribute("src", /frame-b/);

    await first.getByRole("button", { name: "Choose another frame" }).click();
    await expect(first.getByRole("button", { name: "Current frame" })).toBeVisible();
    await first.getByRole("button", { name: "Use frame 1" }).click();

    await expect(first.locator("img[alt^='Screenshot']")).toHaveAttribute("src", /frame-a/);
    const [update] = await app.calls("update_step");
    expect(update.args).toEqual({ id: "s1", patch: { frame: "frame-a.png" } });
    await expect(first.getByRole("textbox", { name: "Step description" })).toHaveValue("Click **File**.");
  });

  test("is only offered when the step has nearby frames", async ({ app }) => {
    await app.openGuide();
    await expect(app.page.locator("#step-s1").getByRole("button", { name: "Choose another frame" })).toBeVisible();
    await expect(app.page.locator("#step-s2").getByRole("button", { name: "Choose another frame" })).toHaveCount(0);
  });
});

test.describe("add step", () => {
  const steps = (app: { page: import("@playwright/test").Page }) => app.page.locator("li[id^='step-']");

  test("imports an image after the focused step", async ({ app }) => {
    await app.openGuide();
    await expect(steps(app)).toHaveCount(2);
    await app.page.getByRole("button", { name: "Add step" }).first().click();
    await app.page.getByRole("menuitem", { name: "Import image" }).click();
    await expect(steps(app)).toHaveCount(3);
    expect(await app.calls("import_step")).toHaveLength(1);
  });

  test("pastes an image from the clipboard", async ({ app }) => {
    await app.openGuide();
    await app.page.getByRole("button", { name: "Add step" }).first().click();
    await app.page.getByRole("menuitem", { name: "Paste from clipboard" }).click();
    await expect(steps(app)).toHaveCount(3);
    expect(await app.calls("paste_step")).toHaveLength(1);
  });

  test("captures a single screenshot from a chosen source", async ({ app }) => {
    await app.openGuide();
    await app.page.getByRole("button", { name: "Add step" }).first().click();
    await app.page.getByRole("menuitem", { name: "Capture screenshot" }).click();
    await app.page.getByRole("button", { name: /Built-in display/ }).click();
    await app.page.getByRole("button", { name: /^Capture in/ }).click();
    await expect(steps(app)).toHaveCount(3);
    const [capture] = await app.calls("capture_step");
    expect(capture.args).toMatchObject({ projectId: "guide-1", sourceId: "monitor-1" });
  });
});

test.describe("step text", () => {
  test("the description grows with its content", async ({ app }) => {
    await app.openGuide();
    const field = app.page.locator("#step-s2").getByRole("textbox", { name: "Step description" });
    await field.click();
    const before = (await field.boundingBox())!.height;
    await field.fill(Array.from({ length: 8 }, (_, i) => `Line ${i + 1}`).join("\n"));
    await expect.poll(async () => (await field.boundingBox())!.height).toBeGreaterThan(before * 3);
    await field.fill("");
    await expect.poll(async () => (await field.boundingBox())!.height).toBeLessThanOrEqual(before + 1);
  });

  test("previews Markdown and returns to editing", async ({ app }) => {
    await app.openGuide();
    const card = app.page.locator("#step-s1");
    await card.getByRole("button", { name: "Preview step description as Markdown" }).click();
    const preview = card.getByRole("region", { name: "Step description preview" });
    await expect(preview.locator("strong")).toHaveText("File");
    await expect(card.getByRole("textbox", { name: "Step description" })).toBeHidden();

    await card.getByRole("button", { name: "Edit step description" }).click();
    await expect(card.getByRole("textbox", { name: "Step description" })).toBeFocused();
  });

  test("cannot preview an empty description", async ({ app }) => {
    await app.openGuide();
    await expect(
      app.page.locator("#step-s2").getByRole("button", { name: "Preview step description as Markdown" }),
    ).toBeDisabled();
  });
});

test.describe("settings", () => {
  test("switches the model provider without errors", async ({ app }) => {
    await app.page.getByRole("button", { name: "Settings", exact: true }).click();
    await app.page.getByRole("tab", { name: "Model" }).click();
    await app.page.getByRole("button", { name: "Google Gemini", exact: true }).click();
    await app.page.getByRole("option", { name: "On this Mac" }).click();
    await expect(app.page.getByRole("button", { name: "On this Mac", exact: true })).toBeVisible();
  });
});

test.describe("document details", () => {
  const meta = async (app: { calls(cmd?: string): Promise<{ args: Record<string, unknown> }[]> }) =>
    (await app.calls("update_project_meta")).map((call) => call.args.meta);

  test("shows the wording exports use until it is changed", async ({ app }) => {
    await app.openGuide();
    const page = app.page;
    await expect(page.getByRole("textbox", { name: "Guide label" })).toHaveAttribute("placeholder", "Step-by-step guide");
    await expect(page.getByRole("textbox", { name: "Prerequisites heading" })).toHaveAttribute("placeholder", "Before you start");
    await expect(page.getByRole("textbox", { name: "Version", exact: true })).toHaveValue("1.0");
    await expect(page.getByRole("textbox", { name: "Author", exact: true })).toHaveValue("");
  });

  test("the “Before you start” heading is editable", async ({ app }) => {
    await app.openGuide();
    const heading = app.page.getByRole("textbox", { name: "Prerequisites heading" });
    await heading.fill("Prima di iniziare");
    await heading.press("Enter");
    await expect.poll(() => meta(app)).toEqual([
      { labels: { guide: "", prerequisites: "Prima di iniziare", exported: "", version: "", author: "" } },
    ]);
  });

  test("the guide label above the title is editable", async ({ app }) => {
    await app.openGuide();
    const label = app.page.getByRole("textbox", { name: "Guide label" });
    await label.fill("Guida passo passo");
    await label.press("Enter");
    await expect.poll(async () => (await meta(app))[0]).toMatchObject({
      labels: { guide: "Guida passo passo" },
    });
  });

  test("version and author are saved", async ({ app }) => {
    await app.openGuide();
    const version = app.page.getByRole("textbox", { name: "Version", exact: true });
    await version.fill("2.3");
    await version.press("Enter");
    const author = app.page.getByRole("textbox", { name: "Author", exact: true });
    await author.fill("Erik");
    await author.press("Enter");
    await expect.poll(() => meta(app)).toEqual([{ version: "2.3" }, { author: "Erik" }]);
  });

  test("an unchanged field saves nothing", async ({ app }) => {
    await app.openGuide();
    const version = app.page.getByRole("textbox", { name: "Version", exact: true });
    await version.click();
    await version.press("Enter");
    expect(await meta(app)).toEqual([]);
  });

  test("translating fills every label from the AI", async ({ app }) => {
    await app.openGuide();
    await app.page.getByRole("button", { name: "Translate labels" }).click();
    await expect(app.page.getByRole("textbox", { name: "Guide label" })).toHaveValue("Guida passo passo");
    await expect(app.page.getByRole("textbox", { name: "Prerequisites heading" })).toHaveValue("Prima di iniziare");
    await expect(app.page.getByRole("textbox", { name: "Export date label" })).toHaveValue("Esportato il");
    await expect(app.page.getByRole("textbox", { name: "Version label" })).toHaveValue("Versione");
    await expect(app.page.getByRole("textbox", { name: "Author label" })).toHaveValue("Autore");
    expect(await app.calls("translate_labels")).toHaveLength(1);
  });
});

test.describe("export", () => {
  test("offers the details line as an option", async ({ app }) => {
    await app.openGuide();
    await app.page.getByRole("button", { name: "Export", exact: true }).click();
    await expect(app.page.getByText("Include date, version and author")).toBeVisible();
  });
});
