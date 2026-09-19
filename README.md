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
4. Switch to **CH** or **Rampage** (or pick **Cleric Chain** / **Rampage Chain** from the tray).
5. Tray **Overlay** (or **Open overlay** in Settings) shows both chains in a movable always-on-top window. Drag it when click-through is off, set opacity in Settings, then turn **Click-through** on so mouse clicks go through to EverQuest. Tray **Overlay** again hides it.
6. Tray **Test log** (or **Open test log** in Settings) opens a popup to inject chat lines as if they came from the log, without being in game. Set the speaker to **YOU** or another character name, pick the channel, then send the message.
7. Alfred checks GitHub for a newer release on launch. A banner offers **Install and restart**. Tray or Settings **Check for updates** does the same check on demand.

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

Rampage is a second chain with its own panel, clock, and commands. Macros use **RCH** (or letter + **CH**) and letters **AAA–ZZZ** (A=1 … Z=26). Display is always three letters.

```text
Curaja shouts, 'GG AAA RCH -- Mluian'
You shout, 'GG RCH CCC -- Beefwich'
You shout, 'GG RCH AAA -- Beefwich'
```

| Command | Effect |
| --- | --- |
| `!rt <tank>` | Set the rampage tank. One tank only; no off tank or split |
| `!rchain 2 [tank]` | Set the rampage interval |

`!take`, `!skip`, `!back`, and `!move` are shared: numbers go to CH, letters go to rampage. `!startchain` / `!stopchain` arm and stop both panels. CH macros never fill the rampage panel and RCH macros never fill CH.

Sound, metronome, and claim alerts follow whichever tab is visible so the two clocks do not talk over each other.

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

Each card shows a **Next** bar until that cleric should CH again. While the chain is running, **Next** is at the top and the cleric whose beat just passed drops to the bottom with a refilled bar for their next turn. The **Cast in** banner has the same countdown as a bar, so you can watch one place; it updates if clerics join or leave. **Last hit** is late/early/on time, and a **CH** bar runs while their Complete Heal is in the air. A solo chain uses CH cast time for the Next countdown so you still see when to recast. `!startchain` waits for your first CH, then that bar counts down from the shout. Timing uses CH and RCH macros, not begin-cast lines.

If someone claims a number that is already taken, Alfred leaves the occupant in place and shows a warning to the person who tried to take it. That person is not added to the chain.

`!take` with no number assigns the next free CH slot. Every Alfred that sees that chat line assigns the same number and shows it on every tab. Only the person who got the slot hears it spoken. Turn the voice off in Settings. `!rtake` with no letter does the same for rampage.

If a tank is set, the chain is running, and someone already on the chain CHs a different target, Alfred names who they CHed versus the tank and speaks **Wrong target** to that user. Both alerts appear on every tab. Turn each one on or off in Settings.

Warnings and alerts hide after 10 seconds, or sooner if you dismiss them. Change that delay in Settings, or set it to 0 to keep them until you close them.

In Settings, pick a next-up voice, metronome, or **None**. Next-up says **GO SOON** when there is still time, or **GO NOW** at 0. Sound and metronome cannot both be on. Skipped numbers are not spoken.

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
**Line coverage:** 72.5% (5269 / 7268).

| Package | Coverage | Hit / lines |
| --- | ---: | ---: |
| src | 31.4% | 443 / 1411 |
| scripts | 70.3% | 426 / 606 |
| src-tauri | 83.8% | 4400 / 5251 |

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
plan.md              Original feature plan
```
