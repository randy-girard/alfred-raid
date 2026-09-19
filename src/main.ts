import "./no-context-menu";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import {
  escapeHtml,
  formatOffset,
  offsetClassName,
  formatSlot,
  spokenSlot,
  liveSnapshot as tickSnapshot,
  shouldChime,
  nextUpSpeech,
  shouldPlayClaimAlert,
  shouldShowYouBanner,
  youCastProgress,
  chainStateLabel,
  tankClockLabel,
  shouldShowWarning,
  isAlertEnabled,
  shouldSpeakWrongTarget,
  shouldSpeakAutoTake,
  shouldSpeakMetronome,
  firstRunSteps,
  setupStepLabel,
  commandListHtml,
  alertMode,
  eqDirStatusText,
  slotClassName,
  watchStatusLabel,
  CHAIN_COMMANDS,
  RAMPAGE_COMMANDS,
  type ChainSnapshot,
  type EqDirectoryProbe,
  type SetupStep,
  type WatchStatus,
} from "./logic";

type View = "chain" | "rampage" | "commands" | "settings";
const VIEWS: View[] = ["chain", "rampage", "commands", "settings"];

type RaidSnapshot = {
  chain: ChainSnapshot;
  rampage: ChainSnapshot;
};

type PanelIds = {
  tank: string;
  interval: string;
  state: string;
  youName: string;
  empty: string;
  slots: string;
  banner: string;
  eta: string;
  offset: string;
  progress: string;
};

type AppConfig = {
  eqDirectory: string;
  alwaysOnTop: boolean;
  soundEnabled: boolean;
  metronomeEnabled: boolean;
  soundLeadSeconds: number;
  intervalSeconds: number;
  castTimeSeconds: number;
  tailPollMs: number;
  setupComplete: boolean;
  chainTag: string;
  alertSlotTaken: boolean;
  alertWrongTarget: boolean;
  alertAutoTakeSound: boolean;
};

let raid: RaidSnapshot | null = null;
let currentView: View = "chain";
let config: AppConfig | null = null;
let watch: WatchStatus | null = null;
let lastChimeAt = { chain: 0, rampage: 0 };
let lastBeatTick: { chain: number | null; rampage: number | null } = {
  chain: null,
  rampage: null,
};
let lastSpokenNumber: { chain: number | null; rampage: number | null } = {
  chain: null,
  rampage: null,
};
let lastSpokenWrongTarget: { chain: string | null; rampage: string | null } = {
  chain: null,
  rampage: null,
};
let lastSpokenAutoTake: { chain: string | null; rampage: string | null } = {
  chain: null,
  rampage: null,
};
let lastClaimWarning: { chain: string | null; rampage: string | null } = {
  chain: null,
  rampage: null,
};
let dismissedWarning: { chain: string | null; rampage: string | null } = {
  chain: null,
  rampage: null,
};
let audioCtx: AudioContext | null = null;
let armedSound = false;
let eqDirTimer: ReturnType<typeof setTimeout> | null = null;
let setupEqTimer: ReturnType<typeof setTimeout> | null = null;
let settingsTimer: ReturnType<typeof setTimeout> | null = null;
let setupSteps: SetupStep[] = [];
let setupIndex = 0;

const EQ_DIR_HINT =
  "On first launch Alfred looks in common EQ, Steam, Wine, and CrossOver folders. Paste or browse if it missed yours.";

function $(id: string): HTMLElement {
  const el = document.getElementById(id);
  if (!el) throw new Error(`missing #${id}`);
  return el;
}

function isView(value: string | undefined): value is View {
  return VIEWS.includes(value as View);
}

function setView(view: View) {
  currentView = view;
  for (const name of VIEWS) {
    $(`view-${name}`).hidden = view !== name;
    $(`tab-${name}`).classList.toggle("is-active", view === name);
  }
}

function renderSetup() {
  const overlay = $("setup");
  const app = $("app");
  $("setup-error").textContent = "";
  if (setupSteps.length === 0) {
    overlay.hidden = true;
    app.inert = false;
    return;
  }
  overlay.hidden = false;
  app.inert = true;
  const step = setupSteps[setupIndex];
  $("setup-step").textContent = setupStepLabel(setupIndex, setupSteps.length);
  $("setup-eq").hidden = step !== "eq";
  $("setup-audio").hidden = step !== "audio";
  $("setup-audio-back").hidden = setupIndex === 0;
  if (step === "audio" && config) {
    const mode = alertMode(config);
    ($("setup-sound") as HTMLInputElement).checked = mode === "sound";
    ($("setup-metronome") as HTMLInputElement).checked = mode === "metronome";
    ($("setup-audio-none") as HTMLInputElement).checked = mode === "none";
  }
}

function goSetup(delta: number) {
  setupIndex = Math.max(0, Math.min(setupSteps.length - 1, setupIndex + delta));
  renderSetup();
}

function renderChain() {
  renderPanel(
    tickSnapshot(raid?.chain ?? null, Date.now()),
    {
      tank: "tank-name",
      interval: "interval",
      state: "chain-state",
      youName: "you-name",
      empty: "empty",
      slots: "slots",
      banner: "you-banner",
      eta: "you-eta",
      offset: "you-offset",
      progress: "you-progress",
    },
    "chain",
  );
  renderPanel(
    tickSnapshot(raid?.rampage ?? null, Date.now()),
    {
      tank: "r-tank-name",
      interval: "r-interval",
      state: "r-chain-state",
      youName: "r-you-name",
      empty: "r-empty",
      slots: "r-slots",
      banner: "r-you-banner",
      eta: "r-you-eta",
      offset: "r-you-offset",
      progress: "r-you-progress",
    },
    "rampage",
  );
  renderAlerts();
}

function renderPanel(
  live: ChainSnapshot | null,
  ids: PanelIds,
  kind: "chain" | "rampage",
) {
  const empty = $(ids.empty);
  const slotsEl = $(ids.slots);
  const banner = $(ids.banner) as HTMLDivElement;
  const audible = currentView === kind;

  $(ids.tank).textContent = live?.yourTank
    || (live?.tanks?.length ? live.tanks.map((tank) => tank.name).join(" · ") : live?.tank)
    || "—";
  const oneClock = Boolean(live?.yourTank) || (live?.tanks?.length ?? 0) <= 1;
  $(ids.interval).textContent = live
    ? oneClock
      ? `${live.intervalSeconds.toFixed(1)}s`
      : live.tanks.map((tank) => `${tank.intervalSeconds.toFixed(1)}s`).join(" · ")
    : "—";
  $(ids.state).textContent = live?.yourTank
    ? chainStateLabel({ running: live.running, armed: live.armed })
    : live?.tanks?.some((tank) => tank.running || tank.armed)
      ? live.tanks
          .map((tank) => `${tank.name} ${tankClockLabel(tank)}`)
          .join(" · ")
      : chainStateLabel({ running: live?.running ?? false, armed: live?.armed });
  $(ids.youName).textContent = live?.yourName || watch?.character || "—";

  if (!live || live.slots.length === 0) {
    slotsEl.innerHTML = "";
    empty.hidden = false;
  } else {
    empty.hidden = true;
    const groups = new Map<string, typeof live.slots>();
    for (const slot of live.slots) {
      const key = live.yourTank || slot.tank || live.tank || "";
      const list = groups.get(key) ?? [];
      list.push(slot);
      groups.set(key, list);
    }
    const showHeadings = !live.yourTank && groups.size > 1;
    slotsEl.innerHTML = [...groups.entries()]
      .map(([name, group]) => {
        const heading =
          showHeadings && name
            ? `<h2 class="tank-group-title">${escapeHtml(name)}</h2>`
            : "";
        const cards = group
          .map((slot) => {
            const width = Math.round(slot.progress * 1000) / 10;
            const castWidth = Math.round(slot.castProgress * 1000) / 10;
            const flags = [
              slot.isNext ? '<span class="flag next">Next</span>' : "",
              slot.skipped ? '<span class="flag skip">Skip</span>' : "",
            ].join("");
            const offset = formatOffset(slot.offsetSeconds);
            const hitMod = offsetClassName(slot.offsetSeconds);
            const tankRunning =
              live.tanks.find((tank) => tank.name === (slot.tank || name))?.running ?? live.running;
            const showCastBar =
              tankRunning &&
              slot.castRemainingSeconds > 0 &&
              Math.abs(slot.castRemainingSeconds - slot.remainingSeconds) > 0.05;
            const remainingLabel =
              slot.lastShoutMs || slot.lastCastMs || tankRunning
                ? `Next ${slot.remainingSeconds.toFixed(1)}s`
                : "—";
            const hitLabel = offset ? `Last hit ${offset}` : "No hit yet";
            return `<article class="${slotClassName(slot)}">
          <div class="slot-head">
            <span class="num">${formatSlot(slot.number, live.slotFormat)}</span>
            <span class="player">${escapeHtml(slot.player)}</span>
            <span class="target">${slot.target ? escapeHtml(slot.target) : ""}</span>
            <span class="flags">${flags}</span>
          </div>
          <div class="bar-row">
            <div class="bar"><span style="width:${width}%"></span></div>
            <span class="eta">${remainingLabel}</span>
          </div>
          ${
            showCastBar
              ? `<div class="bar-row cast">
            <div class="bar"><span style="width:${castWidth}%"></span></div>
            <span class="cast-eta">CH ${slot.castRemainingSeconds.toFixed(1)}s</span>
          </div>`
              : ""
          }
          <div class="hit${hitMod ? ` ${hitMod}` : ""}">${hitLabel}</div>
        </article>`;
          })
          .join("");
        return `${heading}${cards}`;
      })
      .join("");
  }

  const eta = live?.running ? live.youCastIn : live?.youAreNextIn;
  if (
    shouldShowYouBanner({
      running: live?.running ?? false,
      youCastIn: live?.youCastIn,
      youAreNextIn: live?.youAreNextIn,
    })
  ) {
    banner.hidden = false;
    $(ids.eta).textContent = (eta ?? 0).toFixed(1);
    const offsetEl = $(ids.offset);
    offsetEl.textContent = live?.youLastOffset != null ? ` · ${formatOffset(live.youLastOffset)}` : "";
    const width = Math.round(youCastProgress(live) * 1000) / 10;
    $(ids.progress).style.width = `${width}%`;
    if (audible && eta != null) maybeChime(eta, kind);
  } else {
    banner.hidden = true;
  }
  if (audible) maybeMetronome(live, kind);
}

function renderAlerts() {
  const host = $("alerts");
  const items: Array<{
    kind: "chain" | "rampage";
    warning: string;
    urgent: boolean;
    warningKind: ChainSnapshot["warningKind"];
    warningSpeech: string | null;
  }> = [];
  for (const kind of ["chain", "rampage"] as const) {
    const live = kind === "chain"
      ? tickSnapshot(raid?.chain ?? null, Date.now())
      : tickSnapshot(raid?.rampage ?? null, Date.now());
    const warning = live?.warning ?? null;
    if (!warning) {
      dismissedWarning[kind] = null;
      lastClaimWarning[kind] = null;
      lastSpokenWrongTarget[kind] = null;
      lastSpokenAutoTake[kind] = null;
      continue;
    }
    if (
      !isAlertEnabled({
        kind: live?.warningKind,
        alertSlotTaken: config?.alertSlotTaken ?? true,
        alertWrongTarget: config?.alertWrongTarget ?? true,
      })
    ) {
      continue;
    }
    if (!shouldShowWarning({ warning, dismissed: dismissedWarning[kind] })) {
      continue;
    }
    items.push({
      kind,
      warning,
      urgent: live?.warningUrgent ?? false,
      warningKind: live?.warningKind ?? "other",
      warningSpeech: live?.warningSpeech ?? null,
    });
  }

  host.hidden = items.length === 0;
  host.innerHTML = items
    .map((item) => {
      const cls = item.urgent ? "banner danger dismissable" : "banner warn dismissable";
      return `<div class="${cls}" data-warning="${item.kind}" role="alert">
        <span>${escapeHtml(item.warning)}</span>
        <button type="button" class="banner-dismiss" aria-label="Dismiss">×</button>
      </div>`;
    })
    .join("");

  for (const item of items) {
    if (item.warningKind === "slotTaken") {
      maybeClaimAlert(item.warning, item.urgent, item.kind);
    }
    maybeWrongTargetSpeech(item.warning, item.warningKind, item.urgent, item.kind);
    maybeAutoTakeSpeech(item.warningSpeech, item.warningKind, item.urgent, item.kind);
  }
}

function renderStatus() {
  const dot = $("watch-dot");
  const label = $("watch-status");
  const status = watchStatusLabel(watch);
  dot.className = status.kind === "ok" ? "dot ok" : "dot warn";
  label.textContent = status.text;
  $("logs-path").textContent = watch?.logsCanonical || watch?.logsPath || "—";
  $("active-log").textContent = watch?.activeLog || "—";
  $("settings-character").textContent = watch?.character || "—";
}

function fillSettings(cfg: AppConfig) {
  const active = document.activeElement;
  const eq = $("eq-directory") as HTMLInputElement;
  if (active !== eq) {
    eq.value = cfg.eqDirectory;
  }
  ($("always-on-top") as HTMLInputElement).checked = cfg.alwaysOnTop;
  const mode = alertMode(cfg);
  ($("sound-enabled") as HTMLInputElement).checked = mode === "sound";
  ($("metronome-enabled") as HTMLInputElement).checked = mode === "metronome";
  ($("audio-none") as HTMLInputElement).checked = mode === "none";
  const lead = $("sound-lead") as HTMLInputElement;
  lead.disabled = mode !== "sound";
  const interval = $("interval-seconds") as HTMLInputElement;
  const cast = $("cast-time") as HTMLInputElement;
  const tag = $("chain-tag") as HTMLInputElement;
  if (active !== lead) lead.value = String(cfg.soundLeadSeconds);
  if (active !== interval) interval.value = String(cfg.intervalSeconds);
  if (active !== cast) cast.value = String(cfg.castTimeSeconds);
  if (active !== tag) tag.value = cfg.chainTag || "GG";
  ($("alert-slot-taken") as HTMLInputElement).checked = cfg.alertSlotTaken;
  ($("alert-wrong-target") as HTMLInputElement).checked = cfg.alertWrongTarget;
  ($("alert-auto-take-sound") as HTMLInputElement).checked = cfg.alertAutoTakeSound;
}

function optionalNumber(id: string, min: number): number | undefined {
  const raw = ($(id) as HTMLInputElement).value.trim();
  if (raw === "" || raw === "." || raw === "-") return undefined;
  const n = Number(raw);
  if (!Number.isFinite(n) || n < min) return undefined;
  return n;
}

function readSettingsPatch(): Record<string, unknown> {
  return {
    alwaysOnTop: ($("always-on-top") as HTMLInputElement).checked,
    soundEnabled: ($("sound-enabled") as HTMLInputElement).checked,
    metronomeEnabled: ($("metronome-enabled") as HTMLInputElement).checked,
    soundLeadSeconds: optionalNumber("sound-lead", 0),
    intervalSeconds: optionalNumber("interval-seconds", 0.1),
    castTimeSeconds: optionalNumber("cast-time", 0.1),
    chainTag: ($("chain-tag") as HTMLInputElement).value,
    alertSlotTaken: ($("alert-slot-taken") as HTMLInputElement).checked,
    alertWrongTarget: ($("alert-wrong-target") as HTMLInputElement).checked,
    alertAutoTakeSound: ($("alert-auto-take-sound") as HTMLInputElement).checked,
  };
}

async function saveSettingsFromForm() {
  const status = $("save-status");
  try {
    config = await invoke<AppConfig>("save_settings", { patch: readSettingsPatch() });
    fillSettings(config);
    status.textContent = "";
  } catch (err) {
    status.textContent = String(err);
  }
}

function scheduleSaveSettings() {
  if (settingsTimer != null) {
    clearTimeout(settingsTimer);
  }
  settingsTimer = setTimeout(() => {
    settingsTimer = null;
    void saveSettingsFromForm();
  }, 400);
}

function showEqDirStatus(probe: EqDirectoryProbe | null) {
  const el = $("eq-dir-status");
  if (!probe) {
    el.className = "";
    el.textContent = EQ_DIR_HINT;
    return;
  }
  const status = eqDirStatusText(probe);
  el.className = status.kind;
  el.textContent = status.text;
}

async function applyEqDirectory(raw: string) {
  const input = $("eq-directory") as HTMLInputElement;
  if (!raw.trim()) {
    showEqDirStatus(null);
    if (config?.eqDirectory) {
      config = await invoke<AppConfig>("save_settings", { patch: { eqDirectory: "" } });
    }
    return;
  }
  const probe = await invoke<EqDirectoryProbe>("inspect_eq_directory", { path: raw });
  if (probe.ok && probe.path) {
    input.value = probe.path;
  }
  showEqDirStatus(probe);
  if (!probe.ok || probe.path === config?.eqDirectory) {
    return;
  }
  config = await invoke<AppConfig>("save_settings", { patch: { eqDirectory: probe.path } });
}

function scheduleEqDirectoryApply() {
  if (eqDirTimer != null) {
    clearTimeout(eqDirTimer);
  }
  eqDirTimer = setTimeout(() => {
    eqDirTimer = null;
    void applyEqDirectory(($("eq-directory") as HTMLInputElement).value).catch((err) => {
      $("eq-dir-status").className = "warn";
      $("eq-dir-status").textContent = String(err);
    });
  }, 400);
}

function showSetupEqStatus(probe: EqDirectoryProbe | null) {
  const el = $("setup-eq-status");
  const next = $("setup-eq-next") as HTMLButtonElement;
  if (!probe) {
    el.className = "warn";
    el.textContent = "Paste or browse to your EverQuest folder.";
    next.disabled = true;
    return;
  }
  const status = eqDirStatusText(probe);
  el.className = status.kind;
  el.textContent = status.text;
  next.disabled = !probe.ok;
}

async function applySetupEqDirectory(raw: string) {
  const input = $("setup-eq-directory") as HTMLInputElement;
  if (!raw.trim()) {
    showSetupEqStatus(null);
    return;
  }
  const probe = await invoke<EqDirectoryProbe>("inspect_eq_directory", { path: raw });
  if (probe.ok && probe.path) {
    input.value = probe.path;
    ($("eq-directory") as HTMLInputElement).value = probe.path;
  }
  showSetupEqStatus(probe);
  if (!probe.ok || probe.path === config?.eqDirectory) {
    return;
  }
  config = await invoke<AppConfig>("save_settings", { patch: { eqDirectory: probe.path } });
  fillSettings(config);
}

function scheduleSetupEqDirectoryApply() {
  if (setupEqTimer != null) {
    clearTimeout(setupEqTimer);
  }
  setupEqTimer = setTimeout(() => {
    setupEqTimer = null;
    void applySetupEqDirectory(($("setup-eq-directory") as HTMLInputElement).value).catch(
      (err) => {
        $("setup-eq-status").className = "warn";
        $("setup-eq-status").textContent = String(err);
        ($("setup-eq-next") as HTMLButtonElement).disabled = true;
      },
    );
  }, 400);
}

async function finishSetup() {
  const mode =
    document.querySelector<HTMLInputElement>('input[name="setup-alert-mode"]:checked')
      ?.value ?? "sound";
  config = await invoke<AppConfig>("save_settings", {
    patch: {
      soundEnabled: mode === "sound",
      metronomeEnabled: mode === "metronome",
      setupComplete: true,
    },
  });
  fillSettings(config);
  setupSteps = [];
  renderSetup();
  unlockAudio();
  setView("chain");
}

function unlockAudio() {
  if (!audioCtx) {
    audioCtx = new AudioContext();
  }
  if (audioCtx.state === "suspended") {
    void audioCtx.resume();
  }
  armedSound = true;
}

function maybeMetronome(live: ChainSnapshot | null, kind: "chain" | "rampage") {
  if (!live?.running) {
    lastBeatTick[kind] = null;
    lastSpokenNumber[kind] = null;
  }
  if (
    !shouldSpeakMetronome({
      enabled: config?.metronomeEnabled ?? false,
      running: live?.running ?? false,
      currentNumber: live?.currentNumber,
      beatTick: live?.beatTick,
      lastBeatTick: lastBeatTick[kind],
      lastSpokenNumber: lastSpokenNumber[kind],
    })
  ) {
    return;
  }
  lastBeatTick[kind] = live?.beatTick ?? null;
  const number = live?.currentNumber;
  lastSpokenNumber[kind] = number ?? null;
  if (number == null || typeof speechSynthesis === "undefined") return;
  speechSynthesis.cancel();
  const utterance = new SpeechSynthesisUtterance(spokenSlot(number, live?.slotFormat));
  utterance.rate = 1.15;
  speechSynthesis.speak(utterance);
}

function maybeClaimAlert(warning: string, urgent: boolean, kind: "chain" | "rampage") {
  if (
    !shouldPlayClaimAlert({
      warning,
      urgent,
      lastWarning: lastClaimWarning[kind],
      armed: armedSound,
    })
  ) {
    return;
  }
  lastClaimWarning[kind] = warning;
  playClaimAlert();
}

function maybeWrongTargetSpeech(
  warning: string,
  kind: ChainSnapshot["warningKind"],
  urgent: boolean,
  source: "chain" | "rampage",
) {
  if (
    !shouldSpeakWrongTarget({
      enabled: config?.alertWrongTarget ?? true,
      kind,
      urgent,
      warning,
      lastSpoken: lastSpokenWrongTarget[source],
    })
  ) {
    return;
  }
  lastSpokenWrongTarget[source] = warning;
  if (typeof speechSynthesis === "undefined") return;
  speechSynthesis.cancel();
  const utterance = new SpeechSynthesisUtterance("Wrong target");
  utterance.rate = 1.1;
  speechSynthesis.speak(utterance);
}

function maybeAutoTakeSpeech(
  speech: string | null,
  kind: ChainSnapshot["warningKind"],
  urgent: boolean,
  source: "chain" | "rampage",
) {
  if (
    !shouldSpeakAutoTake({
      enabled: config?.alertAutoTakeSound ?? true,
      kind,
      urgent,
      speech,
      lastSpoken: lastSpokenAutoTake[source],
    })
  ) {
    return;
  }
  lastSpokenAutoTake[source] = speech ?? null;
  if (!speech || typeof speechSynthesis === "undefined") return;
  speechSynthesis.cancel();
  const utterance = new SpeechSynthesisUtterance(speech);
  utterance.rate = 1.1;
  speechSynthesis.speak(utterance);
}

function playClaimAlert() {
  if (!audioCtx) return;
  const ctx = audioCtx;
  const beep = (at: number, freq: number, duration: number) => {
    const osc = ctx.createOscillator();
    const gain = ctx.createGain();
    osc.type = "square";
    osc.frequency.setValueAtTime(freq, ctx.currentTime + at);
    gain.gain.setValueAtTime(0.0001, ctx.currentTime + at);
    gain.gain.exponentialRampToValueAtTime(0.22, ctx.currentTime + at + 0.02);
    gain.gain.exponentialRampToValueAtTime(0.0001, ctx.currentTime + at + duration);
    osc.connect(gain);
    gain.connect(ctx.destination);
    osc.start(ctx.currentTime + at);
    osc.stop(ctx.currentTime + at + duration + 0.02);
  };
  beep(0, 440, 0.18);
  beep(0.22, 330, 0.22);
  beep(0.48, 440, 0.28);
}

function maybeChime(eta: number, kind: "chain" | "rampage") {
  const lead = config?.soundLeadSeconds ?? 2;
  if (
    !shouldChime({
      eta,
      lead,
      lastChimeAt: lastChimeAt[kind],
      now: Date.now(),
      soundEnabled: config?.soundEnabled ?? false,
      armed: armedSound,
    })
  ) {
    return;
  }
  lastChimeAt[kind] = Date.now();
  if (typeof speechSynthesis === "undefined") return;
  speechSynthesis.cancel();
  const utterance = new SpeechSynthesisUtterance(nextUpSpeech(eta));
  utterance.rate = 1.15;
  speechSynthesis.speak(utterance);
}

async function loadInitial() {
  config = await invoke<AppConfig>("get_config");
  raid = await invoke<RaidSnapshot>("get_snapshot");
  watch = await invoke<WatchStatus>("get_watch_status");
  $("config-path").textContent = await invoke<string>("get_config_path");
  fillSettings(config);
  renderStatus();
  renderChain();
  if (config.eqDirectory) {
    void invoke<EqDirectoryProbe>("inspect_eq_directory", { path: config.eqDirectory }).then(
      showEqDirStatus,
    );
  }
  setupSteps = firstRunSteps({
    setupComplete: config.setupComplete,
    eqDirectory: config.eqDirectory,
  });
  setupIndex = 0;
  renderSetup();
}

window.addEventListener("DOMContentLoaded", () => {
  $("command-list").innerHTML = commandListHtml(CHAIN_COMMANDS);
  $("rampage-command-list").innerHTML = commandListHtml(RAMPAGE_COMMANDS);

  document.querySelectorAll<HTMLButtonElement>(".tab").forEach((tab) => {
    tab.addEventListener("click", () => {
      if (isView(tab.dataset.view)) setView(tab.dataset.view);
    });
  });

  $("browse-eq").addEventListener("click", async () => {
    const selected = await open({
      directory: true,
      multiple: false,
      title: "Select your EverQuest directory",
    });
    if (typeof selected === "string") {
      ($("eq-directory") as HTMLInputElement).value = selected;
      void applyEqDirectory(selected).catch((err) => {
        $("eq-dir-status").className = "warn";
        $("eq-dir-status").textContent = String(err);
      });
    }
  });

  $("setup-browse-eq").addEventListener("click", async () => {
    const selected = await open({
      directory: true,
      multiple: false,
      title: "Select your EverQuest directory",
    });
    if (typeof selected === "string") {
      ($("setup-eq-directory") as HTMLInputElement).value = selected;
      void applySetupEqDirectory(selected).catch((err) => {
        $("setup-eq-status").className = "warn";
        $("setup-eq-status").textContent = String(err);
        ($("setup-eq-next") as HTMLButtonElement).disabled = true;
      });
    }
  });

  const setupEqInput = $("setup-eq-directory") as HTMLInputElement;
  setupEqInput.addEventListener("paste", () => scheduleSetupEqDirectoryApply());
  setupEqInput.addEventListener("input", () => scheduleSetupEqDirectoryApply());
  setupEqInput.addEventListener("change", () => scheduleSetupEqDirectoryApply());
  setupEqInput.addEventListener("keydown", (event) => {
    if (event.key === "Enter" && !($("setup-eq-next") as HTMLButtonElement).disabled) {
      event.preventDefault();
      goSetup(1);
    }
  });
  $("setup-eq-next").addEventListener("click", () => goSetup(1));
  $("setup-audio-back").addEventListener("click", () => goSetup(-1));
  $("setup-audio-done").addEventListener("click", () => {
    void finishSetup().catch((err) => {
      $("setup-error").textContent = String(err);
    });
  });

  const eqInput = $("eq-directory") as HTMLInputElement;
  eqInput.addEventListener("paste", () => scheduleEqDirectoryApply());
  eqInput.addEventListener("input", () => scheduleEqDirectoryApply());
  eqInput.addEventListener("change", () => scheduleEqDirectoryApply());

  $("always-on-top").addEventListener("change", () => {
    void saveSettingsFromForm();
  });
  for (const id of ["alert-slot-taken", "alert-wrong-target", "alert-auto-take-sound"]) {
    $(id).addEventListener("change", () => {
      void saveSettingsFromForm();
    });
  }
  $("open-tester").addEventListener("click", () => {
    void invoke("open_tester").catch((err) => {
      $("save-status").textContent = String(err);
    });
  });
  document.querySelectorAll<HTMLInputElement>('input[name="alert-mode"]').forEach((input) => {
    input.addEventListener("change", () => {
      void saveSettingsFromForm();
    });
  });
  for (const id of ["sound-lead", "interval-seconds", "cast-time", "chain-tag"]) {
    const input = $(id);
    input.addEventListener("input", () => scheduleSaveSettings());
    input.addEventListener("change", () => {
      if (settingsTimer != null) {
        clearTimeout(settingsTimer);
        settingsTimer = null;
      }
      void saveSettingsFromForm();
    });
  }

  document.addEventListener("pointerdown", unlockAudio, { once: true });

  $("alerts").addEventListener("click", (event) => {
    const target = event.target as HTMLElement | null;
    const el = target?.closest<HTMLElement>("[data-warning]");
    if (!el) return;
    const kind = el.dataset.warning;
    if (kind !== "chain" && kind !== "rampage") return;
    const live = kind === "chain" ? raid?.chain : raid?.rampage;
    if (live?.warning) dismissedWarning[kind] = live.warning;
    renderChain();
  });

  void (async () => {
    await loadInitial();
    await listen<RaidSnapshot>("raid-updated", (event) => {
      raid = event.payload;
      renderChain();
    });
    await listen<WatchStatus>("watch-status", (event) => {
      watch = event.payload;
      renderStatus();
    });
    await listen<AppConfig>("config-updated", (event) => {
      config = event.payload;
      fillSettings(config);
    });
    await listen<string>("open-view", (event) => {
      if (isView(event.payload)) setView(event.payload);
    });
  })();

  const tick = () => {
    renderChain();
    requestAnimationFrame(tick);
  };
  requestAnimationFrame(tick);
});
