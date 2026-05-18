---
name: bump-version
description: Bump hmanlab's version across every file that carries it (Cargo.toml, all npm package.json files, the umbrella's optionalDependencies block) and write a user-facing CHANGELOG entry. Use this when the user says "bump version to X", "release vX", "cut a release", or any similar phrasing — automatic invocation is appropriate when the intent is unambiguous. The skill ends with a clean working tree + a build-verified, committed bump on the current branch.
---

# Bump hmanlab's version

Use this when the user wants to ship a new release. The skill walks you through every file that carries the version number, the CHANGELOG entry's tone, and the verification gates that must pass before the version is committed.

## Files that carry the version

There are **7 files** with the version literal. Miss one and `release.yml`'s npm publish step fails (`E422` because the umbrella's `optionalDependencies` won't resolve to a published platform package).

| File | What changes |
|------|--------------|
| `Cargo.toml` | The `version = "..."` line at the top of `[package]`. Don't touch `[dependencies]` versions. |
| `npm/hmanlab/package.json` | The top-level `"version"` field AND each entry under `"optionalDependencies"` (5 of them: `linux-x64`, `linux-arm64`, `darwin-x64`, `darwin-arm64`, `win32-x64`). |
| `npm/@hmanlab/linux-x64/package.json` | Top-level `"version"`. |
| `npm/@hmanlab/linux-arm64/package.json` | Top-level `"version"`. |
| `npm/@hmanlab/darwin-x64/package.json` | Top-level `"version"`. |
| `npm/@hmanlab/darwin-arm64/package.json` | Top-level `"version"`. |
| `npm/@hmanlab/win32-x64/package.json` | Top-level `"version"`. |

There's also **`Cargo.lock`** — it picks the new version up automatically on the next `cargo build`. Capture the lockfile change in the same commit so the working tree stays clean.

`scripts/release.sh <NEW>` already does the find-and-replace across all of the above. Prefer running it over manual edits — it has a sanity check (`grep -RIn "\"$CUR\"" npm/ Cargo.toml`) that refuses to commit if any file still references the old version.

## Procedure

1. **Verify a clean working tree** before starting. `scripts/release.sh` refuses if there are uncommitted changes; you should too. If the user has uncommitted work that belongs in the release, commit it first as a separate "feature" commit, *then* bump.

2. **Pick the version**. Semver:
   - `X.Y.z` patch bump (`0.1.9 → 0.1.10`) for fixes, small improvements, no behavior breaks.
   - `X.y.0` minor bump for new features or non-breaking changes that materially expand the surface.
   - `x.0.0` major bump for breaking changes — config schema changes, removed commands, etc.

3. **Update the version literals**. Either:
   - Run `bash scripts/release.sh <NEW>` (it sed-s every file then commits + tags). **Don't push yet** — the changelog is still empty.
   - Or, if the user wants the bump split from the tag/commit step, do the sed manually:
     ```
     sed -i -E "0,/^version = \"<OLD>\"$/s//version = \"<NEW>\"/" Cargo.toml
     find npm -name 'package.json' -print0 | while IFS= read -r -d '' f; do
       sed -i -E "s/(\"version\": \")<OLD>(\")/\1<NEW>\2/" "$f"
       sed -i -E "s/(\"@hmanlab\/[a-z0-9-]+\": \")<OLD>(\")/\1<NEW>\2/g" "$f"
     done
     ```
   - Verify nothing still references the old version: `grep -RIn "\"<OLD>\"" npm/ Cargo.toml | grep -v node_modules` must come up empty.

4. **Refresh `Cargo.lock`**: `cargo build --release` once so the lockfile picks up the new package version. Capture the change in the bump commit.

5. **Write the CHANGELOG entry**. See "Writing user-facing CHANGELOG entries" below — this is the part the skill exists for.

6. **Verify the build still passes**:
   - `cargo build --release` (must be clean — zero warnings)
   - `cargo clippy --release --all-targets -- -D warnings` (must be clean)
   - `cargo test --release` (5/5 pass; if you've added tests, they pass too)
   - `cargo fmt --check` on any files you touched (existing fmt nits in unrelated files are fine to leave)

7. **Commit**. Conventional shape used by this repo:
   ```
   release: v<NEW> — <one-line headline>
   ```
   The headline is a tight summary of the release theme: the one or two most important things shipped. NOT a full changelog dump.

8. **Push when the user asks**, not before. The push triggers `release.yml` → builds binaries → tags get cut → npm publish. Don't surprise the user with a release.

## Writing user-facing CHANGELOG entries

The CHANGELOG is read by **end users** of the TUI, not contributors. Follow these rules.

### Tone

- **Plain English, not implementation prose.** Describe what *changed for the user*, not what code moved.
- **No file paths, line numbers, function names, types, or env var dumps.** Those belong in commit messages.
- **No root-cause analysis or postmortem detail.** "X stopped working because Y" → just "X works now." Reasoning lives in PR descriptions.
- **No infra / CI / packaging mechanics.** Users don't care that "the npm provenance bundle expected hmanlab/hmanlab and got rekabytes/hmanlab". They care that "npm package links work again."

### Structure

Use the [Keep a Changelog](https://keepachangelog.com) sections in this order, omitting any that have no entries:

```
## [<NEW>] - <YYYY-MM-DD>

### Added
- **One-line headline.** One or two sentences explaining what the user can now do.

### Changed
- **One-line headline.** What's different from before, from the user's perspective.

### Fixed
- **One-line headline.** What used to be broken and now works.

### Security
- One-line plain note about advisories suppressed or vulnerabilities patched. Optional.

[<NEW>]: https://github.com/hmanlab/hmanlab/compare/<OLD>...<NEW>
```

The trailing compare link is required — every prior release has one.

### Examples

Good (the actual v0.1.9 entries — what we want to keep doing):
- ✅ "**Your chosen model is remembered.** After you switch models with /model, that choice persists across restarts. Loading a saved session no longer overrides your current model — you'll see which model the session used and can switch back with /model if you want."
- ✅ "**Scrollable confirmation popups.** When reviewing a long diff before approving, you can now scroll through it with arrow keys, Page Up/Down, and Home/End. The position indicator (e.g. `35/120 lines`) is shown in the footer."
- ✅ "**npm package links updated.** After the project moved to a new GitHub organization, the npm package pages still pointed at the old repository URL. All links have been updated."

Bad (technical, infra, postmortem — what we used to do and now avoid):
- ❌ "Auto-compaction now triggers via `last_prompt_tokens > AUTO_COMPACT_THRESHOLD` in `send_to_llm`; the buffered user message is replayed after `CompactionDone` lands via `pending_after_compact`."
- ❌ "Fixed E422 in the npm publish step. The umbrella `package.json`'s `repository.url` cross-checked against the GitHub OIDC provenance bundle and mismatched after the org transfer. Sigstore transparency-log entries can't be republished, so 0.1.7 was abandoned."
- ❌ "Bumped MAX_TURNS in `agent.rs` from 10 to 50 because users with reasoning models hit the cap during multi-file refactors."

If the change is genuinely impossible to explain without naming an internal symbol (rare — usually the user's mental model has a non-technical noun for the same thing), use the user-facing noun and link the internal symbol from the commit message instead.

### What to include vs. omit

| Type of change | In CHANGELOG? |
|----------------|---------------|
| New slash command / keyboard shortcut / feature | **Yes**, with a usage example. |
| Behavior fix the user would have noticed | **Yes**, one line. |
| Visible UI change (colors, layout, copy) | **Yes** if non-trivial. |
| Performance improvement the user would notice | **Yes**, with rough magnitude. |
| Security advisory the user should know about | **Yes**, under `### Security`, one line. |
| Internal refactor with no user-visible effect | **No.** Skip entirely. |
| Test additions, CI tweaks, dev-tooling | **No.** |
| Dep bumps with no behavior change | **No** unless the dep is user-visible (e.g. an LLM SDK that adds a new model). |
| Provider model-list updates (BYOK seed lists) | **Yes** if the user can now use new models. |

When in doubt, ask: "would a user who just installed this tool care about this line?" If no, drop it.

## After the bump

- Confirm with the user before `git push`. The push tags the release and starts the publishing pipeline.
- If the build fails post-push (release.yml red), the version is already burnt — the next release must be a new patch number. Sigstore transparency logs do not let you republish.
