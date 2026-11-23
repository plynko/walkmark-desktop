import { addCollection } from "@iconify/svelte";
import { mount } from "svelte";

import App from "./App.svelte";
import heroicons from "./lib/icons/heroicons.json";
import lucide from "./lib/icons/lucide.json";
import { isHudWindow, isMacOS } from "./lib/window";
import "./app.css";

// Register the bundled icon subsets before either Tauri window mounts.
// The desktop CSP blocks Iconify's remote API.
addCollection(heroicons);
addCollection(lucide);

if (isHudWindow()) {
  document.documentElement.dataset.window = "hud";
}

if (isMacOS()) {
  document.documentElement.dataset.platform = "macos";
}

document.addEventListener("contextmenu", (event) => {
  event.preventDefault();
});

mount(App, { target: document.getElementById("root")! });
