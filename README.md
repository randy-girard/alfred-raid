# Alfred

Raid assistant for the Project 1999 guild **GoodGuys**. Alfred lives in the system tray, tails your EverQuest log, and drives a cleric-chain panel plus a separate rampage-chain panel.

It is a [Tauri](https://tauri.app/) 2 app: Rust backend, a small web UI, one native executable per OS (macOS, Windows, Linux).

## Requirements

- [Node.js](https://nodejs.org/) 20+
- [Rust](https://rustup.rs/) (stable)
- Platform extras:
  - **macOS** — Xcode Command Line Tools (`xcode-select --install`)
  - **Windows** — [Microsoft C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) and WebView2 (already on Windows 10/11)
  - **Linux** — WebKitGTK, e.g. `libwebkit2gtk-4.1-dev` plus the usual Tauri [Linux deps](https://v2.tauri.app/start/prerequisites/)

## Make targets

```text
make help       Show this list
make setup      Install Node dependencies
make dev        Run Alfred in development mode (alias: make run)
make build      Build a release app for this machine
make test       Run JS and Rust tests and write coverage/index.html
make coverage   Same as make test
make test-js    UI helper tests only (no coverage)
make test-rust  Rust tests only (no coverage)
make check      Typecheck TypeScript and cargo-check Rust
make fmt        Format Rust sources
make clean      Delete dist/ and src-tauri/target/
make config     Open the OS config folder (creates it if needed)
```

First-time setup is just:

```bash
make setup
make dev
```

`make dev` compiles the Rust side and opens the Alfred window. Closing the window hides it to the tray; use **Quit** on the tray menu to exit.

Release artifacts from `make build` land in `src-tauri/target/release/bundle/` (`.app` / `.dmg` on macOS, `.msi` / `.exe` on Windows, `.deb` / `.AppImage` on Linux).

## Using Alfred

1. On first launch Alfred tries to find your EverQuest folder in common Windows, Steam, Wine, and CrossOver locations. If it cannot, a short walkthrough asks you to paste or browse to it, then to pick an audio cue (next-up sound, metronome, or none). If the folder is already found, you only see the audio step.
2. Alfred validates the folder, watches `Logs` (including through a symlink), and shows the character from `eqlog_Name_Server.txt`.
3. It tails the `eqlog_*.txt` file that is currently being written, starting at the **end** of the file, and switches if you log in another character.
4. **Chain** is the only chain tab (or pick **Chain** from the tray): cleric and rampage rotations share it.
5. Tray **Overlay** (or **Open overlay** in Settings) shows both chains in a movable always-on-top window. Drag it when click-through is off, set opacity in Settings, then turn **Click-through** on so mouse clicks go through to EverQuest. Tray **Overlay** again hides it.
6. Tray **Test log** (or **Open test log** in Settings) opens a popup to inject chat lines as if they came from the log, without being in game. Set the speaker to **YOU** or another character name, pick the channel, then send the message.
7. Tray **Demo scenario** (or **Run demo scenario** in Settings) plays a scripted raid at you so you can watch the panels react. See [Demo scenarios](#demo-scenarios).
8. **Report** collects each chain that Alfred watched and ranks the clerics on it. See [Session reports](#session-reports).
9. Alfred checks GitHub for a newer release on launch. A banner offers **Install and restart**. Tray or Settings **Check for updates** does the same check on demand.

Logging must be on in game (`/log`).

Alfred scans quoted chat from any channel, including **say**. CH macros and `!` commands both work.

### Cleric shouts

Alfred reads lines such as:

```text
Curaja shouts, 'GG 014 CH -- Wreckognize'
You shout, 'GG 001 CH -- Mluian'
Hanbox says out of character, 'GG CH 001 -- Beefwich'
```

It pulls out the cleric, chain number, and target. A line that looks like a CH macro but does not parse shows as a warning on the panel.

Macros must start with the configured **guild tag** (default `GG`). Change it in Settings if you use another tag. `CA 015 CH`, `ST 002 CH`, and similar lines from other groups are ignored. Extra spaces such as `GG  006 CH  -- Tank` still work.

Supported message shapes: `GG 001 CH -- Tank`, `GG CH 001 -- Tank`.

### Rampage chain

Rampage is a second chain with its own clock and commands, on the same **Chain** tab. Macros use **RCH** (or letter + **CH**) and letters **AAA–ZZZ** (A=1 … Z=26). Display is always three letters.

```text
Curaja shouts, 'GG AAA RCH -- Mluian'
You shout, 'GG RCH CCC -- Beefwich'
You shout, 'GG RCH AAA -- Beefwich'
```

| Command | Effect |
| --- | --- |
| `!rt <tank>` | Set the rampage tank. One tank only; no off tank or split |
| `!rchain 2 [tank]` | Set the rampage interval |

`!take`, `!skip`, `!back`, and `!move` are shared: numbers go to CH, letters go to rampage. `!startchain` / `!stopchain` arm and stop both chains. CH macros never fill the rampage rotation and RCH macros never fill CH.

If you are on the rampage chain and not on a CH number, rampage takes the big list and the CH rotations move to the side column. Sound, metronome, and claim alerts follow the chain in the big list, so the two clocks never talk over each other.

### Chain commands

| Command | Effect |
| --- | --- |
| `!startchain [tank]` | Arm CH and rampage. The clock starts on the first CH or RCH from that cleric (`!start`, `!start chain`, `!start-chain`). Name a tank to arm only that tank |
| `!stopchain [tank]` | Stop CH and rampage; slots stay (`!stop`, `!stop chain`, `!stop-chain`) |
| `!mt <tank>` | Set the main tank (owns numbers not in another range) |
| `!ot <tank>` | Set the off tank for a two-tank split |
| `!split <number>` | Cut: below this number stays MT, this number and above go to the off tank |
| `!tank <tank> <from> <to>` | Put a number range on another tank |
| `!untank <tank>` | Remove that tank; its numbers fall back to MT |
| `!skip [slot]` | Skip that slot. Omit it to skip your own CH and rampage slots. `!skip 001` is CH, `!skip AAA` is rampage |
| `!back [slot]` | Put that slot back in. Omit it to restore your own CH and rampage slots |
| `!reset-chain` | Clear chain slots and stop |
| `!take [slot] [name]` | Omit the slot to take the next free CH number. Or `!take 001`, `!take AAA`, `!take 001 Portlia` |
| `!move 001 002` | Swap two CH numbers, or `!move AAA BBB` for rampage |
| `!chain 2 [tank]` | Set the interval to 2 seconds, or that tank only |

`!startchain` arms the chain. The clock does not run until the first CH or RCH after that, and it starts from whoever went. Mid-fight `!startchain` waits for the next shout the same way. Every Alfred shows **Chain is starting** and speaks it unless you turn that voice off in Settings.

Without `!startchain`, the list still follows the last CH or RCH so HP-based chains keep order. If shout gaps look like a timed chain and this Alfred never saw `!startchain`, it starts a **local clock** from those shouts so a late joiner can catch up.

If you join late and missed `!startchain`, take your number as usual. Alfred infers the interval from CH/RCH gaps on your tank (rounded to the nearest second) and syncs you behind the latest shouter. Everyone else already has you from `!take` / your macro; this only starts your app's timer. After you have heard `!startchain`, later shout gaps do not start a new clock — use `!startchain` again.

If you have a number, the panel, countdown, and metronome follow **your tank's chain only**.

Two-tank example:

```text
!mt Mluian
!ot Beefwich
!split 9
!chain 2
!chain 3 ot
!startchain
```

Numbers 001–008 are Mluian; 009+ are Beefwich. Or assign ranges with `!tank Beefwich 9 16`.

The **Commands** tab (and the tray **Commands** item) lists these in the app.

Everything lives on one **Chain** tab. The big list is the rotation you are on — your tank's CH numbers, or the rampage letters if that is the chain you are on. Every other rotation sits in a narrower, dimmer column to its right, one panel per tank, stacked: the other tanks on your own chain first, then the chain you are not on. Each of those panels is labelled **CH** or **RCH** and shows that tank's pace, whether its chain is running, and only the next three clerics up, so you can see how the other side is doing without losing your own place. Skipped clerics are left out of those three, and a **+2 more** line says how many are behind them. If you hold a seat on a side rotation too, it is marked in gold there. The overlay still shows both chains, with the one you are on first.

Your own number is called out in three places so you never have to hunt for it: **Your #** in the header row, on the **Cast in** banner next to the countdown, and on your own card, which carries a gold edge, a larger number, and a **You** tag. If you are sitting out, the header reads `004 (out)`.

Each card shows a **Next** bar until that cleric should CH again. While the chain is running, **Next** is at the top and the cleric whose beat just passed drops to the bottom with a refilled bar for their next turn. The **Cast in** banner has the same countdown as a bar, so you can watch one place; it updates if clerics join or leave. **Last hit** is late/early/on time, and a **CH** bar runs while their Complete Heal is in the air. A solo chain uses CH cast time for the Next countdown so you still see when to recast. `!startchain` waits for your first CH, then that bar counts down from the shout. Timing uses CH and RCH macros, not begin-cast lines.

If someone claims a number that is already taken, Alfred leaves the occupant in place and shows a warning to the person who tried to take it. That person is not added to the chain.

`!take` with no number assigns the next free CH slot. Every Alfred that sees that chat line assigns the same number and shows it. Only the person who got the slot hears it spoken. Turn the voice off in Settings. `!rtake` with no letter does the same for rampage.

If a tank is set, the chain is running, and someone already on the chain CHs a different target, Alfred names who they CHed versus the tank and speaks **Wrong target** to that user. Both alerts appear above the chain list, whichever chain they came from. Turn each one on or off in Settings.

When the pace changes, Alfred posts **The chain is now 2.5s, was 3.4s.** in a quieter teal banner, so a `!chain` you did not hear called never goes unnoticed. A tank on its own clock is named instead: **Beefwich is now 5.0s, was 2.5s.** Re-sending the pace the chain is already on says nothing. This alert is not spoken and has no Settings toggle.

Warnings and alerts hide after 10 seconds, or sooner if you dismiss them. Change that delay in Settings, or set it to 0 to keep them until you close them.

In Settings, pick a next-up voice, metronome, or **None**. Next-up says **GO SOON** when there is still time, or **GO NOW** at 0. Sound and metronome cannot both be on. Skipped numbers are not spoken.

## Session reports

Every chain Alfred watches is recorded as a **session** on the **Report** tab. A session opens on the first `!take`, `!startchain`, or CH, and closes on `!stopchain` or `!reset-chain` — or on its own after three quiet minutes. CH and rampage are recorded separately, so a pull with both gives you two sessions. Sessions where nobody cast are thrown away.

Each session in the picker leads with its **chain score**, so you can compare pulls without opening them.

Pick a session to see:

- **Chain score** — the whole chain as one number, at the top. It is each cleric's score weighted by how many casts they took, so a cleric who covered two turns badly cannot sink an otherwise clean pull.
- **Summary** — tank, start time, length, total casts, missed turns, wrong targets, and the average offset from the beat.
- **Clerics** — one row per cleric, ranked. Casts, share of casts on the beat, average offset, missed turns, wrong targets, and a score.
- **Timeline** — every cast, command, and warning in the session, newest first.

A cast within 0.25s of its beat counts as on time; anything else is early or late. A missed turn is a gap in a cleric's own rhythm that is close to a whole extra cycle, so sitting out one round shows up but a cleric who takes a number and never casts does not skew the rest.

Score starts at 100 and comes off for average drift off the beat (up to 40), each missed turn (8), each wrong target (5), and the share of casts that were late (up to 10). **Clear all sessions** empties the list.

The last 25 finished sessions are written to `sessions.json` next to `config.ini`, so reports survive a restart.

## Demo scenarios

**Run demo scenario** in Settings (or tray **Demo scenario**) opens a window that feeds Alfred a scripted raid, one log line at a time, through the same path a real log line takes. Nothing is sent to EverQuest. Pick a scenario, pick a speed from half to 4×, press **Start**, and watch the Chain and Report tabs move in the main window while the demo window logs each line as it goes in.

Every short scenario runs somewhere between 20 seconds and a minute, so you can watch one start to finish.

| Scenario | Clerics | Interval | What it exercises |
| --- | ---: | ---: | --- |
| Cleric chain | 4 | 2.5s | Clerics take numbers, the chain arms and runs three clean rounds, one cleric drifts late. |
| Mistakes and alerts | 3 | 3.4s | A number that is already taken, a CH on the wrong target, a macro Alfred cannot read, then a skip that re-paces the chain to 5s and a `!back` that puts it at 3.4s again. |
| Laggy cleric | 4 | 2.5s | One cleric is later every round until he misses his turn outright, gets skipped, and comes back on the beat. Watch his row on the Report tab. |
| Wrong tank | 2 + 2 | 5s each | A split chain where clerics on both sides CH the tank they are not assigned to. |
| Main tank goes down | 3 | 3.4s | `!mt` names a new tank mid-pull. One cleric's macro still points at the tank that died. |
| Off tank joins mid-pull | 4 → 2 + 2 | 2.5s, then 5s each | One rotation becomes two when `!ot` and `!split` land mid-chain, then `!untank` folds it back. |
| Two-tank split | 2 + 2 | 5s each | `!ot` and `!split` from the start, with the two rotations on their own clocks. |
| Rampage chain | 3 | 3.4s | RCH letters on their own clock. |
| CH and rampage together | 3 + 3 | 3.4s each | A CH chain on the main tank and a rampage chain on another mob at the same time, with one RCH on the wrong target. |
| Number collisions | 4 | 2.5s | Taken slots, `!take` with no number picking the next free one, the lead taking a number for someone else, and a cleric shouting a slot that is not his. |
| Joining a chain in progress | 3 → 4 | 3.4s | No `!chain` and no `!startchain`: Alfred infers the pace from the shouts it hears and syncs you behind the last cleric, then a real `!startchain` re-anchors everyone. |
| Short-handed chain | 2 → 1 → 3 | 5s, 10s, 3.4s | A pair 5s apart drops to a solo chain on the cast time, then a third cleric arrives and the lead tightens the pace. |
| Chain falls apart | 5 → 2 | 2s out to 5s | Clerics drop one at a time with a re-pace after every loss, until the chain cannot hold. |
| Broken macros | 3 | 3.4s | A CH macro with no number, one with the number after the spell, and a rampage macro with no letter — then the same clerics fixed. |
| Slot shuffle | 4 | 2.5s | `!move`, `!skip`, and `!back` on a running chain, then `!reset-chain` and a fresh pull. |
| Full raid chain | 1–24, growing | 10s ÷ clerics, never under 1s | A long pull that starts short-handed. Latecomers take numbers mid-chain and the lead re-paces, clerics skip out and come back, one drifts off the beat, one misses turns, and one CHes the wrong tank. |
| Everything | — | — | Every short scenario back to back, with a pause between pulls. About eleven minutes at real time, under three at 4×. |

The intervals come from the CH cast time: a cleric cannot cast again until their complete heal lands, so the tightest chain **N** clerics can hold is the 10s cast split **N** ways (rounded up to a tenth), with a floor of **1s** — no demo chain calls numbers faster than that, so past ten clerics the pace stops tightening and the rotation just takes longer to come back around. Each scenario sets that interval with `!chain` before it starts, and a cast is never scripted sooner than 10s after that cleric's last one, even when a `!skip` changes the roster mid-pull.

**Full raid chain** takes three numbers, in the **Roster** box on the demo page: how many clerics are on the chain at the pull (**Clerics at pull**), the roster to **build to**, and how many minutes the CH chain runs (**Chain runs**, up to 30). The box only appears for that scenario; every other pull runs the roster written into its script. The clerics in between wander in while the chain is already going — each takes the next free number and the lead calls a new interval — so you can watch the rotation tighten from a short-handed chain to a full one. Clerics also `!skip` out to med and `!back` in throughout the pull, and the lead re-paces on every change.

The interval is never a choice: it is the 10s cast split across whoever is on the chain at that moment (down to the 1s floor), so it drops as the roster fills and rises when someone steps out. The page shows the opening pace, the pace once it is full, how often your turn comes back around, and how long the run takes at the chosen speed. You take a number in the starting group and in the middle of the pack, so you are never a latecomer and the **Cast in** countdown always has room to run. The default — 8 clerics building to 20 over 4 minutes — leaves a full session on the Report tab to sort through, with joins, skips, misses, and a wrong target in it.

**Start** clears both chains before the first line, so a run never inherits the last one's clerics, and **Stop** ends the run at the next line and clears them again. A run that plays to the end keeps its final state on the panels so you can read it; send `!reset-chain` to clear that.

## Config

Alfred writes `config.ini` on first launch:

| OS | Path |
| --- | --- |
| macOS | `~/Library/Application Support/Alfred/config.ini` |
| Windows | `%APPDATA%\Alfred\config.ini` |
| Linux | `~/.config/alfred/config.ini` |

```ini
[general]
eq_directory = /path/to/EverQuest
always_on_top = false
sound_enabled = true
metronome_enabled = false
sound_lead_seconds = 2
tail_poll_ms = 150
setup_complete = true
alert_slot_taken = true
alert_wrong_target = true
alert_auto_take_sound = true
alert_start_chain_sound = true
alert_dismiss_seconds = 10

[chain]
interval_seconds = 2
cast_time_seconds = 10
tag = GG

[overlay]
opacity = 0.85
clickthrough = false

[window]
x = 120
y = 80
width = 460
height = 800
```

`cast_time_seconds` is a Complete Heal's 10s cast, so it is not on the Settings tab; edit the file if a server changes it. The interval is on the Settings tab, and `!chain` overrides it while a chain runs.

Alfred remembers the last on-screen position and size in that `[window]` section and restores them on the next launch. If the saved spot is no longer on a connected display, it keeps the size and lets the OS place the window. The overlay stores its own position, size, opacity, and click-through flag in `[overlay]`.

Directory watching uses OS file events (FSEvents / inotify / ReadDirectoryChanges). A short size check on the active log is the fallback when those events miss a write (common with Wine / CrossOver).

## Tests

```bash
make test
```

That runs Vitest on the UI helpers and Rust tests (via `cargo llvm-cov` when it is installed, otherwise `cargo test`). It writes a gitignored HTML report to `coverage/index.html`: overall totals, packages (`src`, `scripts`, `src-tauri`), per-file coverage, and a page for each file.

Rust coverage needs LLVM tools and `cargo-llvm-cov`:

```bash
rustup component add llvm-tools-preview
cargo install cargo-llvm-cov --locked
```

On macOS with Homebrew, `brew install cargo-llvm-cov llvm` also works.

<!-- coverage:start -->
**Line coverage:** 70.0% (5638 / 8058).

| Package | Coverage | Hit / lines |
| --- | ---: | ---: |
| src | 29.6% | 506 / 1709 |
| scripts | 70.3% | 426 / 606 |
| src-tauri | 81.9% | 4706 / 5743 |

The HTML report is gitignored. Run `make test` and open `coverage/index.html`.
<!-- coverage:end -->

## CI and releases

Pushes and pull requests run **Tests** with coverage, attach `coverage/index.html` as an artifact, and refresh the summary in this README.

**Release** is manual: Actions → Release → Run workflow.

- Leave **version** empty to bump from [conventional commits](https://www.conventionalcommits.org/) since the last `v*` tag (`feat` → minor, `fix` → patch, `BREAKING CHANGE` / `feat!` → major).
- Or type a semver such as `0.2.0`.
- The notes group those commits by type (Features, Fixes, …) and the job builds macOS (Apple Silicon + Intel), Windows, and Linux installers onto a GitHub release.

Alfred checks that GitHub release on launch and from **Check for updates** (Settings or the tray). If a newer version is out, a banner lets you download it, install in place, and restart. Signed updater files (`latest.json` and `.sig`) are attached to the release.

Auto-update needs two GitHub Actions secrets on this repo:

- `TAURI_SIGNING_PRIVATE_KEY` — contents of `src-tauri/updater.key` (gitignored). The matching public key is already in `src-tauri/tauri.conf.json`.
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` — optional; leave unset if the key has no password.

`make build` signs updater artifacts when that key file is present. Keep the private key backed up; if it is lost, existing installs cannot verify later updates.

The updater reads `https://github.com/randy-girard/alfred-raid/releases/latest/download/latest.json`. That URL only works if the GitHub release assets are downloadable without logging in (a public repo, or a public releases mirror). A private repo will not notify other machines until those assets are public.

Commit subjects should look like `feat(chain): add rampage panel` — short, no sign-off. See `.cursor/rules/commits.mdc`.

## Project layout

```text
src/                 UI (TypeScript)
src-tauri/src/       Rust backend
  parser.rs          Shout + guild-command parsing
  chain.rs           Cleric chain state
  log_watcher.rs     Logs folder watch + tail
  config.rs          INI load/save
  engine.rs          Log lines → chain updates
  report.rs          Chain sessions + cleric rankings
  demo.rs            Scripted demo scenarios
plan.md              Original feature plan
```
