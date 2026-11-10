# Keytastrophe

**Every key you press, live on screen.**

Keytastrophe is a tiny desktop app that visualizes your global keystrokes in real time. It was built for live demos, stream overlays, shortcut debugging, and anyone who wants to see what their keyboard is actually doing.

Built with [Tauri 2](https://v2.tauri.app/), [Vue 3](https://vuejs.org/), and Rust.

> [!WARNING]
> **Platform status — please read before downloading**
>
> - ✅ **macOS** — tested and working.
> - 🟡 **Windows** — implemented, but **UNTESTED**. Testing is planned next.
> - 🟡 **Linux** — implemented, but **UNTESTED**. The Linux backend uses X11, so Wayland is not supported.
>
> If you try Keytastrophe on Windows or Linux, please [open an issue](https://github.com/YOUR-USERNAME/keytastrophe/issues) with your OS version and what happened. Test reports from untested platforms are extremely valuable right now.

## Why "Keytastrophe"?

Because watching a rolling feed of every key you press is somewhere between a keyboard visualizer and a small disaster. The name stuck.

## Features

- Global keystroke capture while the app is running.
- Live rolling display of the last 30 keys.
- Human-readable names for special keys and modifiers: `Space`, `Return`, `Command`, `Shift`, arrows, function keys, and more.
- Native macOS `CGEventTap` backend.
- Automatically re-enables the macOS event tap if the system disables it.
- Errors are shown in the UI and printed to the terminal.
- No accounts, no analytics, no network calls.

## Privacy and responsible use

Keytastrophe listens to **every keystroke system-wide** while it is running. That is the point of the app, but it deserves a clear warning:

- Keystrokes are streamed only to the local app UI.
- Keystrokes are **not** written to disk.
- Keystrokes are **not** sent over the network.
- The UI keeps a small in-memory buffer of the last 30 keys.
- Quit the app when entering passwords or other sensitive information, and don't leave it running if you don't want your typing visible.

Please do not use Keytastrophe to monitor another person's input without their knowledge and consent. Depending on where you live, that can be illegal.

Some antivirus tools may flag any app that globally captures keystrokes. That is a reasonable heuristic — the full source is here for you to inspect.

## Requirements

- **macOS** (tested), or Windows/Linux (untested)
- [Rust](https://www.rust-lang.org/tools/install) (stable)
- [Bun](https://bun.sh/) (recommended) or npm
- [Tauri 2 prerequisites](https://v2.tauri.app/start/prerequisites/) for your platform
  - macOS: Xcode Command Line Tools
  - Windows: Microsoft C++ Build Tools + WebView2
  - Linux: WebKitGTK and the usual Tauri system libraries

## Run it

No prebuilt releases yet — build from source for now.

```bash
git clone https://github.com/YOUR-USERNAME/keytastrophe.git
cd keytastrophe
bun install
bun tauri dev
```

With npm instead of Bun:

```bash
npm install
npx tauri dev
```

Build a release bundle:

```bash
bun tauri build
```

The bundle ends up in `src-tauri/target/release/bundle/`.

### macOS permissions

macOS will not deliver global keystrokes until you allow it. If the app opens but no keys appear, this is almost always the cause.

1. Start the app with `bun tauri dev`.
2. Open **System Settings → Privacy & Security**.
3. Enable Keytastrophe under **Input Monitoring** and **Accessibility**.
   - Dev builds: add `src-tauri/target/debug/keytastrophe` — or enable Terminal/your IDE if that's what launched the app.
   - Release builds: add `Keytastrophe.app`.
4. Quit and restart the app.

If the status line says **"Could not create the keyboard event tap"**, permissions are the usual suspect. The terminal output also prints a `[keytastrophe]` message with details.

## How it works

1. The Vue frontend creates a Tauri [`Channel`](https://v2.tauri.app/develop/calling-frontend/#channels) and calls the `start_keystroke_stream` command.
2. Rust spawns a dedicated OS listener thread and streams key names back over the channel. The command returns immediately.
3. On macOS, the listener is a listen-only `CGEventTap` that reads characters with `CGEventKeyboardGetUnicodeString`.
4. On Windows and Linux, the listener uses [`rdev`](https://github.com/Narsil/rdev).
5. The frontend renders the last 30 keys and shows any listener errors.

A quick map of the code:

| Path | What lives there |
| --- | --- |
| `src/App.vue` | Channel setup, status line, and key display |
| `src-tauri/src/lib.rs` | Tauri command, managed state, macOS and fallback listeners |
| `src-tauri/tauri.conf.json` | App name, identifier, and window config |
| `src-tauri/Cargo.toml` | Rust dependencies, including platform-specific ones |

### Why not `rdev` on macOS?

`rdev` looks up the typed character through the Carbon TIS APIs. On macOS 26+, `TSMGetInputSourceProperty` asserts that it runs on the main queue — and `rdev` calls it from the event-tap thread, which kills the process with `SIGTRAP` on the first key press. Keytastrophe uses a native `CGEventTap` on macOS instead and avoids those APIs entirely. `rdev` is still used on Windows and Linux.

## Testing status

| Platform | Status | Notes |
| --- | --- | --- |
| macOS | ✅ Tested | Native event tap backend; needs Input Monitoring and Accessibility |
| Windows | 🟡 Untested | `rdev` backend included; testing is next |
| Linux | 🟡 Untested | `rdev` backend included; X11 only, Wayland not supported |

Useful checks while developing:

```bash
bun run build
cargo check --manifest-path src-tauri/Cargo.toml
```

## Roadmap

- [ ] Test on Windows.
- [ ] Test on Linux and investigate Wayland support.
- [ ] Add pause/resume for capture.
- [ ] Add a first-run permissions onboarding screen.
- [ ] Add a proper app icon.
- [ ] Add signed and notarized macOS builds.
- [ ] Add CI for all three platforms.

## Contributing

Issues and pull requests are welcome. Platform test reports are especially useful right now — if you run Keytastrophe on Windows or Linux, tell us:

- Your OS and version.
- Whether it built successfully.
- Whether keys appeared, and any terminal output.
- Whether you had to grant extra permissions.

Keep the macOS listener free of TIS/TSM calls (`TISCopyCurrentKeyboardInputSource`, `TSMGetInputSourceProperty`, `UCKeyTranslate`) unless the call is moved onto the main thread — those calls are what caused the original crash.

## License

MIT. See [LICENSE](LICENSE).

---

**TL;DR:** it shows every key you press. macOS works today. Windows and Linux are next. Quit it before typing your password.
