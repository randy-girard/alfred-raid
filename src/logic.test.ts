import { describe, expect, it } from "vitest";
import {
  escapeHtml,
  fileName,
  formatOffset,
  CHAIN_COMMANDS,
  RAMPAGE_COMMANDS,
  commandListHtml,
  liveSnapshot,
  padSlot,
  padLetter,
  formatSlot,
  rotateQueue,
  alertMode,
  shouldChime,
  shouldPlayClaimAlert,
  shouldShowYouBanner,
  shouldSpeakMetronome,
  slotClassName,
  eqDirStatusText,
  watchStatusLabel,
  type ChainSnapshot,
  type SlotSnapshot,
  type WatchStatus,
} from "./logic";

function slot(overrides: Partial<SlotSnapshot> = {}): SlotSnapshot {
  return {
    number: 1,
    player: "Clericone",
    target: "Mluian",
    skipped: false,
    isYou: true,
    isCurrent: false,
    isNext: false,
    remainingSeconds: 0,
    progress: 0,
    lastShoutMs: null,
    offsetSeconds: null,
    tank: "Mluian",
    ...overrides,
  };
}

function snap(overrides: Partial<ChainSnapshot> = {}): ChainSnapshot {
  return {
    tank: "Mluian",
    yourTank: "Mluian",
    intervalSeconds: 2,
    castTimeSeconds: 10,
    yourName: "Clericone",
    currentNumber: 2,
    nextNumber: 1,
    youAreNextIn: 2,
    youCastIn: 2,
    youLastOffset: null,
    running: false,
    startedAtMs: null,
    beatTick: null,
    warning: null,
    warningUrgent: false,
    tanks: [],
    slotFormat: "number",
    slots: [
      slot({ number: 1, isYou: true, isNext: true }),
      slot({
        number: 2,
        player: "Two",
        isYou: false,
        isCurrent: true,
        lastShoutMs: 1_000,
        remainingSeconds: 10,
        progress: 1,
      }),
    ],
    nowMs: 1_000,
    ...overrides,
  };
}

describe("escapeHtml", () => {
  it("escapes markup characters", () => {
    expect(escapeHtml(`<You & "Two" 'Three'>`)).toBe(
      "&lt;You &amp; &quot;Two&quot; &#39;Three&#39;&gt;",
    );
  });
});

describe("fileName", () => {
  it("returns a dash for empty paths", () => {
    expect(fileName(null)).toBe("—");
    expect(fileName(undefined)).toBe("—");
    expect(fileName("")).toBe("—");
  });

  it("uses the last unix or windows segment", () => {
    expect(fileName("/eq/Logs/eqlog_A_P1999Green.txt")).toBe("eqlog_A_P1999Green.txt");
    expect(fileName(String.raw`C:\EQ\Logs\eqlog_A_P1999Green.txt`)).toBe(
      "eqlog_A_P1999Green.txt",
    );
  });
});

describe("helpers", () => {
  it("pads chain numbers", () => {
    expect(padSlot(1)).toBe("001");
    expect(padSlot(14)).toBe("014");
    expect(padLetter(1)).toBe("AAA");
    expect(padLetter(3)).toBe("CCC");
    expect(formatSlot(2, "letter")).toBe("BBB");
  });

  it("rotates a queue from the head number", () => {
    const slots = [
      slot({ number: 1 }),
      slot({ number: 2, player: "Two", isYou: false }),
      slot({ number: 3, player: "Three", isYou: false }),
    ];
    expect(rotateQueue(slots, true, 2, 3).map((s) => s.number)).toEqual([2, 3, 1]);
    expect(rotateQueue(slots, false, 1, 2).map((s) => s.number)).toEqual([2, 3, 1]);
  });

  it("lists every chain command and its effect", () => {
    expect(CHAIN_COMMANDS.map((cmd) => cmd.usage)).toEqual([
      "!startchain",
      "!endchain",
      "!mt <tank>",
      "!ot <tank>",
      "!split <number>",
      "!tank <tank> <from> <to>",
      "!untank <tank>",
      "!skip",
      "!back",
      "!take <number>",
      "!move <from> <to>",
      "!chain <seconds>",
      "!reset-chain",
    ]);
    expect(CHAIN_COMMANDS.every((cmd) => cmd.effect.length > 12)).toBe(true);
    expect(RAMPAGE_COMMANDS.map((cmd) => cmd.usage)).toEqual([
      "!rstartchain",
      "!rendchain",
      "!rmt <tank>",
      "!rot <tank>",
      "!rsplit <slot>",
      "!rtank <tank> <from> <to>",
      "!runtank <tank>",
      "!skip",
      "!back",
      "!rtake <slot>",
      "!rmove <from> <to>",
      "!rchain <seconds>",
      "!rreset-chain",
    ]);
    expect(RAMPAGE_COMMANDS.every((cmd) => cmd.effect.length > 12)).toBe(true);
    const html = commandListHtml();
    expect(html).toContain("!startchain");
    expect(html).toContain("!start-chain");
    expect(html).toContain("!split");
    expect(html).toContain("!ot");
    expect(commandListHtml(RAMPAGE_COMMANDS)).toContain("!rtake");
    expect(commandListHtml(RAMPAGE_COMMANDS)).toContain("!rstartchain");
    expect(html).toContain("!mt &lt;tank&gt;");
    expect(html).toContain("Skip your own slot");
    expect(html).toContain("[slot]");
    expect(html).toContain("optional");
    expect(commandListHtml(RAMPAGE_COMMANDS)).toContain("!rskip");
  });

  it("picks one audio cue when both flags are set", () => {
    expect(alertMode({ soundEnabled: true, metronomeEnabled: false })).toBe("sound");
    expect(alertMode({ soundEnabled: false, metronomeEnabled: true })).toBe("metronome");
    expect(alertMode({ soundEnabled: true, metronomeEnabled: true })).toBe("metronome");
    expect(alertMode({ soundEnabled: false, metronomeEnabled: false })).toBe("none");
  });

  it("formats timing offsets", () => {
    expect(formatOffset(null)).toBe("");
    expect(formatOffset(0)).toBe("±0.00s");
    expect(formatOffset(0.32)).toBe("+0.32s");
    expect(formatOffset(-0.15)).toBe("-0.15s");
  });

  it("shows the you banner while running or in the last 8 seconds idle", () => {
    expect(shouldShowYouBanner({ running: false, youCastIn: null, youAreNextIn: null })).toBe(
      false,
    );
    expect(
      shouldShowYouBanner({ running: false, youCastIn: 1, youAreNextIn: 8 }),
    ).toBe(true);
    expect(
      shouldShowYouBanner({ running: false, youCastIn: 9, youAreNextIn: 8.1 }),
    ).toBe(false);
    expect(shouldShowYouBanner({ running: true, youCastIn: 12, youAreNextIn: null })).toBe(
      true,
    );
  });

  it("speaks metronome numbers on each new beat", () => {
    expect(
      shouldSpeakMetronome({
        enabled: true,
        running: true,
        currentNumber: 2,
        beatTick: 1,
        lastBeatTick: 0,
      }),
    ).toBe(true);
    expect(
      shouldSpeakMetronome({
        enabled: true,
        running: true,
        currentNumber: 2,
        beatTick: 1,
        lastBeatTick: 1,
      }),
    ).toBe(false);
    expect(
      shouldSpeakMetronome({
        enabled: false,
        running: true,
        currentNumber: 2,
        beatTick: 1,
        lastBeatTick: 0,
      }),
    ).toBe(false);
    expect(
      shouldSpeakMetronome({
        enabled: true,
        running: true,
        currentNumber: 3,
        beatTick: 1,
        lastBeatTick: 1,
        lastSpokenNumber: 2,
      }),
    ).toBe(true);
  });

  it("only chimes when armed, enabled, within lead, and not recently chimed", () => {
    const base = {
      eta: 1.5,
      lead: 2,
      lastChimeAt: 0,
      now: 10_000,
      soundEnabled: true,
      armed: true,
    };
    expect(shouldChime(base)).toBe(true);
    expect(shouldChime({ ...base, soundEnabled: false })).toBe(false);
    expect(shouldChime({ ...base, armed: false })).toBe(false);
    expect(shouldChime({ ...base, eta: 3 })).toBe(false);
    expect(shouldChime({ ...base, lastChimeAt: 9_000 })).toBe(false);
  });

  it("plays a claim alert once when your number is taken", () => {
    const base = {
      warning: "Two took your slot 001.",
      urgent: true,
      lastWarning: null as string | null,
      armed: true,
    };
    expect(shouldPlayClaimAlert(base)).toBe(true);
    expect(shouldPlayClaimAlert({ ...base, lastWarning: base.warning })).toBe(false);
    expect(shouldPlayClaimAlert({ ...base, urgent: false })).toBe(false);
    expect(shouldPlayClaimAlert({ ...base, armed: false })).toBe(false);
  });

  it("builds slot class names", () => {
    expect(
      slotClassName(
        slot({ isCurrent: true, isNext: true, isYou: true, skipped: true }),
      ),
    ).toBe("slot is-current is-next is-you is-skipped");
  });
});

describe("watchStatusLabel", () => {
  it("warns when idle or missing a directory", () => {
    expect(watchStatusLabel(null)).toEqual({
      kind: "warn",
      text: "No EverQuest directory set",
    });
    expect(
      watchStatusLabel({
        eqDirectory: null,
        logsPath: null,
        logsCanonical: null,
        activeLog: null,
        character: null,
        backend: "idle",
        lastError: "Set your EverQuest directory in Settings.",
      }),
    ).toEqual({
      kind: "warn",
      text: "Set your EverQuest directory in Settings.",
    });
  });

  it("shows the active log and backend", () => {
    const watch: WatchStatus = {
      eqDirectory: "/eq",
      logsPath: "/eq/Logs",
      logsCanonical: "/eq/Logs",
      activeLog: "/eq/Logs/eqlog_Clericone_P1999Green.txt",
      character: "Clericone",
      backend: "events",
      lastError: null,
    };
    expect(watchStatusLabel(watch)).toEqual({
      kind: "ok",
      text: "Watching eqlog_Clericone_P1999Green.txt · Clericone via OS events",
    });
    expect(watchStatusLabel({ ...watch, lastError: "watch failed" })).toEqual({
      kind: "warn",
      text: "watch failed",
    });
  });

  it("summarizes a pasted EQ directory probe", () => {
    expect(
      eqDirStatusText({
        ok: false,
        path: "",
        logsPath: null,
        activeLog: null,
        character: null,
        error: "That path does not exist.",
      }),
    ).toEqual({ kind: "warn", text: "That path does not exist." });
    expect(
      eqDirStatusText({
        ok: true,
        path: "/eq",
        logsPath: "/eq/Logs",
        activeLog: "/eq/Logs/eqlog_Clericone_P1999Green.txt",
        character: "Clericone",
        error: null,
      }),
    ).toEqual({
      kind: "ok",
      text: "Character Clericone · eqlog_Clericone_P1999Green.txt",
    });
  });
});

describe("liveSnapshot", () => {
  it("returns null when there is no snapshot", () => {
    expect(liveSnapshot(null, 1000)).toBeNull();
  });

  it("recomputes remaining cast time and you-are-next countdown", () => {
    const live = liveSnapshot(snap(), 2_500)!;
    const current = live.slots.find((s) => s.number === 2)!;
    expect(current.remainingSeconds).toBe(8.5);
    expect(current.progress).toBe(0.85);
    expect(live.youAreNextIn).toBe(0.5);
    expect(live.nowMs).toBe(2_500);
  });

  it("keeps letter slot formatting for rampage snapshots", () => {
    const live = liveSnapshot(snap({ slotFormat: "letter" }), 2_500)!;
    expect(live.slotFormat).toBe("letter");
    expect(formatSlot(live.slots[0].number, live.slotFormat)).toBe("AAA");
  });

  it("clamps remaining time and clears you-are-next when it is not you", () => {
    const live = liveSnapshot(
      snap({
        nextNumber: 3,
        slots: [
          slot({ number: 2, player: "Two", isYou: false, isCurrent: true, lastShoutMs: 1_000 }),
          slot({ number: 3, player: "Three", isYou: false, isNext: true }),
        ],
      }),
      20_000,
    )!;
    expect(live.slots[0].remainingSeconds).toBe(0);
    expect(live.slots[0].progress).toBe(0);
    expect(live.youAreNextIn).toBeNull();
  });

  it("iterates a running chain and skips numbers on the fly", () => {
    const live = liveSnapshot(
      snap({
        running: true,
        startedAtMs: 10_000,
        intervalSeconds: 2,
        slots: [
          slot({ number: 1, isYou: true }),
          slot({ number: 2, player: "Two", isYou: false, skipped: true }),
          slot({ number: 3, player: "Three", isYou: false }),
        ],
      }),
      12_100,
    )!;
    expect(live.currentNumber).toBe(3);
    expect(live.nextNumber).toBe(1);
    expect(live.beatTick).toBe(1);
    expect(live.slots.find((s) => s.number === 2)?.isCurrent).toBe(false);
    expect(live.slots.map((s) => s.number)).toEqual([3, 1, 2]);
    expect(live.youCastIn).toBeGreaterThan(1);
  });

  it("rotates so the cleric who is up is first and the last to go is last", () => {
    const live = liveSnapshot(
      snap({
        running: true,
        startedAtMs: 10_000,
        intervalSeconds: 2,
        slots: [
          slot({ number: 1, isYou: true }),
          slot({ number: 2, player: "Two", isYou: false }),
          slot({ number: 3, player: "Three", isYou: false }),
        ],
      }),
      12_100,
    )!;
    expect(live.currentNumber).toBe(2);
    expect(live.slots.map((s) => s.number)).toEqual([2, 3, 1]);
    expect(live.slots[0]?.isCurrent).toBe(true);
  });

  it("puts the next in line first when the chain is stopped", () => {
    const live = liveSnapshot(
      snap({
        running: false,
        currentNumber: 1,
        nextNumber: 2,
        slots: [
          slot({ number: 1, isYou: true, isCurrent: true, isNext: false }),
          slot({
            number: 2,
            player: "Two",
            isYou: false,
            isCurrent: false,
            isNext: true,
            lastShoutMs: null,
          }),
          slot({ number: 3, player: "Three", isYou: false, lastShoutMs: null }),
        ],
      }),
      2_500,
    )!;
    expect(live.slots.map((s) => s.number)).toEqual([2, 3, 1]);
  });
});
