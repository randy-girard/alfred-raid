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
make test       Run JS and Rust tests
make test-js    UI helper tests only
make test-rust  Rust tests only
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

1. On first launch Alfred tries to find your EverQuest folder in common Windows, Steam, Wine, and CrossOver locations. If it cannot, open **Settings** and paste or browse to it.
2. Alfred validates the folder, watches `Logs` (including through a symlink), and shows the character from `eqlog_Name_Server.txt`.
3. It tails the `eqlog_*.txt` file that is currently being written, starting at the **end** of the file, and switches if you log in another character.
4. Switch to **CH** or **Rampage** (or pick **Cleric Chain** / **Rampage Chain** from the tray).

Logging must be on in game (`/log`).

### Chat channels

Alfred reads **shout**, **out of character**, **group**, **guild**, **raid**, and **auction**. It ignores **say**, so tavern chatter will not start a chain.

CH macros and `!` commands both work in those channels.

### Cleric shouts

Alfred reads lines such as:

```text
Curaja shouts, 'GG 014 CH -- Wreckognize'
You shout, 'GG 001 CH -- Mluian'
Hanbox says out of character, 'GG CH 001 -- Beefwich'
Hanbox tells the group, 'CC 001 CH -- Beefwich'
```

It pulls out the cleric, chain number, and target. A line that looks like a CH macro but does not parse shows as a warning on the panel.

Supported message shapes (built in, not configurable): `GG 001 CH -- Tank`, `GG CH 001 -- Tank`, `CC 001 CH -- Tank`.

### Rampage chain

Rampage is a second chain with its own panel, clock, and commands. Macros use **RCH** and letters **AAA–ZZZ** (A=1 … Z=26). Display is always three letters.

```text
Curaja shouts, 'GG AAA RCH -- Mluian'
You shout, 'CC RCH CCC -- Beefwich'
```

| Command | Effect |
| --- | --- |
| `!rstartchain [tank]` | Start iterating (`!rstart`) |
| `!rendchain [tank]` | Stop iterating (`!rend`) |
| `!rmt <tank>` | Set the rampage main tank |
| `!rot <tank>` | Set the rampage off tank |
| `!rsplit <slot>` | Two-tank cut, e.g. `!rsplit CCC` |
| `!rtank <tank> <from> <to>` | Letter range on another tank |
| `!runtank <tank>` | Remove that tank |
| `!rtake AAA` | Move onto that letter. `!take AAA` also works |
| `!rmove AAA BBB` | Swap two letters. `!move AAA BBB` also works |
| `!rchain 2 [tank]` | Set the rampage interval |
| `!rreset-chain` | Clear rampage slots |

`!take 001` stays on the CH chain; `!take AAA` goes to rampage. CH macros never fill the rampage panel and RCH macros never fill CH.

Sound, metronome, and claim alerts follow whichever tab is visible so the two clocks do not talk over each other.

### Chain commands

| Command | Effect |
| --- | --- |
| `!startchain [tank]` | Start iterating (`!start`, `!start chain`, `!start-chain`). Name a tank to start only that chain |
| `!endchain [tank]` | Stop iterating; slots stay (`!end`, `!end chain`, `!end-chain`) |
| `!mt <tank>` | Set the main tank (owns numbers not in another range) |
| `!ot <tank>` | Set the off tank for a two-tank split |
| `!split <number>` | Cut: below this number stays MT, this number and above go to the off tank |
| `!tank <tank> <from> <to>` | Put a number range on another tank |
| `!untank <tank>` | Remove that tank; its numbers fall back to MT |
| `!skip [slot]` | Skip that slot. Omit it to skip your own CH and rampage slots. `!skip 001` is CH, `!skip AAA` is rampage |
| `!back [slot]` | Put that slot back in. Omit it to restore your own CH and rampage slots |
| `!reset-chain` | Clear chain slots and stop |
| `!take 001` | Move the speaker onto that number (leaves their old number empty). Does not start the clock |
| `!move 001 002` | Swap two numbers that are already set |
| `!chain 2 [tank]` | Set the interval to 2 seconds, or that tank only |

`!startchain` can be used mid-fight; Alfred re-anchors the clock to that moment and keeps using the live skip/take list.

If you join late and missed `!startchain`, Alfred watches CH/RCH shouts on the tank you took, infers the interval from those gaps (rounded to the nearest second), and starts **your local clock** so the latest shouter is current and you sit in the right place in that chain. Everyone else already has you from `!take` / your macro; this only syncs your app.

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

The chain panel shows each cleric, a countdown bar, **+/- timing** vs the expected beat, who is current / next, and a **Cast in Xs** timer for you. Your offset is measured from `You begin casting Complete Heal`. Other clerics are measured from their CH macro line.

If someone claims a number that is already taken, Alfred shows a warning. If that number was yours — or you took someone else's — the warning is louder and plays an alert sound.

In Settings, pick a next-up sound, metronome voice, or **None**. Sound and metronome cannot both be on. Skipped numbers are not spoken.

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

[chain]
interval_seconds = 2
cast_time_seconds = 10
```

Directory watching uses OS file events (FSEvents / inotify / ReadDirectoryChanges). A short size check on the active log is the fallback when those events miss a write (common with Wine / CrossOver).

## Tests

```bash
make test
```

That runs Vitest on the UI helpers and `cargo test` for parser, chain, config, log tailing, and raid-sequence engine tests.

## CI and releases

Pushes and pull requests run **Tests** (Vitest + `cargo test`).

**Release** is manual: Actions → Release → Run workflow.

- Leave **version** empty to bump from [conventional commits](https://www.conventionalcommits.org/) since the last `v*` tag (`feat` → minor, `fix` → patch, `BREAKING CHANGE` / `feat!` → major).
- Or type a semver such as `0.2.0`.
- The notes group those commits by type (Features, Fixes, …) and the job builds macOS (Apple Silicon + Intel), Windows, and Linux installers onto a GitHub release.

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
