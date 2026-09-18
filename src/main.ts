import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import {
  escapeHtml,
  formatOffset,
  formatSlot,
  liveSnapshot as tickSnapshot,
  shouldChime,
  shouldPlayClaimAlert,
  shouldShowYouBanner,
  shouldSpeakMetronome,
  commandListHtml,
  alertMode,
  eqDirStatusText,
  slotClassName,
  watchStatusLabel,
  CHAIN_COMMANDS,
  RAMPAGE_COMMANDS,
  type ChainSnapshot,
  type EqDirectoryProbe,
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
  warning: string;
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
let lastClaimWarning: { chain: string | null; rampage: string | null } = {
  chain: null,
  rampage: null,
};
let audioCtx: AudioContext | null = null;
let armedSound = false;
let eqDirTimer: ReturnType<typeof setTimeout> | null = null;
let settingsTimer: ReturnType<typeof setTimeout> | null = null;

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
      warning: "warning",
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
      warning: "r-warning",
    },
    "rampage",
  );
}

function renderPanel(
  live: ChainSnapshot | null,
  ids: PanelIds,
  kind: "chain" | "rampage",
) {
  const empty = $(ids.empty);
  const slotsEl = $(ids.slots);
  const banner = $(ids.banner) as HTMLDivElement;
  const warning = $(ids.warning);
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
    ? live.running
      ? "Running"
      : "Stopped"
    : live?.tanks?.some((tank) => tank.running)
      ? live.tanks
          .map((tank) => `${tank.name} ${tank.running ? "run" : "stop"}`)
          .join(" · ")
      : live?.running
        ? "Running"
        : "Stopped";
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
            const flags = [
              slot.isCurrent ? '<span class="flag now">Now</span>' : "",
              slot.isNext ? '<span class="flag next">Next</span>' : "",
              slot.skipped ? '<span class="flag skip">Skip</span>' : "",
            ].join("");
            const offset = formatOffset(slot.offsetSeconds);
            const offsetClass =
              slot.offsetSeconds == null
                ? "offset"
                : slot.offsetSeconds > 0.005
                  ? "offset late"
                  : slot.offsetSeconds < -0.005
                    ? "offset early"
                    : "offset";
            const tankRunning =
              live.tanks.find((tank) => tank.name === (slot.tank || name))?.running ?? live.running;
            return `<article class="${slotClassName(slot)}">
          <div class="slot-head">
            <span class="num">${formatSlot(slot.number, live.slotFormat)}</span>
            <span class="player">${escapeHtml(slot.player)}</span>
            <span class="target">${slot.target ? escapeHtml(slot.target) : ""}</span>
            <span class="flags">${flags}</span>
          </div>
          <div class="bar-row">
            <div class="bar"><span style="width:${width}%"></span></div>
            <span class="${offsetClass}">${offset || "—"}</span>
          </div>
          <div class="time">${slot.lastShoutMs || tankRunning ? `${slot.remainingSeconds.toFixed(1)}s remaining` : "Waiting"}</div>
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
    offsetEl.textContent = live?.youLastOffset != null ? ` · last ${formatOffset(live.youLastOffset)}` : "";
    if (audible && eta != null) maybeChime(eta, kind);
  } else {
    banner.hidden = true;
  }
  if (audible) maybeMetronome(live, kind);

  if (live?.warning) {
    warning.hidden = false;
    warning.className = live.warningUrgent ? "banner danger" : "banner warn";
    warning.textContent = live.warning;
    if (audible) maybeClaimAlert(live.warning, live.warningUrgent, kind);
  } else {
    warning.hidden = true;
    warning.className = "banner warn";
    warning.textContent = "";
    lastClaimWarning[kind] = null;
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
  const interval = $("interval-seconds") as HTMLInputElement;
  const cast = $("cast-time") as HTMLInputElement;
  if (active !== lead) lead.value = String(cfg.soundLeadSeconds);
  if (active !== interval) interval.value = String(cfg.intervalSeconds);
  if (active !== cast) cast.value = String(cfg.castTimeSeconds);
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
  const utterance = new SpeechSynthesisUtterance(formatSlot(number, live?.slotFormat));
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
  if (
    !shouldChime({
      eta,
      lead: config?.soundLeadSeconds ?? 2,
      lastChimeAt: lastChimeAt[kind],
      now: Date.now(),
      soundEnabled: config?.soundEnabled ?? false,
      armed: armedSound,
    })
  ) {
    return;
  }
  if (!audioCtx) return;
  lastChimeAt[kind] = Date.now();
  const ctx = audioCtx;
  const osc = ctx.createOscillator();
  const gain = ctx.createGain();
  osc.type = "sine";
  osc.frequency.setValueAtTime(784, ctx.currentTime);
  osc.frequency.setValueAtTime(1174, ctx.currentTime + 0.12);
  gain.gain.setValueAtTime(0.0001, ctx.currentTime);
  gain.gain.exponentialRampToValueAtTime(0.18, ctx.currentTime + 0.02);
  gain.gain.exponentialRampToValueAtTime(0.0001, ctx.currentTime + 0.38);
  osc.connect(gain);
  gain.connect(ctx.destination);
  osc.start();
  osc.stop(ctx.currentTime + 0.4);
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
  if (!config.eqDirectory) {
    setView("settings");
  }
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

  const eqInput = $("eq-directory") as HTMLInputElement;
  eqInput.addEventListener("paste", () => scheduleEqDirectoryApply());
  eqInput.addEventListener("input", () => scheduleEqDirectoryApply());
  eqInput.addEventListener("change", () => scheduleEqDirectoryApply());

  $("always-on-top").addEventListener("change", () => {
    void saveSettingsFromForm();
  });
  document.querySelectorAll<HTMLInputElement>('input[name="alert-mode"]').forEach((input) => {
    input.addEventListener("change", () => {
      void saveSettingsFromForm();
    });
  });
  for (const id of ["sound-lead", "interval-seconds", "cast-time"]) {
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
