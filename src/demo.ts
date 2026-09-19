import "./no-context-menu";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  clampDemoOptions,
  demoEntryHtml,
  demoLengthLabel,
  demoPaceHint,
  demoProgressLabel,
  demoScenarioLabel,
  escapeHtml,
  type DemoLogEntry,
  type DemoOptions,
  type DemoScenario,
} from "./logic";

type DemoProgress = {
  running: boolean;
  scenario: string;
  step: number;
  total: number;
};

let scenarios: DemoScenario[] = [];
let preview: DemoScenario | null = null;
let running = false;
let previewTimer: ReturnType<typeof setTimeout> | null = null;

function $(id: string): HTMLElement {
  const el = document.getElementById(id);
  if (!el) throw new Error(`missing #${id}`);
  return el;
}

function selectedScenario(): DemoScenario | null {
  const id = ($("demo-scenario") as HTMLSelectElement).value;
  return scenarios.find((scenario) => scenario.id === id) ?? null;
}

function readOptions(): DemoOptions {
  return clampDemoOptions({
    clerics: Number(($("demo-clerics") as HTMLInputElement).value),
    maxClerics: Number(($("demo-max-clerics") as HTMLInputElement).value),
    minutes: Number(($("demo-minutes") as HTMLInputElement).value),
  });
}

function speed(): number {
  return Number(($("demo-speed") as HTMLSelectElement).value) || 1;
}

function renderControls(progress: DemoProgress) {
  running = progress.running;
  const configurable = selectedScenario()?.configurable ?? false;
  ($("demo-start") as HTMLButtonElement).disabled = running;
  ($("demo-stop") as HTMLButtonElement).disabled = !running;
  ($("demo-scenario") as HTMLSelectElement).disabled = running;
  ($("demo-speed") as HTMLSelectElement).disabled = running;
  // The roster belongs to the raid scenario; a scripted pull carries its own.
  ($("demo-options") as HTMLFieldSetElement).hidden = !configurable;
  ($("demo-options") as HTMLFieldSetElement).disabled = running;
  $("demo-progress").textContent = demoProgressLabel(progress);
}

async function refreshPreview() {
  const scenario = selectedScenario();
  if (!scenario) return;
  const options = readOptions();
  try {
    preview = await invoke<DemoScenario>("demo_preview", {
      scenario: scenario.id,
      options,
    });
  } catch {
    preview = scenario;
  }
  $("demo-length").textContent = demoLengthLabel(preview, speed());
  $("demo-pace").textContent = scenario.configurable ? demoPaceHint(options) : "";
}

function schedulePreview() {
  if (previewTimer != null) clearTimeout(previewTimer);
  previewTimer = setTimeout(() => {
    previewTimer = null;
    void refreshPreview();
  }, 200);
}

function renderScenario() {
  const scenario = selectedScenario();
  $("demo-description").textContent = scenario?.description ?? "";
  if (!running) {
    renderControls({
      running: false,
      scenario: scenario?.id ?? "",
      step: 0,
      total: preview?.steps ?? scenario?.steps ?? 0,
    });
  }
  void refreshPreview();
}

function appendEntry(entry: DemoLogEntry) {
  const log = $("demo-log");
  log.insertAdjacentHTML("beforeend", demoEntryHtml(entry));
  log.scrollTop = log.scrollHeight;
}

async function start() {
  const scenario = selectedScenario();
  if (!scenario) return;
  try {
    await invoke("start_demo", {
      scenario: scenario.id,
      speed: speed(),
      options: readOptions(),
    });
  } catch (err) {
    $("demo-progress").textContent = String(err);
  }
}

window.addEventListener("DOMContentLoaded", () => {
  $("demo-start").addEventListener("click", () => {
    void start();
  });
  $("demo-stop").addEventListener("click", () => {
    void invoke("stop_demo");
  });
  $("demo-clear").addEventListener("click", () => {
    $("demo-log").innerHTML = "";
  });
  $("demo-scenario").addEventListener("change", renderScenario);
  $("demo-speed").addEventListener("change", () => {
    void refreshPreview();
  });
  for (const id of ["demo-clerics", "demo-max-clerics", "demo-minutes"]) {
    $(id).addEventListener("input", schedulePreview);
    $(id).addEventListener("change", schedulePreview);
  }

  void (async () => {
    try {
      scenarios = await invoke<DemoScenario[]>("demo_scenarios");
    } catch (err) {
      $("demo-progress").textContent = String(err);
      return;
    }
    $("demo-scenario").innerHTML = scenarios
      .map(
        (scenario) =>
          `<option value="${escapeHtml(scenario.id)}">${escapeHtml(demoScenarioLabel(scenario))}</option>`,
      )
      .join("");
    renderScenario();

    await listen<DemoProgress>("demo-state", (event) => {
      renderControls(event.payload);
    });
    await listen<DemoLogEntry>("demo-log", (event) => {
      appendEntry(event.payload);
    });
  })();
});
