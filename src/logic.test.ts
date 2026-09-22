import { describe, expect, it } from "vitest";
import {
  escapeHtml,
  fileName,
  formatOffset,
  offsetClassName,
  CHAIN_COMMANDS,
  RAMPAGE_COMMANDS,
  commandListHtml,
  applyCastTiming,
  liveSnapshot,
  padSlot,
  padLetter,
  formatSlot,
  spokenSlot,
  rotateQueue,
  alertMode,
  firstRunSteps,
  setupStepLabel,
  shouldChime,
  nextUpSpeech,
  shouldPlayClaimAlert,
  shouldShowWarning,
  isAlertEnabled,
  alertBannerClass,
  shouldSpeakWrongTarget,
  shouldSpeakAutoTake,
  shouldSpeakStartChain,
  shouldShowYouBanner,
  chainStateLabel,
  youCastProgress,
  shouldSpeakMetronome,
  slotClassName,
  clampOverlayOpacity,
  overlayPanelHtml,
  updateAvailableMessage,
  updateUpToDateMessage,
  updateProgressLabel,
  updateNotesPreview,
  eqDirStatusText,
  watchStatusLabel,
  formatDuration,
  formatPercent,
  onTimeShare,
  scoreClassName,
  sessionKindLabel,
  sessionOptionLabel,
  sessionSlotFormat,
  sessionScoreHtml,
  sessionSummaryHtml,
  clericTableHtml,
  sessionEventsHtml,
  demoEntryHtml,
  demoLengthLabel,
  demoPaceHint,
  demoProgressLabel,
  demoScenarioLabel,
  clampDemoOptions,
  fastestInterval,
  clockIsIdle,
  chainRail,
  primaryChain,
  sideChains,
  sideChainsHtml,
  yourSlotLabel,
  yourSlotNumber,
  yourTankName,
  yourTankSlots,
  type ChainSnapshot,
  type ClericReport,
  type SessionReport,
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
    lastCastMs: null,
    castRemainingSeconds: 0,
    castProgress: 0,
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
    armed: false,
    startedAtMs: null,
    beatTick: null,
    warning: null,
    warningUrgent: false,
    warningKind: "other",
    warningSpeech: null,
    warningAtMs: null,
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
    expect(spokenSlot(1, "number")).toBe("1");
    expect(spokenSlot(14, "number")).toBe("14");
    expect(spokenSlot(1, null)).toBe("1");
    expect(spokenSlot(2, "letter")).toBe("BBB");
  });

  it("rotates a queue from the next number", () => {
    const slots = [
      slot({ number: 1 }),
      slot({ number: 2, player: "Two", isYou: false }),
      slot({ number: 3, player: "Three", isYou: false }),
    ];
    expect(rotateQueue(slots, true, 2, 3).map((s) => s.number)).toEqual([3, 1, 2]);
    expect(rotateQueue(slots, false, 1, 2).map((s) => s.number)).toEqual([2, 3, 1]);
  });

  it("lists every chain command and its effect", () => {
    expect(CHAIN_COMMANDS.map((cmd) => cmd.usage)).toEqual([
      "!startchain",
      "!stopchain",
      "!mt <tank>",
      "!ot <tank>",
      "!split <number>",
      "!tank <tank> <from> <to>",
      "!untank <tank>",
      "!skip",
      "!back",
      "!take",
      "!move <from> <to>",
      "!chain <seconds>",
      "!reset-chain",
    ]);
    expect(CHAIN_COMMANDS.every((cmd) => cmd.effect.length > 12)).toBe(true);
    expect(RAMPAGE_COMMANDS.map((cmd) => cmd.usage)).toEqual([
      "!rt <tank>",
      "!rchain <seconds>",
    ]);
    expect(commandListHtml(RAMPAGE_COMMANDS)).toContain("!rt &lt;tank&gt;");
    expect(commandListHtml(RAMPAGE_COMMANDS)).not.toContain("!rot");
    expect(commandListHtml(RAMPAGE_COMMANDS)).not.toContain("!rsplit");
    expect(commandListHtml(RAMPAGE_COMMANDS)).not.toContain("!rtake");
    expect(commandListHtml(RAMPAGE_COMMANDS)).not.toContain("!rskip");
    expect(commandListHtml(RAMPAGE_COMMANDS)).not.toContain("!rstartchain");
    expect(commandListHtml(RAMPAGE_COMMANDS)).not.toContain("!rstopchain");
    expect(commandListHtml(RAMPAGE_COMMANDS)).not.toContain("!rendchain");
    expect(commandListHtml(RAMPAGE_COMMANDS)).not.toContain("!rreset");
    expect(RAMPAGE_COMMANDS.every((cmd) => cmd.effect.length > 12)).toBe(true);
    const html = commandListHtml();
    expect(html).toContain("!startchain");
    expect(html).toContain("!start-chain");
    expect(html).toContain("!stopchain");
    expect(html).toContain("!stop-chain");
    expect(html).toContain("!split");
    expect(html).toContain("!ot");
    expect(commandListHtml(RAMPAGE_COMMANDS)).toContain("!rchain");
    expect(html).toContain("!mt &lt;tank&gt;");
    expect(html).toContain("Skip your own slot");
    expect(html).toContain("!take 001 Portlia");
    expect(html).toContain("!take AAA");
    expect(html).toContain("[slot]");
    expect(html).toContain("optional");
  });

  it("picks one audio cue when both flags are set", () => {
    expect(alertMode({ soundEnabled: true, metronomeEnabled: false })).toBe("sound");
    expect(alertMode({ soundEnabled: false, metronomeEnabled: true })).toBe("metronome");
    expect(alertMode({ soundEnabled: true, metronomeEnabled: true })).toBe("metronome");
    expect(alertMode({ soundEnabled: false, metronomeEnabled: false })).toBe("none");
  });

  it("walks first-run setup: EQ only if missing, then audio", () => {
    expect(firstRunSteps({ setupComplete: true, eqDirectory: "" })).toEqual([]);
    expect(firstRunSteps({ setupComplete: false, eqDirectory: "/eq" })).toEqual(["audio"]);
    expect(firstRunSteps({ setupComplete: false, eqDirectory: "  " })).toEqual([
      "eq",
      "audio",
    ]);
    expect(firstRunSteps({ setupComplete: false, eqDirectory: "" })).toEqual([
      "eq",
      "audio",
    ]);
    expect(setupStepLabel(0, 2)).toBe("Step 1 of 2");
    expect(setupStepLabel(1, 2)).toBe("Step 2 of 2");
    expect(setupStepLabel(0, 1)).toBe("Step 1 of 1");
  });

  it("formats timing offsets", () => {
    expect(formatOffset(null)).toBe("");
    expect(formatOffset(0)).toBe("on time");
    expect(formatOffset(0.32)).toBe("late 0.32s");
    expect(formatOffset(-0.15)).toBe("early 0.15s");
    expect(offsetClassName(0.32)).toBe("late");
    expect(offsetClassName(-0.15)).toBe("early");
    expect(offsetClassName(0)).toBe("");
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

  it("labels a chain as waiting until the first CH", () => {
    expect(chainStateLabel({ running: false, armed: true })).toBe("Waiting");
    expect(chainStateLabel({ running: true, armed: false })).toBe("Running");
    expect(chainStateLabel({ running: false, armed: false })).toBe("Stopped");
  });

  it("has no next-cast progress when you are not on the chain", () => {
    expect(youCastProgress(null)).toBe(0);
    expect(
      youCastProgress(
        snap({
          slots: [slot({ number: 2, player: "Two", isYou: false, progress: 0.8 })],
        }),
      ),
    ).toBe(0);
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

  it("speaks GO SOON when there is time left, otherwise GO NOW", () => {
    expect(nextUpSpeech(0)).toBe("GO NOW");
    expect(nextUpSpeech(-1)).toBe("GO NOW");
    expect(nextUpSpeech(1)).toBe("GO SOON");
    expect(nextUpSpeech(2)).toBe("GO SOON");
    expect(nextUpSpeech(0.5)).toBe("GO SOON");
  });

  it("plays a claim alert once when your number is taken", () => {
    const base = {
      warning: "002 is already taken.",
      urgent: true,
      lastWarning: null as string | null,
      armed: true,
    };
    expect(shouldPlayClaimAlert(base)).toBe(true);
    expect(shouldPlayClaimAlert({ ...base, lastWarning: base.warning })).toBe(false);
    expect(shouldPlayClaimAlert({ ...base, urgent: false })).toBe(false);
    expect(shouldPlayClaimAlert({ ...base, armed: false })).toBe(false);
  });

  it("toggles slot-taken and wrong-target alerts independently", () => {
    expect(
      isAlertEnabled({
        kind: "slotTaken",
        alertSlotTaken: true,
        alertWrongTarget: false,
      }),
    ).toBe(true);
    expect(
      isAlertEnabled({
        kind: "slotTaken",
        alertSlotTaken: false,
        alertWrongTarget: true,
      }),
    ).toBe(false);
    expect(
      isAlertEnabled({
        kind: "wrongTarget",
        alertSlotTaken: false,
        alertWrongTarget: true,
      }),
    ).toBe(true);
    expect(
      isAlertEnabled({
        kind: "wrongTarget",
        alertSlotTaken: true,
        alertWrongTarget: false,
      }),
    ).toBe(false);
    expect(
      isAlertEnabled({
        kind: "startChain",
        alertSlotTaken: false,
        alertWrongTarget: false,
      }),
    ).toBe(true);
    // A pace change is not one of the alerts you can switch off.
    expect(
      isAlertEnabled({
        kind: "pace",
        alertSlotTaken: false,
        alertWrongTarget: false,
      }),
    ).toBe(true);
  });

  it("shows a pace change as news and a mistake as a warning", () => {
    expect(alertBannerClass("pace", false)).toBe("banner pace dismissable");
    expect(alertBannerClass("slotTaken", false)).toBe("banner warn dismissable");
    expect(alertBannerClass("slotTaken", true)).toBe("banner danger dismissable");
    // Your own mistake still outranks the quieter pace styling.
    expect(alertBannerClass("pace", true)).toBe("banner danger dismissable");
  });

  it("speaks wrong-target once when it is you", () => {
    const base = {
      enabled: true,
      kind: "wrongTarget" as const,
      urgent: true,
      warning: "You CHed Portlia instead of Mluian.",
      lastSpoken: null as string | null,
    };
    expect(shouldSpeakWrongTarget(base)).toBe(true);
    expect(shouldSpeakWrongTarget({ ...base, lastSpoken: base.warning })).toBe(false);
    expect(shouldSpeakWrongTarget({ ...base, enabled: false })).toBe(false);
    expect(shouldSpeakWrongTarget({ ...base, urgent: false })).toBe(false);
    expect(shouldSpeakWrongTarget({ ...base, kind: "slotTaken" })).toBe(false);
  });

  it("speaks auto-take only to the person who got the number", () => {
    const base = {
      enabled: true,
      kind: "autoTake" as const,
      urgent: true,
      speech: "You got 3",
      lastSpoken: null as string | null,
    };
    expect(shouldSpeakAutoTake(base)).toBe(true);
    expect(shouldSpeakAutoTake({ ...base, lastSpoken: base.speech })).toBe(false);
    expect(shouldSpeakAutoTake({ ...base, enabled: false })).toBe(false);
    expect(shouldSpeakAutoTake({ ...base, urgent: false })).toBe(false);
    expect(shouldSpeakAutoTake({ ...base, kind: "wrongTarget" })).toBe(false);
  });

  it("speaks start-chain once and again after a later start", () => {
    const base = {
      enabled: true,
      kind: "startChain" as const,
      urgent: true,
      speech: "Chain is starting",
      warningAtMs: 10_000,
      lastSpoken: null as string | null,
      lastSpokenAt: null as number | null,
      lastWarningAtMs: null as number | null,
      now: 10_000,
    };
    expect(shouldSpeakStartChain(base)).toBe(true);
    expect(
      shouldSpeakStartChain({
        ...base,
        lastSpoken: base.speech,
        lastSpokenAt: 10_000,
        lastWarningAtMs: 10_000,
        now: 10_500,
      }),
    ).toBe(false);
    expect(
      shouldSpeakStartChain({
        ...base,
        lastSpoken: base.speech,
        lastSpokenAt: 10_000,
        lastWarningAtMs: 10_000,
        now: 18_000,
      }),
    ).toBe(false);
    expect(
      shouldSpeakStartChain({
        ...base,
        warningAtMs: 20_000,
        lastSpoken: base.speech,
        lastSpokenAt: 10_000,
        lastWarningAtMs: 10_000,
        now: 20_000,
      }),
    ).toBe(true);
    expect(shouldSpeakStartChain({ ...base, enabled: false })).toBe(false);
    expect(shouldSpeakStartChain({ ...base, urgent: false })).toBe(false);
    expect(shouldSpeakStartChain({ ...base, kind: "autoTake" })).toBe(false);
  });

  it("hides a warning after it is dismissed until a new one arrives", () => {
    expect(shouldShowWarning({ warning: null, dismissed: null })).toBe(false);
    expect(
      shouldShowWarning({ warning: "002 is already taken.", dismissed: null }),
    ).toBe(true);
    expect(
      shouldShowWarning({
        warning: "002 is already taken.",
        dismissed: "002 is already taken.",
      }),
    ).toBe(false);
    expect(
      shouldShowWarning({
        warning: "Could not skip 004.",
        dismissed: "002 is already taken.",
      }),
    ).toBe(true);
    expect(
      shouldShowWarning({
        warning: "Chain is starting.",
        dismissed: null,
        warningAtMs: 1_000,
        now: 10_999,
        dismissSeconds: 10,
      }),
    ).toBe(true);
    expect(
      shouldShowWarning({
        warning: "Chain is starting.",
        dismissed: null,
        warningAtMs: 1_000,
        now: 11_000,
        dismissSeconds: 10,
      }),
    ).toBe(false);
    expect(
      shouldShowWarning({
        warning: "Chain is starting.",
        dismissed: null,
        warningAtMs: 1_000,
        now: 60_000,
        dismissSeconds: 0,
      }),
    ).toBe(true);
  });

  it("builds slot class names", () => {
    expect(
      slotClassName(
        slot({ isCurrent: true, isNext: true, isYou: true, skipped: true }),
      ),
    ).toBe("slot is-current is-next is-you is-skipped");
  });
});

describe("overlay helpers", () => {
  it("clamps overlay opacity", () => {
    expect(clampOverlayOpacity(0.85)).toBe(0.85);
    expect(clampOverlayOpacity(0)).toBe(0.25);
    expect(clampOverlayOpacity(2)).toBe(1);
    expect(clampOverlayOpacity(Number.NaN)).toBe(0.85);
  });

  it("renders compact chain and rampage panels", () => {
    const html = overlayPanelHtml(snap({ running: true, youCastIn: 3 }), "CH");
    expect(html).toContain("CH");
    expect(html).toContain("Running");
    expect(html).toContain("Clericone");
    expect(html).toContain("Cast in 3.0s");
    // Your own number rides along with the countdown.
    expect(html).toContain('<span class="overlay-you-slot">001</span>');
    expect(html).toContain("is-you");
    expect(overlayPanelHtml(null, "Rampage")).toContain("Waiting");
  });
});

describe("other tanks alongside your chain", () => {
  // You are on 001 with Mluian; Beefwich has a four-cleric rotation of its own.
  function split(over: Partial<ChainSnapshot> = {}): ChainSnapshot {
    return snap({
      ...over,
      tank: "Mluian",
      yourTank: "Mluian",
      running: true,
      startedAtMs: 1_000,
      intervalSeconds: 2,
      tanks: [
        {
          name: "Mluian",
          from: null,
          to: null,
          intervalSeconds: 2,
          running: true,
          armed: false,
          startedAtMs: 1_000,
          currentNumber: 1,
          nextNumber: 2,
          beatTick: 0,
          isYou: true,
        },
        {
          name: "Beefwich",
          from: 5,
          to: 8,
          intervalSeconds: 2.5,
          running: true,
          armed: false,
          startedAtMs: 1_000,
          currentNumber: 5,
          nextNumber: 6,
          beatTick: 0,
          isYou: false,
        },
      ],
      slots: [
        slot({ number: 1, isYou: true, tank: "Mluian" }),
        slot({ number: 2, player: "Two", isYou: false, tank: "Mluian" }),
        slot({ number: 5, player: "Five", isYou: false, tank: "Beefwich" }),
        slot({ number: 6, player: "Six", isYou: false, tank: "Beefwich" }),
        slot({ number: 7, player: "Seven", isYou: false, tank: "Beefwich" }),
        slot({ number: 8, player: "Eight", isYou: false, tank: "Beefwich", skipped: true }),
        slot({ number: 9, player: "Nine", isYou: false, tank: "Beefwich" }),
      ],
    });
  }

  it("reports your own number for the header", () => {
    expect(yourSlotLabel(liveSnapshot(snap(), 3_000))).toBe("001");
    expect(yourSlotNumber(liveSnapshot(snap(), 3_000))).toBe(1);
    expect(yourSlotLabel(liveSnapshot(snap({ slotFormat: "letter" }), 3_000))).toBe("AAA");
    // Sitting out still tells you which number is yours.
    const out = snap({
      slots: [slot({ number: 4, isYou: true, skipped: true }), slot({ number: 2, isYou: false })],
    });
    expect(yourSlotLabel(liveSnapshot(out, 3_000))).toBe("004 (out)");
    // Not on the chain at all.
    const none = snap({ slots: [slot({ number: 2, player: "Two", isYou: false })] });
    expect(yourSlotLabel(liveSnapshot(none, 3_000))).toBe("—");
    expect(yourSlotNumber(liveSnapshot(none, 3_000))).toBeNull();
    expect(yourSlotLabel(null)).toBe("—");
    expect(yourSlotNumber(null)).toBeNull();
  });

  it("names your rotation and separates it from the rest", () => {
    const live = liveSnapshot(split(), 3_000)!;
    expect(yourTankName(live)).toBe("Mluian");
    expect(yourTankSlots(live).map((s) => s.number)).toEqual([1, 2]);
    // A chain with one tank has nothing to split off.
    expect(yourTankSlots(liveSnapshot(snap(), 3_000)!).map((s) => s.number)).toEqual([1, 2]);
  });

  it("keeps your own countdown while the other tank runs on its own clock", () => {
    const live = liveSnapshot(split(), 3_000)!;
    expect(live.slots.length).toBe(7);
    expect(live.running).toBe(true);
    expect(live.youCastIn).not.toBeNull();
    // Both rotations have their own current cleric.
    const mine = live.slots.filter((s) => s.tank === "Mluian");
    const theirs = live.slots.filter((s) => s.tank === "Beefwich");
    expect(mine.some((s) => s.isCurrent)).toBe(true);
    expect(theirs.some((s) => s.isCurrent)).toBe(true);
    // The snapshot's own current and next stay on your tank.
    expect(mine.map((s) => s.number)).toContain(live.currentNumber);
  });

  it("trims every other tank to the next few clerics", () => {
    const live = liveSnapshot(split(), 3_000)!;
    const sides = sideChains(live);
    expect(sides.length).toBe(1);
    const [side] = sides;
    expect(side.tank).toBe("Beefwich");
    expect(side.intervalSeconds).toBe(2.5);
    expect(side.state).toBe("Running");
    expect(side.clerics.length).toBe(3);
    // 008 sat out, so it is not one of the three coming up.
    expect(side.clerics.map((s) => s.number)).not.toContain(8);
    expect(side.waiting).toBe(1);
    expect(sideChains(live, 10)[0].waiting).toBe(0);
  });

  it("has nothing to show on the side of a single chain", () => {
    expect(sideChains(liveSnapshot(snap(), 3_000)!)).toEqual([]);
    expect(sideChains(null)).toEqual([]);
    expect(sideChainsHtml([])).toBe("");
  });

  it("renders a quiet panel per tank with the next cleric marked", () => {
    const live = liveSnapshot(split(), 3_000)!;
    const html = sideChainsHtml(sideChains(live));
    expect(html).toContain("side-chain");
    expect(html).toContain("Beefwich");
    expect(html).toContain("CH · Running · 2.5s");
    // The list starts at whoever is up next, not at the lowest number.
    expect(html.indexOf("006")).toBeLessThan(html.indexOf("007"));
    expect(html).toContain("+1 more");
    expect(html).toContain("is-next");
    // Your own clerics are not repeated on the side.
    expect(html).not.toContain("Clericone");
    // A rampage rotation on the side keeps its letters and its own label.
    const rch = liveSnapshot(split({ slotFormat: "letter" }), 3_000)!;
    const rchHtml = sideChainsHtml(sideChains(rch));
    expect(rchHtml).toContain("FFF");
    expect(rchHtml).toContain("RCH · Running · 2.5s");
  });

  it("leads with the chain you are on and sides the other one", () => {
    const ch = liveSnapshot(split(), 3_000)!;
    // You are on the cleric chain, so it leads even when a rampage runs too.
    const rampageWithoutYou = liveSnapshot(
      snap({
        slotFormat: "letter",
        tank: "Grendel",
        yourTank: null,
        slots: [
          slot({ number: 1, player: "Rampone", isYou: false, tank: "Grendel" }),
          slot({ number: 2, player: "Ramptwo", isYou: false, tank: "Grendel" }),
        ],
      }),
      3_000,
    )!;
    expect(primaryChain(ch, rampageWithoutYou).kind).toBe("chain");

    // Off the cleric chain and on rampage, rampage leads instead.
    const chWithoutYou = liveSnapshot(
      snap({ slots: [slot({ number: 2, player: "Two", isYou: false })] }),
      3_000,
    )!;
    const yourRampage = liveSnapshot(
      snap({
        slotFormat: "letter",
        tank: "Grendel",
        slots: [slot({ number: 1, isYou: true, tank: "Grendel" })],
      }),
      3_000,
    )!;
    const primary = primaryChain(chWithoutYou, yourRampage);
    expect(primary.kind).toBe("rampage");
    expect(yourSlotLabel(primary.live)).toBe("AAA");

    // The rail carries the other tanks on your chain plus every rampage one.
    const rail = chainRail(ch, yourRampage);
    expect(rail.map((side) => `${side.kind} ${side.tank}`)).toEqual([
      "CH Beefwich",
      "RCH Grendel",
    ]);
    // With nothing else running there is no rail at all.
    expect(chainRail(liveSnapshot(snap(), 3_000), null)).toEqual([]);
  });

  it("falls back to whichever chain has clerics when you are on neither", () => {
    const empty = liveSnapshot(snap({ slots: [] }), 3_000)!;
    const rampage = liveSnapshot(
      snap({
        slotFormat: "letter",
        slots: [slot({ number: 1, player: "Rampone", isYou: false })],
      }),
      3_000,
    )!;
    expect(primaryChain(empty, rampage).kind).toBe("rampage");
    expect(primaryChain(null, null).kind).toBe("chain");
  });

  it("swaps the big list when the side chain is pinned", () => {
    const ch = liveSnapshot(
      snap({ slots: [slot({ number: 2, player: "Two", isYou: false })] }),
      3_000,
    )!;
    const yourRampage = liveSnapshot(
      snap({
        slotFormat: "letter",
        tank: "Grendel",
        slots: [slot({ number: 1, isYou: true, tank: "Grendel" })],
      }),
      3_000,
    )!;
    expect(primaryChain(ch, yourRampage).kind).toBe("rampage");
    expect(primaryChain(ch, yourRampage, "chain").kind).toBe("chain");
    expect(primaryChain(ch, yourRampage, "rampage").kind).toBe("rampage");
    expect(sideChainsHtml(chainRail(yourRampage, ch))).toContain('data-kind="chain"');
  });

  it("keeps the overlay on your rotation alone", () => {
    const live = liveSnapshot(split(), 3_000)!;
    const html = overlayPanelHtml(live, "CH");
    expect(html).toContain("Clericone");
    expect(html).not.toContain("Seven");
  });
});

describe("update helpers", () => {
  it("describes an available update and progress", () => {
    expect(
      updateAvailableMessage({ version: "0.2.0", currentVersion: "0.1.0" }),
    ).toBe("Alfred 0.2.0 is available. You have 0.1.0.");
    expect(updateUpToDateMessage("0.1.0")).toBe("You're on the latest version (0.1.0).");
    expect(updateProgressLabel(0, 0)).toBe("Downloading update…");
    expect(updateProgressLabel(50, 100)).toBe("Downloading update… 50%");
    expect(updateProgressLabel(3, 2)).toBe("Downloading update… 100%");
  });

  it("trims release notes for the banner", () => {
    expect(updateNotesPreview("  New overlay.  ")).toBe("New overlay.");
    expect(updateNotesPreview("")).toBe("");
    expect(updateNotesPreview("a".repeat(12), 10)).toBe("aaaaaaaaaa…");
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

  it("recomputes remaining cast time while stopped without a next-up countdown", () => {
    const live = liveSnapshot(snap(), 2_500)!;
    const current = live.slots.find((s) => s.number === 2)!;
    expect(current.remainingSeconds).toBe(8.5);
    expect(current.progress).toBe(0.85);
    expect(live.youAreNextIn).toBeNull();
    expect(live.youCastIn).toBeNull();
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
    expect(live.slots.map((s) => s.number)).toEqual([1, 2, 3]);
    expect(live.youCastIn).toBeGreaterThan(1);
  });

  it("counts down a solo cleric to the next beat", () => {
    const live = liveSnapshot(
      snap({
        running: true,
        startedAtMs: 10_000,
        intervalSeconds: 2,
        currentNumber: 1,
        nextNumber: 1,
        slots: [slot({ number: 1, isYou: true })],
      }),
      11_000,
    )!;
    expect(live.currentNumber).toBe(1);
    expect(live.slots[0].isCurrent).toBe(true);
    expect(live.slots[0].isNext).toBe(false);
    expect(live.slots[0].remainingSeconds).toBe(9);
    expect(live.youCastIn).toBe(9);
  });

  it("starts a solo countdown from chain start, not a stale shout", () => {
    const live = liveSnapshot(
      snap({
        running: true,
        startedAtMs: 20_000,
        intervalSeconds: 2,
        currentNumber: 1,
        nextNumber: 1,
        slots: [
          slot({
            number: 1,
            isYou: true,
            lastShoutMs: 1_000,
            lastCastMs: 1_000,
          }),
        ],
      }),
      21_000,
    )!;
    expect(live.slots[0].remainingSeconds).toBe(9);
    expect(live.youCastIn).toBe(9);
  });

  it("resets a solo countdown when the cleric CHs", () => {
    const live = liveSnapshot(
      snap({
        running: true,
        startedAtMs: 10_000,
        intervalSeconds: 2,
        currentNumber: 1,
        nextNumber: 1,
        slots: [
          slot({
            number: 1,
            isYou: true,
            lastShoutMs: 20_000,
            lastCastMs: 20_000,
          }),
        ],
      }),
      21_000,
    )!;
    expect(live.slots[0].remainingSeconds).toBe(9);
    expect(live.slots[0].progress).toBe(0.9);
    expect(live.youCastIn).toBe(9);
  });

  it("updates your next-cast progress when clerics join or leave", () => {
    const at = 12_100;
    const two = liveSnapshot(
      snap({
        running: true,
        startedAtMs: 10_000,
        intervalSeconds: 2,
        slots: [
          slot({ number: 1, isYou: true }),
          slot({ number: 2, player: "Two", isYou: false }),
        ],
      }),
      at,
    )!;
    const three = liveSnapshot(
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
      at,
    )!;
    expect(two.youCastIn).toBeCloseTo(1.9, 5);
    expect(youCastProgress(two)).toBeCloseTo(1.9 / 4, 5);
    expect(three.youCastIn).toBeCloseTo(3.9, 5);
    expect(youCastProgress(three)).toBeCloseTo(3.9 / 6, 5);
    expect(three.youCastIn!).toBeGreaterThan(two.youCastIn!);
  });

  it("keeps a CH cast bar ticking while the chain is running", () => {
    const live = liveSnapshot(
      snap({
        running: true,
        startedAtMs: 10_000,
        intervalSeconds: 2,
        slots: [
          slot({ number: 1, isYou: true }),
          slot({
            number: 2,
            player: "Two",
            isYou: false,
            lastShoutMs: 12_000,
            lastCastMs: 12_000,
          }),
        ],
      }),
      14_000,
    )!;
    const two = live.slots.find((s) => s.number === 2)!;
    expect(two.castRemainingSeconds).toBe(8);
    expect(two.castProgress).toBe(0.8);
    expect(two.remainingSeconds).not.toBe(two.castRemainingSeconds);
  });

  it("starts the CH bar from lastCastMs", () => {
    expect(
      applyCastTiming(
        slot({ lastCastMs: 5_000, lastShoutMs: null }),
        10,
        6_000,
      ),
    ).toMatchObject({
      castRemainingSeconds: 9,
      castProgress: 0.9,
    });
  });

  it("puts next first and the cleric who just went last with a refilled bar", () => {
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
    expect(live.nextNumber).toBe(3);
    expect(live.slots.map((s) => s.number)).toEqual([3, 1, 2]);
    expect(live.slots[0]?.isNext).toBe(true);
    expect(live.slots[0]?.isCurrent).toBe(false);
    const justWent = live.slots[live.slots.length - 1]!;
    expect(justWent.number).toBe(2);
    expect(justWent.isCurrent).toBe(true);
    expect(justWent.remainingSeconds).toBeCloseTo(5.9, 5);
    expect(justWent.progress).toBeCloseTo(5.9 / 6, 5);
  });

  it("stops showing a running clock after the shouts go quiet", () => {
    const live = liveSnapshot(
      snap({
        running: true,
        shoutSync: true,
        heardStart: false,
        startedAtMs: 10_000,
        intervalSeconds: 2,
        slots: [
          slot({ number: 1, isYou: true, lastShoutMs: 10_000, lastCastMs: 10_000 }),
          slot({ number: 2, player: "Two", isYou: false, lastShoutMs: 12_000, lastCastMs: 12_000 }),
        ],
      }),
      20_000,
    )!;
    expect(clockIsIdle(
      snap({
        running: true,
        shoutSync: true,
        heardStart: false,
        startedAtMs: 10_000,
        intervalSeconds: 2,
        slots: [
          slot({ number: 1, isYou: true, lastShoutMs: 10_000, lastCastMs: 10_000 }),
          slot({ number: 2, player: "Two", isYou: false, lastShoutMs: 12_000, lastCastMs: 12_000 }),
        ],
      }),
      20_000,
    )).toBe(true);
    expect(live.running).toBe(false);
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
    expect(live.youAreNextIn).toBeNull();
    expect(live.youCastIn).toBeNull();
  });
});

function cleric(overrides: Partial<ClericReport> = {}): ClericReport {
  return {
    rank: 1,
    player: "Clericone",
    slots: [1],
    heals: 10,
    onTime: 8,
    early: 1,
    late: 1,
    missedTurns: 0,
    wrongTarget: 0,
    skips: 0,
    isYou: true,
    avgOffsetSeconds: 0.04,
    worstLateSeconds: null,
    score: 96,
    ...overrides,
  };
}

function report(overrides: Partial<SessionReport> = {}): SessionReport {
  return {
    id: 3,
    kind: "ch",
    live: false,
    startedAtMs: 1_700_000_000_000,
    endedAtMs: 1_700_000_192_000,
    durationSeconds: 192,
    tank: "Mluian",
    totalHeals: 24,
    missedTurns: 1,
    wrongTarget: 1,
    warnings: 2,
    avgOffsetSeconds: 0.12,
    score: 92,
    clerics: [cleric()],
    events: [
      {
        atMs: 1_700_000_000_000,
        kind: "start",
        player: "Raidlead",
        number: null,
        target: null,
        tank: "Mluian",
        offsetSeconds: null,
        text: "Raidlead started the chain",
      },
      {
        atMs: 1_700_000_002_000,
        kind: "heal",
        player: "Clericone",
        number: 1,
        target: "Mluian",
        tank: "Mluian",
        offsetSeconds: 0.4,
        text: "Clericone cast 001",
      },
    ],
    ...overrides,
  };
}

describe("session report helpers", () => {
  it("formats how long a session ran", () => {
    expect(formatDuration(0)).toBe("0s");
    expect(formatDuration(-4)).toBe("0s");
    expect(formatDuration(45)).toBe("45s");
    expect(formatDuration(192)).toBe("3m 12s");
    expect(formatDuration(3_720)).toBe("1h 02m");
  });

  it("labels a session by score, chain, length, and cleric count", () => {
    const label = sessionOptionLabel(report());
    expect(label).toContain("Score 92");
    expect(label).toContain("CH");
    expect(label).toContain("3m 12s");
    expect(label).toContain("1 cleric");
    expect(label).toContain("Mluian");
    expect(sessionOptionLabel(report({ live: true }))).toContain("running");
    expect(
      sessionOptionLabel(report({ kind: "rampage", clerics: [cleric(), cleric()] })),
    ).toContain("Rampage");
    expect(sessionKindLabel("rampage")).toBe("Rampage");
    expect(sessionSlotFormat("rampage")).toBe("letter");
    expect(sessionSlotFormat("ch")).toBe("number");
  });

  it("reports on-time share only when beats were measured", () => {
    expect(onTimeShare(cleric())).toBeCloseTo(0.8, 5);
    expect(onTimeShare(cleric({ onTime: 0, early: 0, late: 0 }))).toBeNull();
    expect(formatPercent(0.8)).toBe("80%");
    expect(formatPercent(null)).toBe("—");
  });

  it("grades a score by how close to the beat it is", () => {
    expect(scoreClassName(97)).toBe("score good");
    expect(scoreClassName(75)).toBe("score ok");
    expect(scoreClassName(40)).toBe("score bad");
  });

  it("leads with the score the whole chain earned", () => {
    const html = sessionScoreHtml(report());
    expect(html).toContain("92");
    expect(html).toContain("score good");
    expect(html).toContain("Chain score");
    expect(html).toContain("1 cleric");
    expect(html).toContain("24 casts");
    expect(sessionScoreHtml(report({ score: 64, clerics: [cleric(), cleric()] }))).toContain(
      "score bad",
    );
  });

  it("summarises a session as labelled facts", () => {
    const html = sessionSummaryHtml(report());
    expect(html).toContain("Mluian");
    expect(html).toContain("3m 12s");
    expect(html).toContain("Missed turns");
    expect(sessionSummaryHtml(report({ live: true }))).toContain("Running");
  });

  it("ranks clerics in a table and marks you", () => {
    const html = clericTableHtml(
      report({
        clerics: [
          cleric({ rank: 1 }),
          cleric({
            rank: 2,
            player: "Two",
            isYou: false,
            slots: [2],
            avgOffsetSeconds: 0.8,
            missedTurns: 2,
            wrongTarget: 1,
            score: 61,
          }),
        ],
      }),
    );
    expect(html).toContain("001");
    expect(html).toContain("cleric-row is-you");
    expect(html).toContain("late 0.80s");
    expect(html).toContain("score bad");
    expect(clericTableHtml(report({ clerics: [] }))).toContain("No casts");
  });

  it("uses rampage letters in a rampage report", () => {
    const html = clericTableHtml(report({ kind: "rampage" }));
    expect(html).toContain("AAA");
  });

  it("shows the newest timeline rows first and escapes names", () => {
    const html = sessionEventsHtml(report());
    const heal = html.indexOf("Clericone cast");
    const start = html.indexOf("started the chain");
    expect(heal).toBeGreaterThan(-1);
    expect(heal).toBeLessThan(start);
    expect(html).toContain("late 0.40s");
    expect(sessionEventsHtml(report({ events: [] }))).toContain("Nothing happened");

    const escaped = sessionEventsHtml(
      report({
        events: [
          {
            atMs: 1_700_000_000_000,
            kind: "warning",
            player: null,
            number: null,
            target: null,
            tank: null,
            offsetSeconds: null,
            text: "<script>",
          },
        ],
      }),
    );
    expect(escaped).toContain("&lt;script&gt;");
    expect(escaped).not.toContain("<script>");
  });

  it("keeps the timeline to the most recent rows", () => {
    const events = Array.from({ length: 12 }, (_, index) => ({
      atMs: 1_700_000_000_000 + index * 1_000,
      kind: "heal" as const,
      player: "Clericone",
      number: 1,
      target: "Mluian",
      tank: "Mluian",
      offsetSeconds: null,
      text: `cast ${index}`,
    }));
    const html = sessionEventsHtml(report({ events }), 3);
    expect(html).toContain("cast 11");
    expect(html).toContain("cast 9");
    expect(html).not.toContain("cast 8");
  });
});

describe("demo scenario helpers", () => {
  const scenario = {
    id: "chain",
    name: "Cleric chain",
    description: "Four clerics take numbers.",
    steps: 24,
    seconds: 65,
    configurable: false,
    clerics: 0,
    intervalSeconds: 0,
  };
  const raid = {
    ...scenario,
    id: "raid",
    name: "Full raid chain",
    configurable: true,
    steps: 220,
    seconds: 480,
    clerics: 20,
    intervalSeconds: 2,
  };

  it("labels a scenario with its length", () => {
    expect(demoScenarioLabel(scenario)).toBe("Cleric chain · 24 steps · 1m 05s");
    expect(demoScenarioLabel(raid)).toBe("Full raid chain · set up below");
  });

  it("clamps a roster to something Alfred will run", () => {
    expect(clampDemoOptions({})).toEqual({ clerics: 8, maxClerics: 20, minutes: 4 });
    expect(clampDemoOptions({ clerics: 99, maxClerics: 99, minutes: 900 })).toEqual({
      clerics: 24,
      maxClerics: 24,
      minutes: 30,
    });
    expect(clampDemoOptions({ clerics: 0, maxClerics: 0, minutes: 0 })).toEqual({
      clerics: 1,
      maxClerics: 1,
      minutes: 1,
    });
    // A roster cannot build down to fewer than the clerics already standing there.
    expect(clampDemoOptions({ clerics: 12, maxClerics: 4 }).maxClerics).toBe(12);
    expect(clampDemoOptions({ clerics: Number.NaN }).clerics).toBe(8);
    expect(clampDemoOptions({ clerics: 7.4 }).clerics).toBe(7);
  });

  it("knows how fast a roster can chain a 10s heal", () => {
    expect(fastestInterval(1)).toBe(10);
    expect(fastestInterval(3)).toBeCloseTo(3.4, 5);
    expect(fastestInterval(4)).toBe(2.5);
    expect(fastestInterval(10)).toBe(1);
    // The beat never goes under a second however many clerics pile on.
    expect(fastestInterval(20)).toBe(1);
    expect(fastestInterval(24)).toBe(1);
    expect(fastestInterval(0)).toBe(10);
  });

  it("explains the pace, the latecomers, and when your turn comes back", () => {
    const growing = demoPaceHint({ clerics: 8, maxClerics: 20, minutes: 4 });
    expect(growing).toContain("8 clerics split a 10s CH 1.3s apart");
    expect(growing).toContain("12 more take numbers mid-pull");
    expect(growing).toContain("chain is 20 at 1s");
    expect(growing).toContain("every 20s");
    const settled = demoPaceHint({ clerics: 3, maxClerics: 3, minutes: 2 });
    expect(settled).toContain("3.4s apart");
    expect(settled).toContain("nobody else shows up");
  });

  it("shows how long a run takes at the chosen speed", () => {
    expect(demoLengthLabel(raid, 1)).toBe("220 lines, about 8m 00s.");
    expect(demoLengthLabel(raid, 4)).toContain("2m 00s at 4×");
  });

  it("describes where the run is", () => {
    expect(demoProgressLabel({ running: false, step: 0, total: 0 })).toContain("Pick");
    expect(demoProgressLabel({ running: false, step: 0, total: 24 })).toContain("Ready");
    expect(demoProgressLabel({ running: true, step: 4, total: 24 })).toBe("Step 4 of 24…");
    expect(demoProgressLabel({ running: false, step: 24, total: 24 })).toBe(
      "Finished 24 steps.",
    );
  });

  it("renders a log entry with the line Alfred was fed", () => {
    const html = demoEntryHtml({
      atMs: 1_700_000_000_000,
      step: 4,
      total: 24,
      note: "Portlia follows",
      line: "[Fri Sep 18 16:27:00 2026] Portlia shouts, 'GG 002 CH -- Mluian'",
      applied: true,
      level: "line",
    });
    expect(html).toContain("4/24");
    expect(html).toContain("Portlia follows");
    expect(html).toContain("GG 002 CH -- Mluian");
    expect(html).not.toContain("ignored by Alfred");

    const ignored = demoEntryHtml({
      atMs: 1_700_000_000_000,
      step: 5,
      total: 24,
      note: "Chatter",
      line: "[Fri Sep 18 16:27:00 2026] Portlia shouts, 'hello'",
      applied: false,
      level: "skip",
    });
    expect(ignored).toContain("ignored by Alfred");

    const done = demoEntryHtml({
      atMs: 1_700_000_000_000,
      step: 24,
      total: 24,
      note: "Demo finished.",
      line: "",
      applied: false,
      level: "done",
    });
    expect(done).toContain("is-done");
    expect(done).not.toContain("24/24");
  });
});
