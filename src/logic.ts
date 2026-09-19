export type SlotSnapshot = {
  number: number;
  player: string;
  target: string | null;
  skipped: boolean;
  isYou: boolean;
  isCurrent: boolean;
  isNext: boolean;
  remainingSeconds: number;
  progress: number;
  lastShoutMs: number | null;
  lastCastMs: number | null;
  castRemainingSeconds: number;
  castProgress: number;
  offsetSeconds: number | null;
  tank: string | null;
};

export type TankSnapshot = {
  name: string;
  from: number | null;
  to: number | null;
  intervalSeconds: number;
  running: boolean;
  armed: boolean;
  startedAtMs: number | null;
  currentNumber: number | null;
  nextNumber: number | null;
  beatTick: number | null;
  isYou: boolean;
};

export type WarningKind =
  | "other"
  | "slotTaken"
  | "wrongTarget"
  | "autoTake"
  | "startChain"
  | "pace";

export type ChainSnapshot = {
  tank: string | null;
  yourTank: string | null;
  intervalSeconds: number;
  castTimeSeconds: number;
  yourName: string | null;
  currentNumber: number | null;
  nextNumber: number | null;
  youAreNextIn: number | null;
  youCastIn: number | null;
  youLastOffset: number | null;
  running: boolean;
  armed: boolean;
  startedAtMs: number | null;
  beatTick: number | null;
  warning: string | null;
  warningUrgent: boolean;
  warningKind: WarningKind;
  warningSpeech: string | null;
  warningAtMs: number | null;
  tanks: TankSnapshot[];
  slots: SlotSnapshot[];
  slotFormat: "number" | "letter";
  nowMs: number;
};

export type WatchStatus = {
  eqDirectory: string | null;
  logsPath: string | null;
  logsCanonical: string | null;
  activeLog: string | null;
  character: string | null;
  backend: string;
  lastError: string | null;
};

export type EqDirectoryProbe = {
  ok: boolean;
  path: string;
  logsPath: string | null;
  activeLog: string | null;
  character: string | null;
  error: string | null;
};

export function escapeHtml(value: string): string {
  return value.replace(/[&<>"']/g, (ch) => {
    switch (ch) {
      case "&":
        return "&amp;";
      case "<":
        return "&lt;";
      case ">":
        return "&gt;";
      case '"':
        return "&quot;";
      default:
        return "&#39;";
    }
  });
}

export function fileName(path: string | null | undefined): string {
  if (!path) return "—";
  const parts = path.split(/[/\\]/);
  return parts[parts.length - 1] || path;
}

export function padSlot(number: number): string {
  return String(number).padStart(3, "0");
}

export function padLetter(number: number): string {
  const idx = Math.max(1, Math.min(26, Math.round(number))) - 1;
  return String.fromCharCode(65 + idx).repeat(3);
}

export function formatSlot(
  number: number,
  format: "number" | "letter" | null | undefined,
): string {
  return format === "letter" ? padLetter(number) : padSlot(number);
}

export function spokenSlot(
  number: number,
  format: "number" | "letter" | null | undefined,
): string {
  return format === "letter" ? padLetter(number) : String(Math.round(number));
}

export type CommandHelp = {
  usage: string;
  aliases?: string[];
  optional?: string;
  effect: string;
};

export const CHAIN_COMMANDS: CommandHelp[] = [
  {
    usage: "!startchain",
    aliases: ["!start-chain", "!start", "!start chain"],
    optional: "tank",
    effect: "Arm CH and rampage and announce that the chain is starting. The clock starts on the first CH or RCH, from that cleric. Name a tank to arm only that tank.",
  },
  {
    usage: "!stopchain",
    aliases: ["!stop-chain", "!stop", "!stop chain"],
    optional: "tank",
    effect: "Stop CH and rampage. Cleric slots stay. Add a tank name to stop only that tank.",
  },
  {
    usage: "!mt <tank>",
    effect: "Set the main tank. Owns every number that is not in another tank's range.",
  },
  {
    usage: "!ot <tank>",
    effect: "Set the off tank for a two-tank split. Follow with !split.",
  },
  {
    usage: "!split <number>",
    effect: "Two-tank cut: numbers below this stay on the main tank, this number and above go to the off tank.",
  },
  {
    usage: "!tank <tank> <from> <to>",
    effect: "Put a number range on another tank. Use this when you need more than two tanks.",
  },
  {
    usage: "!untank <tank>",
    effect: "Remove that tank. Its numbers fall back to the main tank.",
  },
  {
    usage: "!skip",
    optional: "slot",
    effect: "Skip your own slot on CH and rampage. Add 001 to skip a CH number or AAA to skip a rampage letter.",
  },
  {
    usage: "!back",
    optional: "slot",
    effect: "Put your own slot back in on CH and rampage, or a specific 001 / AAA slot.",
  },
  {
    usage: "!take",
    optional: "slot",
    effect: "Omit the slot to take the next free CH number. Or pick one: !take 001, !take AAA, or name someone else: !take 001 Portlia. Does not start the clock.",
  },
  {
    usage: "!move <from> <to>",
    effect: "Swap two slots that are already set. Numbers are CH, letters are rampage, for example !move AAA BBB.",
  },
  {
    usage: "!chain <seconds>",
    optional: "tank",
    effect: "Set the chain interval, for example !chain 2. Add a tank name to set that tank only.",
  },
  {
    usage: "!reset-chain",
    aliases: ["!resetchain"],
    effect: "Clear all slots and stop the chain. Tank and interval stay.",
  },
];

export const RAMPAGE_COMMANDS: CommandHelp[] = [
  {
    usage: "!rt <tank>",
    effect: "Set the rampage tank. Rampage has one tank; there is no off tank or split.",
  },
  {
    usage: "!rchain <seconds>",
    optional: "tank",
    effect: "Set the rampage interval, for example !rchain 2.",
  },
];

export function commandListHtml(commands: CommandHelp[] = CHAIN_COMMANDS): string {
  return commands
    .map((cmd) => {
      const aliases = cmd.aliases?.length
        ? `<span class="aliases">${cmd.aliases
            .map((alias) => `<code>${escapeHtml(alias)}</code>`)
            .join(" ")}</span>`
        : "";
      const optional = cmd.optional
        ? `<span class="optional"><code>[${escapeHtml(cmd.optional)}]</code> optional</span>`
        : "";
      return `<article class="command">
        <div class="command-usage"><code>${escapeHtml(cmd.usage)}</code>${optional}${aliases}</div>
        <p>${escapeHtml(cmd.effect)}</p>
      </article>`;
    })
    .join("");
}

export function formatOffset(seconds: number | null | undefined): string {
  if (seconds == null || Number.isNaN(seconds)) return "";
  if (Math.abs(seconds) < 0.005) return "on time";
  if (seconds > 0) return `late ${seconds.toFixed(2)}s`;
  return `early ${Math.abs(seconds).toFixed(2)}s`;
}

export function offsetClassName(seconds: number | null | undefined): string {
  if (seconds == null || Number.isNaN(seconds)) return "";
  if (seconds > 0.005) return "late";
  if (seconds < -0.005) return "early";
  return "";
}

export function chainStateLabel(opts: {
  running: boolean;
  armed?: boolean;
}): string {
  if (opts.running) return "Running";
  if (opts.armed) return "Waiting";
  return "Stopped";
}

export function tankClockLabel(tank: { running: boolean; armed?: boolean }): string {
  if (tank.running) return "run";
  if (tank.armed) return "wait";
  return "stop";
}

export function shouldShowYouBanner(opts: {
  running: boolean;
  youCastIn: number | null | undefined;
  youAreNextIn: number | null | undefined;
}): boolean {
  if (opts.running) return opts.youCastIn != null;
  return opts.youAreNextIn != null && opts.youAreNextIn <= 8;
}

export function youCastProgress(snapshot: ChainSnapshot | null | undefined): number {
  const you = snapshot?.slots.find((slot) => slot.isYou && !slot.skipped);
  if (!you) return 0;
  return you.progress;
}

export type AlertMode = "sound" | "metronome" | "none";
export type SetupStep = "eq" | "audio";

export function firstRunSteps(opts: {
  setupComplete?: boolean;
  eqDirectory?: string | null;
}): SetupStep[] {
  if (opts.setupComplete) return [];
  const steps: SetupStep[] = [];
  if (!opts.eqDirectory?.trim()) steps.push("eq");
  steps.push("audio");
  return steps;
}

export function setupStepLabel(index: number, total: number): string {
  return `Step ${index + 1} of ${total}`;
}

export function alertMode(opts: {
  soundEnabled: boolean;
  metronomeEnabled: boolean;
}): AlertMode {
  if (opts.metronomeEnabled) return "metronome";
  if (opts.soundEnabled) return "sound";
  return "none";
}

export function shouldSpeakMetronome(opts: {
  enabled: boolean;
  running: boolean;
  currentNumber: number | null | undefined;
  beatTick: number | null | undefined;
  lastBeatTick: number | null;
  lastSpokenNumber?: number | null;
}): boolean {
  if (!opts.enabled || !opts.running) return false;
  if (opts.currentNumber == null || opts.beatTick == null) return false;
  if (opts.lastBeatTick !== opts.beatTick) return true;
  return (
    opts.lastSpokenNumber != null && opts.lastSpokenNumber !== opts.currentNumber
  );
}

export function shouldPlayClaimAlert(opts: {
  warning: string | null | undefined;
  urgent: boolean;
  lastWarning: string | null;
  armed: boolean;
}): boolean {
  if (!opts.armed || !opts.urgent || !opts.warning) return false;
  return opts.lastWarning !== opts.warning;
}

export function isAlertEnabled(opts: {
  kind: WarningKind | null | undefined;
  alertSlotTaken: boolean;
  alertWrongTarget: boolean;
}): boolean {
  if (opts.kind === "slotTaken") return opts.alertSlotTaken;
  if (opts.kind === "wrongTarget") return opts.alertWrongTarget;
  return true;
}

/// Urgent reads as danger, a pace change reads as news, everything else warns.
export function alertBannerClass(
  kind: WarningKind | null | undefined,
  urgent: boolean,
): string {
  if (urgent) return "banner danger dismissable";
  if (kind === "pace") return "banner pace dismissable";
  return "banner warn dismissable";
}

export function shouldSpeakAutoTake(opts: {
  enabled: boolean;
  kind: WarningKind | null | undefined;
  urgent: boolean;
  speech: string | null | undefined;
  lastSpoken: string | null;
}): boolean {
  if (!opts.enabled || opts.kind !== "autoTake" || !opts.urgent || !opts.speech) {
    return false;
  }
  return opts.lastSpoken !== opts.speech;
}

export function shouldSpeakWrongTarget(opts: {
  enabled: boolean;
  kind: WarningKind | null | undefined;
  urgent: boolean;
  warning: string | null | undefined;
  lastSpoken: string | null;
}): boolean {
  if (!opts.enabled || opts.kind !== "wrongTarget" || !opts.urgent || !opts.warning) {
    return false;
  }
  return opts.lastSpoken !== opts.warning;
}

export function shouldSpeakStartChain(opts: {
  enabled: boolean;
  kind: WarningKind | null | undefined;
  urgent: boolean;
  speech: string | null | undefined;
  warningAtMs: number | null | undefined;
  lastSpoken: string | null;
  lastSpokenAt: number | null;
  lastWarningAtMs: number | null;
  now: number;
}): boolean {
  if (!opts.enabled || opts.kind !== "startChain" || !opts.urgent || !opts.speech) {
    return false;
  }
  if (opts.warningAtMs != null && opts.lastWarningAtMs === opts.warningAtMs) {
    return false;
  }
  if (
    opts.lastSpoken === opts.speech &&
    opts.lastSpokenAt != null &&
    opts.now - opts.lastSpokenAt < 2000
  ) {
    return false;
  }
  return true;
}

export function shouldShowWarning(opts: {
  warning: string | null | undefined;
  dismissed: string | null;
  warningAtMs?: number | null;
  now?: number;
  dismissSeconds?: number;
}): boolean {
  if (!opts.warning) return false;
  if (opts.dismissed === opts.warning) return false;
  const ttl = opts.dismissSeconds;
  if (
    ttl != null &&
    ttl > 0 &&
    opts.warningAtMs != null &&
    opts.now != null &&
    opts.now - opts.warningAtMs >= ttl * 1000
  ) {
    return false;
  }
  return true;
}

export function shouldChime(opts: {
  eta: number;
  lead: number;
  lastChimeAt: number;
  now: number;
  soundEnabled: boolean;
  armed: boolean;
}): boolean {
  if (!opts.soundEnabled || !opts.armed) return false;
  if (opts.eta > opts.lead) return false;
  if (opts.now - opts.lastChimeAt < 8000) return false;
  return true;
}

export function nextUpSpeech(seconds: number): string {
  return seconds > 0 ? "GO SOON" : "GO NOW";
}

export function slotClassName(slot: SlotSnapshot): string {
  return [
    "slot",
    slot.isCurrent ? "is-current" : "",
    slot.isNext ? "is-next" : "",
    slot.isYou ? "is-you" : "",
    slot.skipped ? "is-skipped" : "",
  ]
    .filter(Boolean)
    .join(" ");
}

/// How many clerics of another tank's rotation the side panels show.
export const SIDE_CHAIN_CLERICS = 3;

export type SideChain = {
  tank: string;
  /// "CH" or "RCH", so a rampage rotation is never mistaken for a cleric one.
  kind: string;
  format: "number" | "letter";
  intervalSeconds: number;
  state: string;
  clerics: SlotSnapshot[];
  waiting: number;
};

/// Rampage chains use letters, cleric chains use numbers.
export function chainKindLabel(live: ChainSnapshot): string {
  return live.slotFormat === "letter" ? "RCH" : "CH";
}

/// Every rotation in one snapshot, trimmed to the next few clerics. Skipped
/// clerics are left out: they are not up next. Your own rotation is left out
/// too unless you ask for it, since it belongs in the main list.
export function sideChains(
  live: ChainSnapshot | null,
  limit: number = SIDE_CHAIN_CLERICS,
  opts: { includeYours?: boolean } = {},
): SideChain[] {
  if (!live) return [];
  const groups = tankGroups(live.slots);
  if (groups.size === 0) return [];
  if (groups.size <= 1 && !opts.includeYours) return [];
  const yours = opts.includeYours ? null : yourTankName(live);
  const names = live.tanks.map((tank) => tank.name).filter((name) => groups.has(name));
  for (const name of groups.keys()) {
    if (!names.includes(name)) names.push(name);
  }
  return names
    .filter((name) => name !== yours)
    .map((name) => {
      const group = groups.get(name) ?? [];
      const queue = group.filter((slot) => !slot.skipped);
      const tank = live.tanks.find((item) => item.name === name);
      return {
        tank: name,
        kind: chainKindLabel(live),
        format: live.slotFormat,
        intervalSeconds: tank?.intervalSeconds ?? live.intervalSeconds,
        state: chainStateLabel({ running: tank?.running ?? false, armed: tank?.armed }),
        clerics: queue.slice(0, limit),
        waiting: Math.max(0, queue.length - limit),
      };
    })
    .filter((side) => side.clerics.length > 0);
}

/// The chain you are on, which is the one that gets the big list. If you are
/// not on either, whichever has clerics on it wins, cleric chain first.
export function primaryChain(
  chain: ChainSnapshot | null,
  rampage: ChainSnapshot | null,
): { live: ChainSnapshot | null; kind: "chain" | "rampage" } {
  const onChain = yourSlotNumber(chain) != null;
  const onRampage = yourSlotNumber(rampage) != null;
  if (onRampage && !onChain) return { live: rampage, kind: "rampage" };
  if (onChain) return { live: chain, kind: "chain" };
  if ((chain?.slots.length ?? 0) > 0) return { live: chain, kind: "chain" };
  if ((rampage?.slots.length ?? 0) > 0) return { live: rampage, kind: "rampage" };
  return { live: chain, kind: "chain" };
}

/// Everything that is not the main list: the other tanks on your own chain,
/// then every rotation on the chain you are not running.
export function chainRail(
  primary: ChainSnapshot | null,
  secondary: ChainSnapshot | null,
  limit: number = SIDE_CHAIN_CLERICS,
): SideChain[] {
  return [
    ...sideChains(primary, limit),
    ...sideChains(secondary, limit, { includeYours: true }),
  ];
}

export function sideChainsHtml(sides: SideChain[]): string {
  return sides
    .map((side) => {
      const rows = side.clerics
        .map((slot) => {
          const eta = slot.remainingSeconds > 0 ? `${slot.remainingSeconds.toFixed(1)}s` : "—";
          const flag = slot.isNext ? " is-next" : slot.isCurrent ? " is-current" : "";
          const you = slot.isYou ? " is-you" : "";
          return `<li class="side-slot${flag}${you}">
        <span class="num">${formatSlot(slot.number, side.format)}</span>
        <span class="player">${escapeHtml(slot.player)}</span>
        <span class="eta">${eta}</span>
      </li>`;
        })
        .join("");
      const waiting = side.waiting > 0 ? `<small class="side-more">+${side.waiting} more</small>` : "";
      return `<section class="side-chain">
      <header>
        <strong>${escapeHtml(side.tank)}</strong>
        <small>${escapeHtml(side.kind)} · ${escapeHtml(side.state)} · ${side.intervalSeconds.toFixed(1)}s</small>
      </header>
      <ol>${rows}</ol>
      ${waiting}
    </section>`;
    })
    .join("");
}

export function clampOverlayOpacity(value: number): number {
  if (!Number.isFinite(value)) return 0.85;
  return Math.min(1, Math.max(0.25, value));
}

export function overlayPanelHtml(
  live: ChainSnapshot | null,
  title: string,
): string {
  const state = chainStateLabel({
    running: live?.running ?? false,
    armed: live?.armed,
  });
  const interval = live ? `${live.intervalSeconds.toFixed(1)}s` : "—";
  const eta = live?.running ? live.youCastIn : live?.youAreNextIn;
  const showYou = shouldShowYouBanner({
    running: live?.running ?? false,
    youCastIn: live?.youCastIn,
    youAreNextIn: live?.youAreNextIn,
  });
  const youLine = showYou
    ? `<div class="overlay-you"><span class="overlay-you-slot">${escapeHtml(
        yourSlotLabel(live),
      )}</span> Cast in ${(eta ?? 0).toFixed(1)}s</div>`
    : "";
  // The overlay is small, so it only ever shows your own rotation.
  const mine = live ? yourTankSlots(live) : [];
  const slots =
    !live || mine.length === 0
      ? `<p class="overlay-empty">Waiting</p>`
      : mine
          .map((slot) => {
            const width = Math.round(slot.progress * 1000) / 10;
            const next = slot.isNext ? " Next" : "";
            return `<article class="${slotClassName(slot)} overlay-slot">
  <div class="overlay-slot-head">
    <span class="num">${formatSlot(slot.number, live.slotFormat)}</span>
    <span class="player">${escapeHtml(slot.player)}</span>
    <span class="overlay-slot-flag">${next}</span>
  </div>
  <div class="bar"><span style="width:${width}%"></span></div>
</article>`;
          })
          .join("");
  return `<section class="overlay-panel">
  <header class="overlay-panel-head">
    <h2>${escapeHtml(title)}</h2>
    <span>${escapeHtml(state)} · ${escapeHtml(interval)}</span>
  </header>
  ${youLine}
  <div class="overlay-slots">${slots}</div>
</section>`;
}

export type SessionEventKind =
  | "heal"
  | "start"
  | "stop"
  | "reset"
  | "claim"
  | "skip"
  | "back"
  | "move"
  | "tank"
  | "interval"
  | "wrongTarget"
  | "warning";

export type SessionEvent = {
  atMs: number;
  kind: SessionEventKind;
  player: string | null;
  isYou: boolean;
  number: number | null;
  target: string | null;
  tank: string | null;
  offsetSeconds: number | null;
  text: string;
};

export type ClericReport = {
  rank: number;
  player: string;
  slots: number[];
  heals: number;
  onTime: number;
  early: number;
  late: number;
  missedTurns: number;
  wrongTarget: number;
  skips: number;
  isYou: boolean;
  avgOffsetSeconds: number | null;
  worstLateSeconds: number | null;
  score: number;
};

export type SessionReport = {
  id: number;
  kind: "ch" | "rampage";
  live: boolean;
  startedAtMs: number;
  endedAtMs: number | null;
  durationSeconds: number;
  tank: string | null;
  totalHeals: number;
  missedTurns: number;
  wrongTarget: number;
  warnings: number;
  avgOffsetSeconds: number | null;
  score: number;
  clerics: ClericReport[];
  events: SessionEvent[];
};

export function sessionSlotFormat(kind: SessionReport["kind"]): "number" | "letter" {
  return kind === "rampage" ? "letter" : "number";
}

export function sessionKindLabel(kind: SessionReport["kind"]): string {
  return kind === "rampage" ? "Rampage" : "CH";
}

export function formatDuration(seconds: number): string {
  if (!Number.isFinite(seconds) || seconds <= 0) return "0s";
  const total = Math.round(seconds);
  const hours = Math.floor(total / 3600);
  const minutes = Math.floor((total % 3600) / 60);
  const secs = total % 60;
  if (hours > 0) return `${hours}h ${String(minutes).padStart(2, "0")}m`;
  if (minutes > 0) return `${minutes}m ${String(secs).padStart(2, "0")}s`;
  return `${secs}s`;
}

export function formatClockTime(ms: number): string {
  if (!Number.isFinite(ms) || ms <= 0) return "—";
  return new Date(ms).toLocaleTimeString([], {
    hour: "numeric",
    minute: "2-digit",
  });
}

export function formatLogTime(ms: number): string {
  if (!Number.isFinite(ms) || ms <= 0) return "—";
  return new Date(ms).toLocaleTimeString([], {
    hour12: false,
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  });
}

export function sessionOptionLabel(report: SessionReport): string {
  const parts = [
    `Score ${Math.round(report.score)}`,
    sessionKindLabel(report.kind),
    formatClockTime(report.startedAtMs),
    report.live ? "running" : formatDuration(report.durationSeconds),
    `${report.clerics.length} cleric${report.clerics.length === 1 ? "" : "s"}`,
  ];
  if (report.tank) parts.push(report.tank);
  return parts.join(" · ");
}

export function onTimeShare(cleric: ClericReport): number | null {
  const rated = cleric.onTime + cleric.early + cleric.late;
  if (rated === 0) return null;
  return cleric.onTime / rated;
}

export function formatPercent(share: number | null): string {
  if (share == null) return "—";
  return `${Math.round(share * 100)}%`;
}

export function scoreClassName(score: number): string {
  if (score >= 90) return "score good";
  if (score >= 70) return "score ok";
  return "score bad";
}

export function sessionScoreHtml(report: SessionReport): string {
  const score = Math.round(report.score);
  return `<div class="session-score">
    <span class="${scoreClassName(report.score)} session-score-badge">${score}</span>
    <div class="session-score-copy">
      <strong>Chain score</strong>
      <small>${escapeHtml(sessionKindLabel(report.kind))} · ${escapeHtml(
        report.clerics.length === 1 ? "1 cleric" : `${report.clerics.length} clerics`,
      )} · ${escapeHtml(String(report.totalHeals))} casts</small>
    </div>
  </div>`;
}

export function sessionSummaryHtml(report: SessionReport): string {
  const facts: Array<[string, string]> = [
    ["Chain", sessionKindLabel(report.kind)],
    ["Tank", report.tank || "—"],
    ["Started", formatClockTime(report.startedAtMs)],
    ["Length", report.live ? "Running" : formatDuration(report.durationSeconds)],
    ["Casts", String(report.totalHeals)],
    ["Missed turns", String(report.missedTurns)],
    ["Wrong target", String(report.wrongTarget)],
    ["Avg offset", formatOffset(report.avgOffsetSeconds) || "on time"],
  ];
  return facts
    .map(
      ([label, value]) =>
        `<div><span class="k">${escapeHtml(label)}</span><strong>${escapeHtml(value)}</strong></div>`,
    )
    .join("");
}

export function clericTableHtml(report: SessionReport): string {
  if (report.clerics.length === 0) {
    return `<p class="empty">No casts were recorded in this session.</p>`;
  }
  const format = sessionSlotFormat(report.kind);
  const rows = report.clerics
    .map((cleric) => {
      const slots = cleric.slots.length
        ? cleric.slots.map((slot) => formatSlot(slot, format)).join(" ")
        : "—";
      const offset = formatOffset(cleric.avgOffsetSeconds);
      const you = cleric.isYou ? " is-you" : "";
      return `<tr class="cleric-row${you}">
        <td class="rank">${cleric.rank}</td>
        <td class="who"><strong>${escapeHtml(cleric.player)}</strong><small>${escapeHtml(slots)}</small></td>
        <td>${cleric.heals}</td>
        <td>${escapeHtml(formatPercent(onTimeShare(cleric)))}</td>
        <td class="${offsetClassName(cleric.avgOffsetSeconds)}">${escapeHtml(offset || "—")}</td>
        <td>${cleric.missedTurns}</td>
        <td>${cleric.wrongTarget}</td>
        <td><span class="${scoreClassName(cleric.score)}">${Math.round(cleric.score)}</span></td>
      </tr>`;
    })
    .join("");
  return `<table class="cleric-table">
    <thead>
      <tr>
        <th>#</th><th>Cleric</th><th>Casts</th><th>On time</th><th>Avg</th><th>Missed</th><th>Wrong</th><th>Score</th>
      </tr>
    </thead>
    <tbody>${rows}</tbody>
  </table>`;
}

export function sessionEventsHtml(report: SessionReport, limit = 150): string {
  const events = report.events.slice(-limit).reverse();
  if (events.length === 0) {
    return `<p class="empty">Nothing happened in this session yet.</p>`;
  }
  const format = sessionSlotFormat(report.kind);
  return events
    .map((event) => {
      const slot = event.number != null ? formatSlot(event.number, format) : "";
      const offset = event.kind === "heal" ? formatOffset(event.offsetSeconds) : "";
      const offsetHtml = offset
        ? `<span class="timeline-offset ${offsetClassName(event.offsetSeconds)}">${escapeHtml(offset)}</span>`
        : "";
      return `<li class="timeline-row is-${escapeHtml(event.kind)}">
        <span class="timeline-time">${escapeHtml(formatLogTime(event.atMs))}</span>
        <span class="timeline-slot">${escapeHtml(slot)}</span>
        <span class="timeline-text">${escapeHtml(event.text)}</span>
        ${offsetHtml}
      </li>`;
    })
    .join("");
}

export type DemoScenario = {
  id: string;
  name: string;
  description: string;
  steps: number;
  seconds: number;
  configurable: boolean;
  clerics: number;
  intervalSeconds: number;
};

export type DemoOptions = {
  clerics: number;
  maxClerics: number;
  minutes: number;
};

export const DEMO_CAST_SECONDS = 10;

/// Matches the Rust floor: a demo chain never beats faster than a second.
export const DEMO_MIN_INTERVAL_SECONDS = 1;

/// Mirrors the Rust clamps so the page cannot ask for a chain Alfred will not run.
export function clampDemoOptions(options: Partial<DemoOptions>): DemoOptions {
  const clamp = (value: number, min: number, max: number, fallback: number) =>
    Number.isFinite(value) ? Math.min(max, Math.max(min, value)) : fallback;
  const clerics = Math.round(clamp(options.clerics ?? 8, 1, 24, 8));
  return {
    clerics,
    maxClerics: Math.round(clamp(options.maxClerics ?? 20, clerics, 24, Math.max(clerics, 20))),
    minutes: Math.round(clamp(options.minutes ?? 4, 1, 30, 4)),
  };
}

export function fastestInterval(clerics: number): number {
  if (!Number.isFinite(clerics) || clerics <= 0) return DEMO_CAST_SECONDS;
  const split = Math.ceil((DEMO_CAST_SECONDS / clerics) * 10) / 10;
  return Math.max(DEMO_MIN_INTERVAL_SECONDS, split);
}

export function demoPaceHint(options: DemoOptions): string {
  const opening = fastestInterval(options.clerics);
  const full = fastestInterval(options.maxClerics);
  const turn = formatDuration(options.maxClerics * full);
  const joiners = options.maxClerics - options.clerics;
  const start = `${options.clerics} clerics split a ${DEMO_CAST_SECONDS}s CH ${opening}s apart`;
  const middle =
    joiners === 0
      ? "and nobody else shows up"
      : `, then ${joiners} more take numbers mid-pull until the chain is ${options.maxClerics} at ${full}s`;
  return `${start}${middle}. Your turn comes around every ${turn} on a full chain, and clerics skip out and come back while it runs.`;
}

export function demoLengthLabel(scenario: DemoScenario, speed: number): string {
  const real = formatDuration(scenario.seconds);
  if (speed === 1) return `${scenario.steps} lines, about ${real}.`;
  return `${scenario.steps} lines, about ${real} in real time — ${formatDuration(
    scenario.seconds / speed,
  )} at ${speed}×.`;
}

export type DemoLogEntry = {
  atMs: number;
  step: number;
  total: number;
  note: string;
  line: string;
  applied: boolean;
  level: "line" | "skip" | "warn" | "done";
};

export function demoScenarioLabel(scenario: DemoScenario): string {
  if (scenario.configurable) return `${scenario.name} · set up below`;
  return `${scenario.name} · ${scenario.steps} steps · ${formatDuration(scenario.seconds)}`;
}

export function demoProgressLabel(opts: {
  running: boolean;
  step: number;
  total: number;
}): string {
  if (opts.total === 0) return "Pick a scenario.";
  if (!opts.running) {
    return opts.step >= opts.total && opts.step > 0
      ? `Finished ${opts.total} steps.`
      : "Ready. Press Start and watch the Alfred window.";
  }
  return `Step ${opts.step} of ${opts.total}…`;
}

export function demoEntryHtml(entry: DemoLogEntry): string {
  const step = entry.level === "done" ? "" : `${entry.step}/${entry.total}`;
  const line = entry.line
    ? `<code class="demo-line">${escapeHtml(entry.line)}</code>`
    : "";
  const ignored =
    entry.level === "skip" ? `<span class="demo-flag">ignored by Alfred</span>` : "";
  return `<li class="demo-entry is-${escapeHtml(entry.level)}">
    <div class="demo-entry-head">
      <span class="demo-time">${escapeHtml(formatLogTime(entry.atMs))}</span>
      <span class="demo-step">${escapeHtml(step)}</span>
      <span class="demo-note">${escapeHtml(entry.note)}</span>
    </div>
    ${line}${ignored}
  </li>`;
}

export function updateAvailableMessage(opts: {
  version: string;
  currentVersion: string;
}): string {
  return `Alfred ${opts.version} is available. You have ${opts.currentVersion}.`;
}

export function updateUpToDateMessage(currentVersion: string): string {
  return `You're on the latest version (${currentVersion}).`;
}

export function updateProgressLabel(downloaded: number, contentLength: number): string {
  if (!Number.isFinite(downloaded) || downloaded < 0) downloaded = 0;
  if (!Number.isFinite(contentLength) || contentLength <= 0) {
    return "Downloading update…";
  }
  const pct = Math.min(100, Math.max(0, Math.round((downloaded / contentLength) * 100)));
  return `Downloading update… ${pct}%`;
}

export function updateNotesPreview(notes: string | null | undefined, max = 240): string {
  const text = (notes ?? "").replace(/\s+/g, " ").trim();
  if (!text) return "";
  if (text.length <= max) return text;
  return `${text.slice(0, max).trimEnd()}…`;
}

export function watchStatusLabel(watch: WatchStatus | null): { kind: "ok" | "warn"; text: string } {
  if (!watch || watch.backend === "idle") {
    return {
      kind: "warn",
      text: watch?.lastError || "No EverQuest directory set",
    };
  }
  if (watch.lastError) {
    return { kind: "warn", text: watch.lastError };
  }
  const source = watch.backend === "events" ? "OS events" : "polling";
  const who = watch.character ? ` · ${watch.character}` : "";
  return {
    kind: "ok",
    text: `Watching ${fileName(watch.activeLog)}${who} via ${source}`,
  };
}

export function eqDirStatusText(probe: EqDirectoryProbe): { kind: "ok" | "warn"; text: string } {
  if (!probe.ok) {
    return { kind: "warn", text: probe.error || "Invalid EverQuest directory" };
  }
  if (probe.character) {
    const log = probe.activeLog ? ` · ${fileName(probe.activeLog)}` : "";
    return { kind: "ok", text: `Character ${probe.character}${log}` };
  }
  return {
    kind: "ok",
    text: "EverQuest folder found. No eqlog yet — log in with /log on.",
  };
}

function nextCastOrigin(slot: SlotSnapshot, startedAtMs: number): number {
  return Math.max(slot.lastCastMs ?? 0, slot.lastShoutMs ?? 0, startedAtMs);
}

export function applyCastTiming(
  slot: SlotSnapshot,
  castTimeSeconds: number,
  now: number,
): SlotSnapshot {
  const start = slot.lastCastMs ?? slot.lastShoutMs;
  if (!start) {
    return { ...slot, lastCastMs: null, castRemainingSeconds: 0, castProgress: 0 };
  }
  const remaining = Math.max(0, castTimeSeconds - (now - start) / 1000);
  const progress = castTimeSeconds > 0 ? Math.min(1, remaining / castTimeSeconds) : 0;
  return {
    ...slot,
    lastCastMs: start,
    castRemainingSeconds: remaining,
    castProgress: progress,
  };
}

function rotation(slots: SlotSnapshot[]): number[] {
  return slots
    .filter((slot) => !slot.skipped)
    .map((slot) => slot.number)
    .sort((a, b) => a - b);
}

export function rotateQueue(
  slots: SlotSnapshot[],
  _running: boolean,
  currentNumber: number | null | undefined,
  nextNumber: number | null | undefined,
): SlotSnapshot[] {
  if (slots.length <= 1) return slots;
  const ordered = [...slots].sort((a, b) => a.number - b.number);
  const head = nextNumber ?? currentNumber;
  if (head == null) return ordered;
  const idx = ordered.findIndex((slot) => slot.number === head);
  if (idx <= 0) return ordered;
  return [...ordered.slice(idx), ...ordered.slice(0, idx)];
}

function scheduleSlots(
  slots: SlotSnapshot[],
  startedAtMs: number,
  intervalSeconds: number,
  now: number,
  castTimeSeconds: number,
): SlotSnapshot[] {
  const rot = rotation(slots);
  if (rot.length === 0 || intervalSeconds <= 0) return slots;
  const intervalMs = Math.max(1, Math.round(intervalSeconds * 1000));
  const elapsed = Math.max(0, now - startedAtMs);
  const ticks = Math.floor(elapsed / intervalMs);
  const idx = ticks % rot.length;
  const currentNumber = rot[idx];
  const nextNumber = rot[(idx + 1) % rot.length];
  const cycleSeconds = intervalSeconds * rot.length;
  const solo = rot.length <= 1;
  return slots.map((slot) => {
    if (slot.skipped) {
      return { ...slot, isCurrent: false, isNext: false, remainingSeconds: 0, progress: 0 };
    }
    const slotIdx = rot.indexOf(slot.number);
    if (slotIdx < 0) {
      return { ...slot, isCurrent: false, isNext: false, remainingSeconds: 0, progress: 0 };
    }
    if (solo) {
      const origin = nextCastOrigin(slot, startedAtMs);
      const remaining = Math.max(0, castTimeSeconds - (now - origin) / 1000);
      const progress = castTimeSeconds > 0 ? Math.min(1, remaining / castTimeSeconds) : 0;
      return {
        ...slot,
        isCurrent: slot.number === currentNumber,
        isNext: false,
        remainingSeconds: remaining,
        progress,
      };
    }
    let steps = (slotIdx - idx + rot.length) % rot.length;
    if (steps === 0) steps = rot.length;
    const nextBeat = startedAtMs + (ticks + steps) * intervalMs;
    const remaining = Math.max(0, (nextBeat - now) / 1000);
    const progress = cycleSeconds > 0 ? Math.min(1, remaining / cycleSeconds) : 0;
    return {
      ...slot,
      isCurrent: slot.number === currentNumber,
      isNext: slot.number === nextNumber && slot.number !== currentNumber,
      remainingSeconds: remaining,
      progress,
    };
  });
}

function applySchedule(snapshot: ChainSnapshot, now: number): ChainSnapshot {
  const rot = rotation(snapshot.slots);
  if (!snapshot.startedAtMs || rot.length === 0 || snapshot.intervalSeconds <= 0) {
    return { ...snapshot, nowMs: now };
  }
  const slots = scheduleSlots(
    snapshot.slots,
    snapshot.startedAtMs,
    snapshot.intervalSeconds,
    now,
    snapshot.castTimeSeconds,
  );
  const you = slots.find((slot) => slot.isYou && !slot.skipped);
  const youCastIn = you ? you.remainingSeconds : null;
  const currentNumber = slots.find((slot) => slot.isCurrent)?.number ?? null;
  const nextNumber = slots.find((slot) => slot.isNext)?.number ?? null;
  const youAreNextIn = you?.isNext ? youCastIn : null;
  const intervalMs = Math.max(1, Math.round(snapshot.intervalSeconds * 1000));
  const ticks = Math.floor(Math.max(0, now - snapshot.startedAtMs) / intervalMs);
  return {
    ...snapshot,
    slots: rotateQueue(slots, true, currentNumber, nextNumber),
    currentNumber,
    nextNumber,
    youCastIn,
    youAreNextIn,
    beatTick: ticks,
    nowMs: now,
    running: true,
  };
}

/// The tank whose rotation is yours, by name. Falls back to the tank on your
/// own slot, then to the chain's tank.
export function yourTankName(snapshot: ChainSnapshot): string {
  if (snapshot.yourTank) return snapshot.yourTank;
  const you = snapshot.slots.find((slot) => slot.isYou);
  return you?.tank || snapshot.tank || "";
}

export function tankGroups(slots: SlotSnapshot[]): Map<string, SlotSnapshot[]> {
  const groups = new Map<string, SlotSnapshot[]>();
  for (const slot of slots) {
    const key = slot.tank || "";
    const list = groups.get(key) ?? [];
    list.push(slot);
    groups.set(key, list);
  }
  return groups;
}

/// Your own number on the chain, or null when you are not on it.
export function yourSlotNumber(live: ChainSnapshot | null): number | null {
  const you = live?.slots.find((slot) => slot.isYou);
  return you?.number ?? null;
}

/// Your number for the header, with a note when you are sitting out.
export function yourSlotLabel(live: ChainSnapshot | null): string {
  const you = live?.slots.find((slot) => slot.isYou);
  if (!you) return "—";
  const slot = formatSlot(you.number, live?.slotFormat ?? "number");
  return you.skipped ? `${slot} (out)` : slot;
}

/// The slots on your own rotation, which is what the big list shows.
export function yourTankSlots(live: ChainSnapshot): SlotSnapshot[] {
  const groups = tankGroups(live.slots);
  if (groups.size <= 1) return live.slots;
  return groups.get(yourTankName(live)) ?? live.slots;
}

function applyAllTanks(snapshot: ChainSnapshot, now: number): ChainSnapshot {
  const groups = tankGroups(snapshot.slots);
  const slots: SlotSnapshot[] = [];
  for (const [name, group] of groups) {
    const tank = snapshot.tanks.find((item) => item.name === name);
    let scheduled = group;
    if (tank?.running && tank.startedAtMs != null) {
      scheduled = scheduleSlots(
        group,
        tank.startedAtMs,
        tank.intervalSeconds,
        now,
        snapshot.castTimeSeconds,
      );
    } else {
      scheduled = group.map((slot) => {
        const start = slot.lastCastMs ?? slot.lastShoutMs;
        if (!start) {
          return { ...slot, remainingSeconds: 0, progress: 0 };
        }
        const remaining = Math.max(0, snapshot.castTimeSeconds - (now - start) / 1000);
        return {
          ...slot,
          remainingSeconds: remaining,
          progress:
            snapshot.castTimeSeconds > 0
              ? Math.min(1, remaining / snapshot.castTimeSeconds)
              : 0,
        };
      });
    }
    const current = scheduled.find((slot) => slot.isCurrent)?.number ?? tank?.currentNumber;
    const next = scheduled.find((slot) => slot.isNext)?.number ?? tank?.nextNumber;
    slots.push(
      ...rotateQueue(scheduled, tank?.running ?? false, current, next),
    );
  }
  // Your own rotation still drives the banner and the meta line, even though
  // every tank was scheduled on its own clock.
  const yours = yourTankName(snapshot);
  const mine = yours ? slots.filter((slot) => (slot.tank || "") === yours) : [];
  const you = mine.find((slot) => slot.isYou && !slot.skipped);
  const youCastIn = you ? you.remainingSeconds : null;
  const yourClock = snapshot.tanks.find((tank) => tank.name === yours);
  return {
    ...snapshot,
    slots,
    currentNumber: mine.find((slot) => slot.isCurrent)?.number ?? null,
    nextNumber: mine.find((slot) => slot.isNext)?.number ?? null,
    youCastIn,
    youAreNextIn: you?.isNext ? youCastIn : null,
    nowMs: now,
    running: snapshot.yourTank
      ? (yourClock?.running ?? snapshot.running)
      : snapshot.tanks.some((tank) => tank.running),
  };
}

export function liveSnapshot(
  snapshot: ChainSnapshot | null,
  now: number,
): ChainSnapshot | null {
  if (!snapshot) return null;
  // More than one tank in the slots means more than one clock to honour, so
  // each rotation is scheduled on its own.
  const splitView =
    tankGroups(snapshot.slots).size > 1 ||
    (!snapshot.yourTank && (snapshot.tanks?.length ?? 0) > 1);
  const live = splitView
    ? applyAllTanks(snapshot, now)
    : snapshot.running && snapshot.startedAtMs != null
      ? applySchedule(snapshot, now)
      : idleSnapshot(snapshot, now);
  return {
    ...live,
    slots: live.slots.map((slot) => applyCastTiming(slot, live.castTimeSeconds, now)),
  };
}

function idleSnapshot(snapshot: ChainSnapshot, now: number): ChainSnapshot {
  const cast = snapshot.castTimeSeconds;
  let slots = snapshot.slots.map((slot) => {
    const start = slot.lastCastMs ?? slot.lastShoutMs;
    if (!start) {
      return { ...slot, remainingSeconds: 0, progress: 0 };
    }
    const remaining = Math.max(0, cast - (now - start) / 1000);
    return {
      ...slot,
      remainingSeconds: remaining,
      progress: cast > 0 ? Math.min(1, remaining / cast) : 0,
    };
  });
  return {
    ...snapshot,
    slots: rotateQueue(slots, false, snapshot.currentNumber, snapshot.nextNumber),
    youAreNextIn: null,
    youCastIn: null,
    nowMs: now,
  };
}
