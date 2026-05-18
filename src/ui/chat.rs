//! The main chat surface (message history + input box).

use ratatui::{
    layout::{Position, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Padding, Paragraph, Wrap},
    Frame,
};

use crate::app::App;

use super::markdown::{parse_inline_md, wrap_styled_segments};
use super::theme;

/// Period of one full breath, in animation ticks. The ticker fires every
/// 120 ms (see `main::run`), so 30 ticks ≈ 3.6 s — slow enough to read as
/// breathing rather than blinking.
const BREATH_PERIOD: u64 = 30;

/// Sine-interpolate between two RGB colors using `tick` as phase. Returns
/// `lo` at the trough and `hi` at the peak of each breath cycle.
fn breath_color(tick: u64, lo: (u8, u8, u8), hi: (u8, u8, u8)) -> Color {
    let phase = (tick % BREATH_PERIOD) as f32 / BREATH_PERIOD as f32 * std::f32::consts::TAU;
    let t = (phase.sin() * 0.5) + 0.5;
    let lerp = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t) as u8;
    Color::Rgb(lerp(lo.0, hi.0), lerp(lo.1, hi.1), lerp(lo.2, hi.2))
}

/// Sky-tinted breath used for the "thinking" indicator. Pulses between a
/// muted version of the sky-blue role color and the full sky color.
fn thinking_breath(tick: u64) -> Color {
    breath_color(tick, (55, 90, 105), (137, 220, 235))
}

/// Peach-tinted breath used for the active tool row — peach is the
/// theme's primary accent, so an in-flight tool reads as the focal point.
fn tool_breath(tick: u64) -> Color {
    breath_color(tick, (115, 80, 60), (250, 179, 135))
}

/// Boil a tool call down to a `verb · primary-arg` summary the user can scan
/// at a glance. Tool-specific so the most-informative argument bubbles up:
/// `read_file({"path":"src/main.rs"})` → `read · src/main.rs`,
/// `run_command({"command":"cargo build"})` → `$ cargo build`. Unknown
/// tools fall back to `name(json)` so nothing is lost. Accepts the model's
/// TitleCase aliases (`Read`, `Bash`, …) the same way `tools::resolve_tool_alias` does.
fn tool_summary(name: &str, args: Option<&serde_json::Value>) -> String {
    let get_str = |key: &str| -> Option<String> {
        args.and_then(|v| v.get(key))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    };
    match name {
        "read_file" | "Read" => format!("read · {}", get_str("path").unwrap_or_else(|| "?".into())),
        "list_dir" | "LS" | "List" => {
            format!("ls · {}", get_str("path").unwrap_or_else(|| ".".into()))
        }
        "find_files" | "Glob" => {
            format!(
                "find · {}",
                get_str("pattern").unwrap_or_else(|| "?".into())
            )
        }
        "git_status" => "git status".into(),
        "git_log" => {
            let n = args
                .and_then(|v| v.get("limit"))
                .and_then(|v| v.as_i64())
                .map(|n| format!(" -n {n}"))
                .unwrap_or_default();
            format!("git log{n}")
        }
        "git_diff" => match get_str("path") {
            Some(p) if !p.is_empty() => format!("git diff · {p}"),
            _ => "git diff".into(),
        },
        "git_show" => format!(
            "git show · {}",
            get_str("rev").unwrap_or_else(|| "?".into())
        ),
        "edit_file" | "Edit" => format!("edit · {}", get_str("path").unwrap_or_else(|| "?".into())),
        "multi_edit" | "MultiEdit" => {
            let path = get_str("path").unwrap_or_else(|| "?".into());
            let count = args
                .and_then(|v| v.get("edits"))
                .and_then(|v| v.as_array())
                .map(|a| a.len())
                .unwrap_or(0);
            if count > 0 {
                format!("multi-edit · {path} ({count} edits)")
            } else {
                format!("multi-edit · {path}")
            }
        }
        "write_file" | "Write" => {
            format!("write · {}", get_str("path").unwrap_or_else(|| "?".into()))
        }
        "run_command" | "Bash" | "Shell" => {
            format!("$ {}", get_str("command").unwrap_or_else(|| "?".into()))
        }
        other => {
            let json = args
                .and_then(|v| serde_json::to_string(v).ok())
                .unwrap_or_else(|| "{}".into());
            format!("{other}({json})")
        }
    }
}

/// Membership info for the "reading N files" consolidation card. Set on
/// every message that's part of a run of consecutive collapsed read-only
/// tool calls (see `compute_read_groups`); `None` for everything else.
#[derive(Clone, Copy)]
struct ReadGroup {
    /// Index of the first visible message in the run — anchor for the
    /// `reading N files` header.
    first: usize,
    /// Index of the last visible message in the run — used to know when
    /// to emit the trailing spacer.
    last: usize,
    /// Total number of visible messages in the run. Drives the header
    /// count and decides whether to consolidate at all (requires ≥ 2).
    count: usize,
}

/// Compute consolidation groups: runs of ≥ 2 consecutive **collapsed**
/// read-only tool messages, skipping any hidden messages between them.
/// Returns a vec parallel to `messages` — `Some(g)` means msg is part of
/// the consolidation card, `None` means it renders standalone.
fn compute_read_groups(
    messages: &[crate::ollama::ChatMessage],
    expanded: &std::collections::HashSet<usize>,
) -> Vec<Option<ReadGroup>> {
    let mut out = vec![None; messages.len()];
    let visible: Vec<usize> = messages
        .iter()
        .enumerate()
        .filter(|(_, m)| !m.hidden)
        .map(|(i, _)| i)
        .collect();

    let groupable = |i: usize| -> bool {
        let m = &messages[i];
        m.role == "tool"
            && crate::tools::is_readonly_tool(m.name.as_deref().unwrap_or(""))
            && !expanded.contains(&i)
    };

    let mut run: Vec<usize> = Vec::new();
    let flush = |run: &mut Vec<usize>, out: &mut Vec<Option<ReadGroup>>| {
        if run.len() >= 2 {
            let info = ReadGroup {
                first: run[0],
                last: *run.last().unwrap(),
                count: run.len(),
            };
            for &k in run.iter() {
                out[k] = Some(info);
            }
        }
        run.clear();
    };
    for &idx in &visible {
        if groupable(idx) {
            run.push(idx);
        } else {
            flush(&mut run, &mut out);
        }
    }
    flush(&mut run, &mut out);
    out
}

/// Build one full-width line for a "reading N files" card. The bg color
/// fills from column 0 all the way to `width`, so consecutive card lines
/// stack into a single visual block without a border.
fn card_line(prefix: &str, text: &str, fg: Color, bg: Color, width: usize) -> Line<'static> {
    let used = prefix.chars().count() + text.chars().count();
    let pad = width.saturating_sub(used);
    let pad_str = if pad > 0 { " ".repeat(pad) } else { String::new() };
    Line::from(vec![
        Span::styled(prefix.to_string(), Style::default().bg(bg)),
        Span::styled(text.to_string(), Style::default().fg(fg).bg(bg)),
        Span::styled(pad_str, Style::default().bg(bg)),
    ])
}

/// Find the arguments the model passed to the tool call that produced
/// `messages[i]`. The chat-completion convention pairs each `tool` message
/// positionally with one entry of the preceding assistant's `tool_calls`,
/// so we walk back to the nearest assistant message and index by how many
/// tool messages sit between it and `i`.
fn args_for_tool_msg(
    messages: &[crate::ollama::ChatMessage],
    i: usize,
) -> Option<&serde_json::Value> {
    if messages.get(i)?.role != "tool" {
        return None;
    }
    let mut prior_tools: usize = 0;
    let mut asst_idx: Option<usize> = None;
    for j in (0..i).rev() {
        match messages[j].role.as_str() {
            "tool" => prior_tools += 1,
            "assistant" => {
                asst_idx = Some(j);
                break;
            }
            // user / info — no preceding assistant tool_calls relate to this tool
            _ => return None,
        }
    }
    let tcs = messages[asst_idx?].tool_calls.as_ref()?;
    tcs.get(prior_tools).map(|tc| &tc.function.arguments)
}

pub(super) fn render_chat(f: &mut Frame, area: Rect, app: &mut App) {
    // Chat is the always-focused surface — wear the active border colour.
    let block = theme::panel_block("chat", true).padding(Padding::horizontal(1));
    let inner = block.inner(area);

    // Stash inner geometry so the mouse selection code can hit-test.
    app.chat_x = inner.x;
    app.chat_y = inner.y;
    app.chat_w = inner.width;
    app.chat_h = inner.height;

    // 2-col gutter under each speaker label: rendered as a colored `▎` bar
    // in the role's color, but recorded in `text_lines` as two spaces so
    // copy-on-drag doesn't grab the bar glyph and selection cell-widths
    // still line up.
    let indent = "  ";
    let gutter_glyph = "▎ ";
    let content_width = (inner.width as usize).saturating_sub(indent.len()).max(10);

    let mut lines: Vec<Line> = Vec::new();
    // Parallel plain-text copy of `lines` so copy-on-drag can extract the
    // selected substring without re-parsing styled spans.
    let mut text_lines: Vec<String> = Vec::new();
    let mut ranges: Vec<(usize, u16, u16)> = Vec::with_capacity(app.messages.len());
    let last_idx = app.messages.len().saturating_sub(1);
    // Read-card grouping: consecutive collapsed read-only tool messages
    // (read_file, list_dir, git_*, etc.) coalesce into a single tinted
    // tile rather than rendering as N standalone rows. Expanding any tool
    // breaks it out of the group and shows its full output (and diff if
    // applicable) the normal way.
    let read_groups = compute_read_groups(&app.messages, &app.expanded_tools);
    let card_bg = theme::color::BG_CARD;
    let card_width = inner.width as usize;
    // Reset per-frame hover hit-test list — populated below as each card
    // file row is emitted, consumed after the paragraph render to paint
    // the hover overlay on whichever row is under the cursor.
    app.card_row_targets.clear();
    for (i, msg) in app.messages.iter().enumerate() {
        if msg.hidden {
            continue;
        }
        // Card-grouped rendering: header row (once) + one dim file row
        // per member. Each file row's line range maps back to its
        // message index so clicking it still toggles that single tool's
        // expansion (and breaks it out of the group on the next frame).
        if let Some(group) = read_groups[i] {
            let line_start = lines.len() as u16;
            if i == group.first {
                let header_text = format!("reading {} files", group.count);
                text_lines.push(format!("  {header_text}"));
                lines.push(card_line(
                    "  ",
                    &header_text,
                    theme::color::FG,
                    card_bg,
                    card_width,
                ));
            }
            let summary = tool_summary(
                msg.name.as_deref().unwrap_or("tool"),
                args_for_tool_msg(&app.messages, i),
            );
            text_lines.push(format!("    {summary}"));
            lines.push(card_line(
                "    ",
                &summary,
                theme::color::FG_DIM,
                card_bg,
                card_width,
            ));
            // Range covers just this msg's row inside the card.
            let line_end_excl = lines.len() as u16;
            let logical_row = line_end_excl.saturating_sub(1);
            ranges.push((i, logical_row, line_end_excl));
            // Record this row as a hover target — the post-render overlay
            // below uses this to know which screen row to repaint with the
            // hover bg when the cursor lands on it.
            app.card_row_targets.push((logical_row, i));
            // Spacer goes AFTER the last group member, not between them
            // (that's what makes the card read as one block).
            if i == group.last && i != last_idx {
                text_lines.push(String::new());
                lines.push(Line::from(""));
            }
            // Done with this message — skip the standalone render below.
            let _ = line_start;
            continue;
        }
        let line_start = lines.len() as u16;
        let is_tool = msg.role == "tool";
        let tool_expanded = is_tool && app.expanded_tools.contains(&i);
        let is_active_tool = is_tool && app.active_tool_msg_idx == Some(i);

        // Tool rows are detected as errored by their content's first line —
        // the agent loop in agent.rs wraps tool failures as "error: {e}".
        let tool_errored =
            is_tool && !is_active_tool && msg.content.trim_start().starts_with("error:");

        // Header line. For tool messages we collapse what used to be three
        // separate signals (`● ⏵ tool · name`, the `→ name(json)` echo on the
        // assistant message, and the trailing `(N lines)`) into one row that
        // reads as `verb · primary-arg` with state encoded in glyph + color.
        let (label, color) = match msg.role.as_str() {
            "tool" => {
                let summary = tool_summary(
                    msg.name.as_deref().unwrap_or("tool"),
                    args_for_tool_msg(&app.messages, i),
                );
                // Glyph carries fold state when settled; an open circle marks
                // the actively running tool (also picks up the breath color).
                let glyph = if is_active_tool {
                    "◌"
                } else if tool_expanded {
                    "⏷"
                } else {
                    "⏵"
                };
                let suffix = if is_active_tool {
                    "  · running…".to_string()
                } else if tool_errored {
                    "  · failed".to_string()
                } else if tool_expanded {
                    String::new()
                } else {
                    let body_lines = msg.content.lines().count().max(1);
                    format!("  ({body_lines}L)")
                };
                let color = if tool_errored {
                    theme::color::TOOL_ERROR
                } else {
                    theme::color::TOOL
                };
                (format!("{glyph} {summary}{suffix}"), color)
            }
            other => {
                let (text, c) = theme::role_label(other);
                (text.to_string(), c)
            }
        };
        let header_text = label.clone();
        text_lines.push(header_text.clone());
        let header_style = if is_active_tool {
            Style::default()
                .fg(tool_breath(app.anim_tick))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(color).add_modifier(Modifier::BOLD)
        };
        lines.push(Line::from(Span::styled(header_text, header_style)));

        // Only the most-recent message is actively streaming. Passing
        // `app.generating` to *every* assistant message made historical
        // turns from non-reasoning models (which never emit `</think>`) get
        // treated as mid-stream and re-render the breathing placeholder.
        // Scope "is generating" to the tail message instead.
        let is_streaming_here = app.generating && i == last_idx;

        // Split assistant content into `<think>…</think>` reasoning + the
        // user-facing answer. Reasoning-tuned models (Qwen3 family, including
        // hmanlab-ai v0.1) emit chain-of-thought in <think> blocks; we render
        // it as a foldable header (collapsed by default) so the chat stays
        // readable while still letting the user expand to inspect reasoning.
        // Other roles render verbatim.
        let (think_text, visible_content): (Option<&str>, &str) = if msg.role == "assistant" {
            split_thinking(&msg.content, is_streaming_here)
        } else {
            (None, msg.content.as_str())
        };

        // Render the thinking header (and body, if expanded) for this assistant turn.
        if let Some(think) = think_text {
            let thought_expanded = app.expanded_thoughts.contains(&i);
            let body_lines = think.lines().count().max(1);
            let chevron = if thought_expanded { "⏷" } else { "⏵" };
            let suffix = if thought_expanded {
                String::new()
            } else {
                format!(
                    "  ({body_lines} line{})",
                    if body_lines == 1 { "" } else { "s" }
                )
            };
            let header_text = format!("{indent}{chevron} thinking{suffix}");
            text_lines.push(header_text.clone());
            lines.push(Line::from(Span::styled(
                header_text,
                Style::default()
                    .fg(theme::color::FG_DIM)
                    .add_modifier(Modifier::ITALIC),
            )));
            if thought_expanded {
                for paragraph in think.split('\n') {
                    if paragraph.is_empty() {
                        text_lines.push(String::new());
                        lines.push(Line::from(""));
                        continue;
                    }
                    let body_style = Style::default()
                        .fg(theme::color::FG_DIM)
                        .add_modifier(Modifier::ITALIC);
                    let segments = parse_inline_md(paragraph, body_style);
                    let wrapped = wrap_styled_segments(segments, content_width);
                    for spans in wrapped {
                        let mut plain = String::with_capacity(content_width);
                        plain.push_str(indent);
                        for span in &spans {
                            plain.push_str(span.content.as_ref());
                        }
                        text_lines.push(plain);
                        let mut line_spans: Vec<Span<'static>> =
                            Vec::with_capacity(spans.len() + 1);
                        line_spans.push(Span::styled(
                            gutter_glyph.to_string(),
                            Style::default().fg(theme::color::FG_DIMMER),
                        ));
                        line_spans.extend(spans);
                        lines.push(Line::from(line_spans));
                    }
                }
            }
        }

        let trimmed = visible_content.trim_end_matches(['\n', '\r']);

        // Render body, unless this is a collapsed tool.
        let show_body = !is_tool || tool_expanded;
        // Tools that went through y/n approval (write_file, edit_file,
        // save_memory) carry the authorised diff. When the tool row is
        // expanded, we render that diff colourised instead of the raw
        // text result — re-using the same green/red/dim scheme as the
        // confirm popup. The text fallback below still runs for tools
        // without a diff (read_file, run_command, etc.).
        let render_diff = is_tool && tool_expanded && msg.diff.is_some();
        if render_diff {
            if let Some(diff) = msg.diff.as_ref() {
                let gutter_style = Style::default().fg(theme::color::FG_DIMMER);
                for dl in diff {
                    let style = match dl.kind {
                        crate::tools::DiffLineKind::Added => {
                            Style::default().fg(theme::color::SUCCESS)
                        }
                        crate::tools::DiffLineKind::Removed => {
                            Style::default().fg(theme::color::ERROR)
                        }
                        crate::tools::DiffLineKind::Context => {
                            Style::default().fg(theme::color::FG_DIM)
                        }
                        crate::tools::DiffLineKind::Summary => Style::default()
                            .fg(theme::color::WARNING)
                            .add_modifier(Modifier::BOLD),
                    };
                    for spans in wrap_styled_segments(
                        vec![(dl.text.clone(), style)],
                        content_width,
                    ) {
                        let mut plain = String::with_capacity(content_width);
                        plain.push_str(indent);
                        for span in &spans {
                            plain.push_str(span.content.as_ref());
                        }
                        text_lines.push(plain);
                        let mut line_spans: Vec<Span<'static>> =
                            Vec::with_capacity(spans.len() + 1);
                        line_spans.push(Span::styled(gutter_glyph.to_string(), gutter_style));
                        line_spans.extend(spans);
                        lines.push(Line::from(line_spans));
                    }
                }
            }
        } else if show_body {
            if trimmed.trim().is_empty() {
                if msg.role == "assistant"
                    && is_streaming_here
                    && msg.tool_calls.as_ref().map_or(true, |t| t.is_empty())
                {
                    // Breathing "thinking" line — the assistant has nothing to
                    // show yet (either still inside <think>…</think> or a
                    // non-reasoning model not having streamed any tokens).
                    // Color pulses on `app.anim_tick`; the line text is plain
                    // so copy-on-drag still captures something sensible.
                    let plain_text = format!("{indent}● thinking");
                    text_lines.push(plain_text);
                    let breath = thinking_breath(app.anim_tick);
                    lines.push(Line::from(vec![
                        Span::styled(gutter_glyph.to_string(), Style::default().fg(breath)),
                        Span::styled(
                            "● thinking",
                            Style::default()
                                .fg(breath)
                                .add_modifier(Modifier::BOLD),
                        ),
                    ]));
                }
            } else {
                let base_style = match msg.role.as_str() {
                    "info" => Style::default().fg(theme::color::SYSTEM),
                    "summary" => Style::default().fg(theme::color::SYSTEM),
                    "tool" if tool_errored => Style::default().fg(theme::color::TOOL_ERROR),
                    "tool" => Style::default().fg(theme::color::TOOL),
                    _ => Style::default().fg(theme::color::FG),
                };
                // Dim version of the role color for the body gutter — full
                // saturation reads as too loud when it runs down every line.
                let gutter_style = Style::default().fg(theme::color::FG_DIMMER);
                for paragraph in trimmed.split('\n') {
                    if paragraph.is_empty() {
                        text_lines.push(String::new());
                        lines.push(Line::from(""));
                        continue;
                    }
                    let segments = parse_inline_md(paragraph, base_style);
                    let wrapped = wrap_styled_segments(segments, content_width);
                    for spans in wrapped {
                        let mut plain = String::with_capacity(content_width);
                        plain.push_str(indent);
                        for span in &spans {
                            plain.push_str(span.content.as_ref());
                        }
                        text_lines.push(plain);
                        let mut line_spans: Vec<Span<'static>> =
                            Vec::with_capacity(spans.len() + 1);
                        line_spans
                            .push(Span::styled(gutter_glyph.to_string(), gutter_style));
                        line_spans.extend(spans);
                        lines.push(Line::from(line_spans));
                    }
                }
            }
        }

        // (Previously: an echo of `→ tool_name(json-args)` for each call on
        // the assistant message. That row duplicated the consolidated tool
        // header rendered when the matching `tool` message arrives, so it's
        // omitted — the model's text still renders above, and each tool call
        // gets one clean status row below.)

        let line_end_excl = lines.len() as u16;
        ranges.push((i, line_start, line_end_excl));

        // Spacer between messages, but not after the very last one
        if i != last_idx {
            text_lines.push(String::new());
            lines.push(Line::from(""));
        }
    }
    app.rendered_text_lines = text_lines;
    app.message_line_ranges = ranges;

    let total = lines.len() as u16;
    let visible = inner.height;
    let max_scroll = total.saturating_sub(visible);

    if app.follow {
        app.scroll = max_scroll;
    } else {
        app.scroll = app.scroll.min(max_scroll);
    }

    let para = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((app.scroll, 0));
    f.render_widget(para, area);

    // Hover overlay for card rows. Repaint the cell bg for whichever
    // card file row sits under the cursor — gives a "this is clickable"
    // affordance without adding a chevron / arrow. Done by mutating the
    // buffer post-render so we don't have to know the hover row at
    // line-build time (which is awkward because the scroll offset is
    // computed *after* lines are assembled).
    if app.hover_x >= inner.x
        && app.hover_x < inner.x.saturating_add(inner.width)
        && app.hover_y >= inner.y
        && app.hover_y < inner.y.saturating_add(inner.height)
    {
        // Translate hover screen Y back to a logical line index using the
        // same scroll offset the paragraph rendered with.
        let hovered_logical = (app.hover_y as u32)
            .saturating_sub(inner.y as u32)
            .saturating_add(app.scroll as u32);
        // O(N) over visible card rows — N is usually 2–10, never enough
        // to matter. Bail on the first match because each logical row
        // belongs to one card entry.
        let hit = app
            .card_row_targets
            .iter()
            .any(|(row, _)| *row as u32 == hovered_logical);
        if hit {
            let y = app.hover_y;
            let x_start = inner.x;
            let x_end = inner.x.saturating_add(inner.width).saturating_sub(1);
            let bg = theme::color::BG_CARD_HOVER;
            let buf = f.buffer_mut();
            for x in x_start..=x_end {
                if let Some(cell) = buf.cell_mut(Position::new(x, y)) {
                    let s = cell.style().bg(bg);
                    cell.set_style(s);
                }
            }
        }
    }

    // Paint the selection overlay on top of the chat. Cells inside the
    // (sel_start, sel_end) rectangle, clamped to the chat inner area, get the
    // REVERSED modifier so they look highlighted.
    if let (Some(start), Some(end)) = (app.sel_start, app.sel_end) {
        let ((sx, sy), (ex, ey)) = if (start.1, start.0) <= (end.1, end.0) {
            (start, end)
        } else {
            (end, start)
        };
        let cx_min = inner.x;
        let cx_max = inner.x.saturating_add(inner.width).saturating_sub(1);
        let cy_min = inner.y;
        let cy_max = inner.y.saturating_add(inner.height).saturating_sub(1);
        let row_lo = sy.max(cy_min);
        let row_hi = ey.min(cy_max);
        if row_lo <= row_hi {
            let buf = f.buffer_mut();
            for y in row_lo..=row_hi {
                let row_start = if y == sy { sx.max(cx_min) } else { cx_min };
                let row_end = if y == ey { ex.min(cx_max) } else { cx_max };
                if row_start > row_end {
                    continue;
                }
                for x in row_start..=row_end {
                    if let Some(cell) = buf.cell_mut(Position::new(x, y)) {
                        let s = cell.style().add_modifier(Modifier::REVERSED);
                        cell.set_style(s);
                    }
                }
            }
        }
    }
}

pub(super) fn render_input(f: &mut Frame, area: Rect, app: &mut App) {
    let first_line = app.input.lines().first().cloned().unwrap_or_default();
    let is_cmd = first_line.trim_start().starts_with('/');

    // Title encodes input mode; border colour echoes it so the box state
    // is scannable from across the screen.
    let (title, border_color) = if app.generating {
        (
            "▎ generating · Ctrl+C to cancel".to_string(),
            theme::color::WARNING,
        )
    } else if is_cmd {
        (
            "▎ command · Enter to run".to_string(),
            theme::color::ACCENT_ALT,
        )
    } else if app.yn_pending {
        (
            "▎ [Y] yes  ·  [N] no  ·  type to override".to_string(),
            theme::color::ASSISTANT,
        )
    } else {
        ("▎ message".to_string(), theme::color::ACCENT)
    };

    let block = ratatui::widgets::Block::default()
        .borders(ratatui::widgets::Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .title(Span::styled(
            format!(" {title} "),
            Style::default()
                .fg(border_color)
                .add_modifier(Modifier::BOLD),
        ))
        .padding(Padding::horizontal(1));
    app.input.set_block(block);
    f.render_widget(&app.input, area);
}

/// Split an assistant message into its `<think>` reasoning block and the
/// visible answer. Qwen3's chat template *prepends* `<think>\n` to the
/// assistant prefix, so streamed output starts directly with reasoning text
/// and emits `</think>` once the model is ready to answer.
///
/// Returns `(thinking, visible)` where:
///   - `thinking` is `Some(text)` if the model produced any reasoning content,
///     `None` if the message has no thinking (or thinking is empty).
///   - `visible` is the post-`</think>` answer.
///
/// While still streaming and `</think>` hasn't arrived yet, everything so far
/// is reasoning — we report `visible = ""` so the existing "generating dots"
/// branch renders progress without leaking raw thoughts. Once generation
/// finishes without ever emitting `</think>`, we fall back to treating the
/// whole content as visible (legacy / non-reasoning models).
fn split_thinking(s: &str, generating: bool) -> (Option<&str>, &str) {
    const CLOSE: &str = "</think>";
    const OPEN: &str = "<think>";
    if let Some(idx) = s.find(CLOSE) {
        let raw_think = &s[..idx];
        // Strip a leading "<think>" if present (some templates include it in
        // the streamed content rather than the prompt) plus surrounding
        // whitespace.
        let trimmed_think = raw_think
            .trim_start_matches(OPEN)
            .trim_matches(|c: char| c == '\n' || c == '\r' || c == ' ');
        let after = &s[idx + CLOSE.len()..];
        let visible = after.trim_start_matches(['\n', '\r']);
        if trimmed_think.is_empty() {
            (None, visible)
        } else {
            (Some(trimmed_think), visible)
        }
    } else if generating {
        // Mid-stream: thinking in progress, no answer yet. Hide content;
        // the generating-spinner branch will show a "…" placeholder.
        (None, "")
    } else {
        // Finished without a closing </think>: legacy / non-thinking model.
        // Render content as-is.
        (None, s)
    }
}
