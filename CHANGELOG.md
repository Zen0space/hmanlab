# Changelog

All notable changes to this project are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.9] - 2026-05-18

### Added
- **Workspace trust gate.** Launching in a workspace that isn't on the persisted trusted list now shows a pre-TUI arrow-key + Enter prompt (`▎ Workspace trust`) before the alternate screen opens. Choice is saved in `~/.config/hmanlab/config.json` (`trusted_workspaces: []`) so repeat launches in the same folder don't re-ask. New `/trust` and `/untrust` slash commands flip it later. Destructive tools (`write_file`, `edit_file`, `multi_edit`, `run_command`, `save_memory`, `forget_memory`) auto-deny in untrusted workspaces via a short-circuit in the `StreamMsg::ConfirmRequest` handler — read-only tools (`read_file`, `list_dir`, `find_files`, `git_*`, `read_memory`) work either way. Sidebar + `@`-mention autocomplete reveal dotfiles (`.env`, `.hmanlab/`, `.editorconfig`) **only when trusted** — build-artefact dirs (`.git`, `target`, `node_modules`) stay hidden regardless. Non-TTY launches skip the prompt and stay untrusted.
- **`multi_edit` tool.** Batches N `{old_string, new_string}` edits to the same file in one call, one approval popup, one cumulative diff. Mirrors Claude Code's `MultiEdit` so any model trained on those traces reaches for it automatically; system prompt nudges the rest. All-or-nothing: a mid-batch validation failure (snippet missing, ambiguous, empty, no-op) leaves the on-disk file untouched and names the failing edit index. Eliminates the "6 sequential edit_file calls + 6 confirms for one README touch-up" pattern.
- **Model persistence across sessions and restarts.** New `last_model` / `last_provider` fields in `config.json` are written on every `/model` switch (picker or `/model <name>`) and reused on the next launch as the second-priority initial model (after `--model` flag, before alphabetical Ollama fallback). Loading a saved session via `/load` no longer overwrites your currently-selected model — a one-line info card surfaces the original session's model so you can `/model` back if you want.
- **Scrollable confirm popup.** Long diffs no longer truncate with "…N more lines hidden". `↑↓` scrolls one line, `PgUp/PgDn` ten, `Home/End` jump. Position indicator (`35/120 lines`) sits in the footer next to the `[y] allow [n] deny [Esc] deny` row, which stays pinned regardless of scroll. Per-prompt scroll resets so a new confirm starts at the top.
- **Click tool message to re-see diff.** `write_file` / `edit_file` / `multi_edit` / `save_memory` tool rows now carry the authorised diff (`diff: Option<Vec<DiffLine>>` on `ChatMessage`, `#[serde(skip)]` — UI-only). Click the row (or Ctrl+T expand-all) and the diff re-renders inline with the same green/red/dim/yellow palette as the confirm popup. Read-only tools still show their raw text output on expand.
- **Read-card consolidation.** Consecutive collapsed read-only tool calls coalesce into a single borderless tile with a `reading N files` header, list of paths in dim grey, and `BG_CARD` (catppuccin surface0) bg fill. Hidden messages don't break runs; expanding any single tool breaks it out of the group on the next frame.
- **Hover-highlight on card rows.** Cursor over a clickable card file row repaints with `BG_CARD_HOVER` (surface2) bg via direct buffer mutation post-render — gives a "this is clickable" affordance without adding a chevron or arrow icon. Tracked via `hover_x` / `hover_y` updated on every mouse event.
- **First-round visual polish.** Each message body line now carries a colored `▎` gutter bar in role accent (sky/green/lavender/peach). Role labels reformatted to `▎ user` / `▎ assistant` / `▎ tool` / `▎ system`. Plain-text capture for copy-on-drag stays as spaces so the bar glyph isn't grabbed by selection.

### Changed
- **`git_show` returns full message + diff** instead of `--stat`-only summary. One call now answers "read the latest commit" (`rev: "HEAD"`); big commits still tail-truncate via the shared `MAX_CMD_BYTES` cap, in which case the model falls back to `git_diff` with a `path:` filter.
- **`/workspace` is repeatable.** Relative paths used to canonicalise against the process CWD, so `/workspace ../sibling` always meant "from the dir I started in" and chained switches felt broken. Now resolves against `self.workspace` and expands `~` / `~/path` against `$HOME`. Error messages include the base path used so failures are diagnosable. No-op switches print `Already in workspace: …` instead of silently re-seeding.
- **`/settings` mutates in place instead of stacking a second card.** The placeholder "Account: loading…" card is now overwritten by the resolved `StreamMsg::Settings` reply via a stashed message index (`pending_settings_msg_idx`). Refresh feels like an actual refresh; falls back to append if the index drifted (`/clear` mid-flight, etc.).
- **Confirm prompts report `+NL -NL` instead of bytes.** Both `edit_file` and `write_file` now compute the diff first, derive line totals from it, then format the prompt — same shape used by `multi_edit`. Bytes still appear in the tool result text where the count is genuinely useful.
- **Agent loop cap raised: `MAX_TURNS` 10 → 50.** The previous panic-button triggered on legit multi-file work (15–30 tool calls is routine for refactors). Error message rewritten to call out "model likely stuck in a loop" instead of being mistaken for a chat-history limit.

### Fixed
- **`/workspace` only worked once.** Root cause: `PathBuf::canonicalize()` resolves relative paths against the process CWD, not the current workspace. See "Changed" above.

[0.1.9]: https://github.com/hmanlab/hmanlab/compare/0.1.8...0.1.9

## [0.1.8] - 2026-05-18

### Fixed
- **npm publish unblocked (third time's the charm).** The 0.1.7 publish job got past the OIDC 404 from 0.1.6 but failed with `E422 Unprocessable Entity — Error verifying sigstore provenance bundle: Failed to validate repository information: package.json: "repository.url" is "git+https://github.com/rekabytes/hmanlab.git", expected to match "https://github.com/hmanlab/hmanlab" from provenance`. npm cross-checks the package manifest's `repository.url` against the GitHub repo claim in the OIDC provenance bundle; the manifests still pointed at the pre-transfer `rekabytes/hmanlab` URL while the bundle (minted from the new repo location) said `hmanlab/hmanlab`. All `repository.url` / `homepage` / `bugs.url` fields across the umbrella `hmanlab` and the 5 `@hmanlab/<plat>` manifests now point at `hmanlab/hmanlab`. Same retry-loop-poisons-sigstore issue forced 0.1.7 to be abandoned (`@hmanlab/linux-x64@0.1.7` has a tlog entry that can never be republished); 0.1.8 is a fresh version so sigstore accepts new provenance.
- Updated stale `rekabytes/hmanlab` URLs in `Cargo.toml` (repository / homepage / documentation), `README.md` (CI + downloads badges, install/from-source rows), `install.sh` (`REPO`), `SECURITY.md` advisory link, `.github/ISSUE_TEMPLATE/config.yml`, `scripts/release.sh`, and the per-platform `npm/@hmanlab/*/README.md` files. CHANGELOG compare-links for historical releases left untouched (those refer to where the commits actually lived).

### Security
- Suppress `RUSTSEC-2024-0436` in cargo-audit (`.cargo/audit.toml`) and OSV-Scanner (`osv-scanner.toml`). The `paste` crate is unmaintained (INFO severity, no CVE, no fixed version possible since the repo is archived). Pulled in transitively via `ratatui 0.29`; will revisit if ratatui drops the dep or a maintained fork ships.
- `RUSTSEC-2026-0002` (`lru` unsound `IterMut`, real Stacked Borrows UB) **remains open**. The fix needs `ratatui >= 0.30` which requires `tui-textarea >= 0.8` — not yet released by upstream `rhysd/tui-textarea`. Tracking; will land in a follow-up release once `tui-textarea 0.8` ships.

## [0.1.7] - 2026-05-18

### Fixed
- **npm publish unblocked.** The 0.1.6 release reached the publish job (the workflow gate fix from 0.1.6 worked) but failed with `E404` on the first platform package: npmjs.com's Trusted Publisher entries still referenced the old `rekabytes/hmanlab` repo after the GitHub transfer, so OIDC tokens minted from `hmanlab/hmanlab` weren't accepted. The retry loop then poisoned sigstore's transparency log with a half-completed provenance entry for `@hmanlab/linux-x64@0.1.6`, permanently blocking that version from being re-released. Trusted Publisher entries on all 6 packages have been updated to `hmanlab/hmanlab`; 0.1.7 is a fresh version so sigstore will accept new provenance entries. Same code as 0.1.5/0.1.6.

## [0.1.6] - 2026-05-18

### Fixed
- **npm publish gate broken after the GitHub repository transfer.** The publish job in `release.yml` skipped silently on the `0.1.5` release because its `if:` was hard-coded to `github.repository_owner == 'rekabytes'`, and the repo had moved to the `hmanlab` org. The build matrix kept attaching binaries to the Release, but no `npm publish` ever ran — leaving npm pinned at `0.1.4` after a successful-looking release. Gate is now `github.repository == 'hmanlab/hmanlab'`. Reminder: the npmjs.com Trusted Publisher entry for each of the 6 packages must also point at the new repo for OIDC to mint a publish token.

### Notes
- Version-only catch-up release for npm. No runtime behaviour change vs. `0.1.5`; everything in the `0.1.5` changelog (the `/update` and `/settings` commands, the Esc-as-interrupt rewire, the README install/update overhaul) is what's actually being published to npm with `0.1.6`.

## [0.1.5] - 2026-05-17

### Added
- `/update` slash command — checks the npm registry for the latest published version, prints `current → latest`, and runs `npm install -g hmanlab@latest` in the background so you can keep chatting while it installs. Detects cargo installs (binary under `~/.cargo/bin` or `target/`) and surfaces `cargo install hmanlab --force` instead. Reports a clean fallback when `npm` isn't on `PATH`.
- `/settings` slash command (aliases: `/whoami`, `/account`, `/me`) — shows the running version (with an upgrade hint if npm has a newer one), active model, Ollama host, configured BYOK providers (presence only — never the key), workspace, and your authenticated account (name, email, training opt-in, admin badge) fetched from `/v1/auth/me`.
- `api::Client::fetch_me()` and a public `Me` struct in `src/api.rs` for the account look-up.
- Public `update_check::fetch_latest_npm()` and `update_check::newer()` so other modules can run a fresh registry check without going through the 24 h startup cache.

### Changed
- **`Esc` no longer quits.** In chat mode it now interrupts an in-flight generation (same effect as `Ctrl+C` mid-stream), or clears the draft input and dismisses any open `/`/`@` autocomplete popup, or no-ops. Quit is `Ctrl+C` (when idle), `Ctrl+Q`, `/quit`, or `/exit`.
- `README.md` install table now shows each method's exact binary location (`~/.local/bin/hmanlab`, `$(npm root -g)/../bin/hmanlab`, `~/.cargo/bin/hmanlab`) and warns against mixing channels — the most common cause of "update doesn't take effect."
- New `README.md` **Updating** section: a `which hmanlab` → command lookup table so users always know the right update path for their install, and explicit notes on `/update`'s curl-install limitation.
- `README.md` slash-commands and key-bindings tables refreshed to cover `/update`, `/settings`, the new `Esc` behavior, and `Ctrl+Q`.

## [0.1.4] - 2026-05-17

### Added
- Subpackage `README.md` + `LICENSE` for each `@hmanlab/<plat>` artifact, so `npmjs.com` and Socket can render docs and license info per-platform.
- `npm test` smoke check for `bin/hmanlab.js` (asserts the "no prebuilt binary" error path) and a `node-smoke` CI job on ubuntu/macos/windows.
- OpenSSF Scorecard workflow (`.github/workflows/scorecard.yml`) — weekly + on push to `main` + on branch-protection-rule changes; publishes results to the public dataset Socket reads.

### Changed
- **Supply chain — `npm publish` now uses OIDC trusted publishing.** The `release.yml` `publish` job no longer reads `NPM_TOKEN`; npm mints a short-lived token via GitHub's OIDC issuer, authorised by the Trusted Publisher entry on npmjs.com. Requires the `NPM_TOKEN` secret to be deleted from the repo once a release publishes green.
- Pinned every GitHub Action in `ci.yml` and `release.yml` to a commit SHA (with the human-readable version as a comment). Stops tag-based supply-chain attacks on the build/publish pipeline.
- Backfilled `0.1.1` / `0.1.2` / `0.1.3` entries above.

## [0.1.3] - 2026-05-16

### Changed
- UI redesign: Catppuccin Mocha palette applied across the TUI.
- README restructured with a centered hero, grouped sections, and collapsible details.

### Added
- Slash-command autocomplete and `@`-file autocomplete in the input box.

## [0.1.2] - 2026-05-16

### Added
- One-line curl installer (`curl -fsSL …/install.sh | sh`) and per-platform binaries attached to GitHub Releases.

### Fixed
- Release publish is now idempotent and retries on the npm packument race (409 "Failed to save packument") so a partial-failure re-run picks up where it left off.

## [0.1.1] - 2026-05-16

### Fixed
- Release workflow now fires on Release **publish**, not on bare tag push — lets you draft notes before kicking off the build + npm publish pipeline.

## [0.1.0] - 2026-05-16

### Added
- First public release.
- Background update check on startup. Hits `registry.npmjs.org/hmanlab` once per launch (cached 24 h, skipped on debug builds, 3 s timeout, fails silently), and surfaces a green `vX.Y.Z available — npm i -g hmanlab` notice in the header when a newer release is published. Never blocks startup; never modifies the user's machine.
- Streaming TUI chat against local Ollama (`/api/chat`) or any OpenAI-compatible `/chat/completions` endpoint.
- BYOK providers: z.ai (subscription + usage URLs), Ollama Cloud (Bearer auth against `ollama.com`), OpenCode Go (`opencode.ai/zen/go/v1`).
- Agentic tool calls: `read_file`, `list_dir`, `find_files`, `git_status`, `git_log`, `git_diff`, `git_show`, `edit_file`, `write_file`, `run_command` (30 s timeout), and the memory tools. Every mutating call asks for confirmation in the TUI.
- Persistent memory store at `~/.hmanlab/memory/` (user scope) and `<workspace>/.hmanlab/memory/` (project scope), with an auto-maintained `MEMORY.md` index injected into the system prompt.
- `/compact` slash command + auto-compaction once the prompt token count crosses ~24 000. Compaction summary is persisted as a rolling `compact-current` project memory.
- `/disconnect` slash command with an arrow-key picker that lists every provider with a stored key and lets you remove one.
- Session persistence to the hmanlab-api backend (default `https://be-ai.senireka.my`, override with `--api-url`).
- Sidebar workspace tree with click-to-expand directories and click-to-open files.
- Inline markdown rendering (`**bold**`, `` `code` ``) and OSC 52 clipboard copy on drag-select.
- First-run wizard for Ollama URL + hmanlab-api key, saved to `~/.config/hmanlab/config.json` (mode 600).
- npm packaging via the per-arch optional-dependency pattern: umbrella `hmanlab` + `@hmanlab/{linux-x64,linux-arm64,darwin-x64,darwin-arm64,win32-x64}`.

[0.1.8]: https://github.com/hmanlab/hmanlab/compare/0.1.7...0.1.8
[0.1.7]: https://github.com/hmanlab/hmanlab/compare/0.1.6...0.1.7
[0.1.6]: https://github.com/hmanlab/hmanlab/compare/0.1.5...0.1.6
[0.1.5]: https://github.com/rekabytes/hmanlab/compare/0.1.4...0.1.5
[0.1.4]: https://github.com/rekabytes/hmanlab/compare/0.1.3...0.1.4
[0.1.3]: https://github.com/rekabytes/hmanlab/compare/0.1.2...0.1.3
[0.1.2]: https://github.com/rekabytes/hmanlab/compare/0.1.1...0.1.2
[0.1.1]: https://github.com/rekabytes/hmanlab/compare/v0.1.0...0.1.1
[0.1.0]: https://github.com/rekabytes/hmanlab/releases/tag/v0.1.0
