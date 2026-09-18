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
  offsetSeconds: number | null;
  tank: string | null;
};

export type TankSnapshot = {
  name: string;
  from: number | null;
  to: number | null;
  intervalSeconds: number;
  running: boolean;
  startedAtMs: number | null;
  currentNumber: number | null;
  nextNumber: number | null;
  beatTick: number | null;
  isYou: boolean;
};

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
  startedAtMs: number | null;
  beatTick: number | null;
  warning: string | null;
  warningUrgent: boolean;
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
    effect: "Start iterating from now. Omit the tank to start every chain that has clerics; name one to start only that chain.",
  },
  {
    usage: "!endchain",
    aliases: ["!end-chain", "!end", "!end chain"],
    optional: "tank",
    effect: "Stop iterating. Cleric slots stay. Add a tank name to stop only that chain.",
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
    usage: "!take <number>",
    effect: "Move the speaker onto that number, leaving their old number empty. Does not start the clock.",
  },
  {
    usage: "!move <from> <to>",
    effect: "Swap two numbers that are already set.",
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
    usage: "!rstartchain",
    aliases: ["!rstart-chain", "!rstart"],
    optional: "tank",
    effect: "Start the rampage chain from now. Same rules as !startchain.",
  },
  {
    usage: "!rendchain",
    aliases: ["!rend-chain", "!rend"],
    optional: "tank",
    effect: "Stop the rampage chain. Slots stay.",
  },
  {
    usage: "!rmt <tank>",
    effect: "Set the rampage main tank.",
  },
  {
    usage: "!rot <tank>",
    effect: "Set the rampage off tank. Follow with !rsplit.",
  },
  {
    usage: "!rsplit <slot>",
    effect: "Two-tank cut on the rampage chain, for example !rsplit CCC.",
  },
  {
    usage: "!rtank <tank> <from> <to>",
    effect: "Put a letter range on another rampage tank, for example !rtank Beefwich AAA FFF.",
  },
  {
    usage: "!runtank <tank>",
    effect: "Remove that rampage tank. Its slots fall back to the main tank.",
  },
  {
    usage: "!skip",
    aliases: ["!rskip"],
    optional: "slot",
    effect: "Skip your rampage slot, or a letter like AAA. Same command as CH; numbers go to the CH chain.",
  },
  {
    usage: "!back",
    aliases: ["!rback"],
    optional: "slot",
    effect: "Put your rampage slot back in, or a letter if you include one.",
  },
  {
    usage: "!rtake <slot>",
    effect: "Move onto that rampage letter. !take AAA also works. Does not start the clock.",
  },
  {
    usage: "!rmove <from> <to>",
    effect: "Swap two rampage letters that are already set. !move AAA BBB also works.",
  },
  {
    usage: "!rchain <seconds>",
    optional: "tank",
    effect: "Set the rampage interval, for example !rchain 2.",
  },
  {
    usage: "!rreset-chain",
    aliases: ["!rresetchain", "!rreset"],
    effect: "Clear rampage slots and stop. Tank and interval stay.",
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
  if (Math.abs(seconds) < 0.005) return "±0.00s";
  const sign = seconds > 0 ? "+" : "";
  return `${sign}${seconds.toFixed(2)}s`;
}

export function shouldShowYouBanner(opts: {
  running: boolean;
  youCastIn: number | null | undefined;
  youAreNextIn: number | null | undefined;
}): boolean {
  if (opts.running) return opts.youCastIn != null;
  return opts.youAreNextIn != null && opts.youAreNextIn <= 8;
}

export type AlertMode = "sound" | "metronome" | "none";

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

function rotation(slots: SlotSnapshot[]): number[] {
  return slots
    .filter((slot) => !slot.skipped)
    .map((slot) => slot.number)
    .sort((a, b) => a - b);
}

export function rotateQueue(
  slots: SlotSnapshot[],
  running: boolean,
  currentNumber: number | null | undefined,
  nextNumber: number | null | undefined,
): SlotSnapshot[] {
  if (slots.length <= 1) return slots;
  const ordered = [...slots].sort((a, b) => a.number - b.number);
  const head = running ? currentNumber : (nextNumber ?? currentNumber);
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
  return slots.map((slot) => {
    if (slot.skipped) {
      return { ...slot, isCurrent: false, isNext: false, remainingSeconds: 0, progress: 0 };
    }
    const slotIdx = rot.indexOf(slot.number);
    if (slotIdx < 0) {
      return { ...slot, isCurrent: false, isNext: false, remainingSeconds: 0, progress: 0 };
    }
    const steps = (slotIdx - idx + rot.length) % rot.length;
    const nextBeat = startedAtMs + (ticks + steps) * intervalMs;
    const remaining = Math.max(0, (nextBeat - now) / 1000);
    const progress = cycleSeconds > 0 ? Math.min(1, remaining / cycleSeconds) : 0;
    return {
      ...slot,
      isCurrent: slot.number === currentNumber,
      isNext: slot.number === nextNumber,
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

function applyAllTanks(snapshot: ChainSnapshot, now: number): ChainSnapshot {
  const groups = new Map<string, SlotSnapshot[]>();
  for (const slot of snapshot.slots) {
    const key = slot.tank || "";
    const list = groups.get(key) ?? [];
    list.push(slot);
    groups.set(key, list);
  }
  const slots: SlotSnapshot[] = [];
  for (const [name, group] of groups) {
    const tank = snapshot.tanks.find((item) => item.name === name);
    let scheduled = group;
    if (tank?.running && tank.startedAtMs != null) {
      scheduled = scheduleSlots(group, tank.startedAtMs, tank.intervalSeconds, now);
    } else {
      scheduled = group.map((slot) => {
        if (!slot.lastShoutMs) {
          return { ...slot, remainingSeconds: 0, progress: 0 };
        }
        const remaining = Math.max(
          0,
          snapshot.castTimeSeconds - (now - slot.lastShoutMs) / 1000,
        );
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
  return {
    ...snapshot,
    slots,
    currentNumber: null,
    nextNumber: null,
    youCastIn: null,
    youAreNextIn: null,
    nowMs: now,
    running: snapshot.tanks.some((tank) => tank.running),
  };
}

export function liveSnapshot(
  snapshot: ChainSnapshot | null,
  now: number,
): ChainSnapshot | null {
  if (!snapshot) return null;
  const splitView = !snapshot.yourTank && (snapshot.tanks?.length ?? 0) > 1;
  if (splitView) {
    return applyAllTanks(snapshot, now);
  }
  if (snapshot.running && snapshot.startedAtMs != null) {
    return applySchedule(snapshot, now);
  }
  const cast = snapshot.castTimeSeconds;
  const slots = snapshot.slots.map((slot) => {
    if (!slot.lastShoutMs) {
      return { ...slot, remainingSeconds: 0, progress: 0 };
    }
    const remaining = Math.max(0, cast - (now - slot.lastShoutMs) / 1000);
    return {
      ...slot,
      remainingSeconds: remaining,
      progress: cast > 0 ? Math.min(1, remaining / cast) : 0,
    };
  });
  let youAreNextIn = snapshot.youAreNextIn;
  if (snapshot.nextNumber != null && snapshot.currentNumber != null) {
    const current = snapshot.slots.find((s) => s.number === snapshot.currentNumber);
    const next = snapshot.slots.find((s) => s.number === snapshot.nextNumber);
    if (current?.lastShoutMs && next?.isYou) {
      youAreNextIn = Math.max(
        0,
        snapshot.intervalSeconds - (now - current.lastShoutMs) / 1000,
      );
    } else {
      youAreNextIn = null;
    }
  }
  return {
    ...snapshot,
    slots: rotateQueue(slots, false, snapshot.currentNumber, snapshot.nextNumber),
    youAreNextIn,
    youCastIn: youAreNextIn,
    nowMs: now,
  };
}
