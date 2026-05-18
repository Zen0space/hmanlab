//! Overlay surfaces: model picker, session picker, add-model dialog,
//! confirm popup. All four share the `centered_rect` helper.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Clear, List, ListItem, Padding, Paragraph, Wrap},
    Frame,
};

use crate::app::{AddModelStep, App, InlinePopup, PickerEntry, SLASH_COMMANDS};
use crate::tools;

use super::markdown::{parse_inline_md, wrap_styled_segments};
use super::theme;

pub(super) fn render_picker(f: &mut Frame, full: Rect, app: &App) {
    let area = centered_rect(60, 60, full);
    f.render_widget(Clear, area);
    let items: Vec<ListItem> = app
        .picker_entries
        .iter()
        .enumerate()
        .map(|(i, e)| {
            let label = e.display();
            let entry_color = match e {
                PickerEntry::Ollama(_) => theme::color::FG,
                PickerEntry::Extra(_) => theme::color::ASSISTANT,
                PickerEntry::AddZaiSubscription
                | PickerEntry::AddZaiUsage
                | PickerEntry::AddOllamaCloud
                | PickerEntry::AddOpenCode => theme::color::USER,
            };
            let style = if i == app.picker_index {
                Style::default()
                    .fg(ratatui::style::Color::Black)
                    .bg(theme::color::ACCENT_ALT)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(entry_color)
            };
            ListItem::new(format!(" {} ", label)).style(style)
        })
        .collect();
    let list = List::new(items).block(
        theme::popup_block("select model — ↑↓ Enter Esc", false).padding(Padding::horizontal(1)),
    );
    f.render_widget(list, area);
}

pub(super) fn render_disconnect_picker(f: &mut Frame, full: Rect, app: &App) {
    let area = centered_rect(60, 50, full);
    f.render_widget(Clear, area);
    let items: Vec<ListItem> = app
        .disconnect_entries
        .iter()
        .enumerate()
        .map(|(i, e)| {
            let label = format!(" {} — {} ", e.label, e.preview);
            let style = if i == app.disconnect_index {
                Style::default()
                    .fg(ratatui::style::Color::Black)
                    .bg(theme::color::ERROR)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme::color::ERROR)
            };
            ListItem::new(label).style(style)
        })
        .collect();
    let title = format!(
        "disconnect provider ({}) — ↑↓ Enter Esc",
        app.disconnect_entries.len()
    );
    let list =
        List::new(items).block(theme::popup_block(&title, true).padding(Padding::horizontal(1)));
    f.render_widget(list, area);
}

pub(super) fn render_session_picker(f: &mut Frame, full: Rect, app: &App) {
    let area = centered_rect(80, 70, full);
    f.render_widget(Clear, area);
    let items: Vec<ListItem> = app
        .session_picker_items
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let id_str = s.id.replace('-', "");
            let short = &id_str[..id_str.len().min(8)];
            let model = s.model.as_deref().unwrap_or("?");
            let title: String = s.title.chars().take(70).collect();
            let label = format!(" {short}  [{model}]  {title} ");
            let style = if i == app.session_picker_index {
                Style::default()
                    .fg(ratatui::style::Color::Black)
                    .bg(theme::color::ACCENT_ALT)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme::color::FG)
            };
            ListItem::new(label).style(style)
        })
        .collect();
    let title = format!(
        "sessions ({}) — ↑↓ Enter Esc",
        app.session_picker_items.len()
    );
    let list =
        List::new(items).block(theme::popup_block(&title, false).padding(Padding::horizontal(1)));
    f.render_widget(list, area);
}

pub(super) fn render_add_model(f: &mut Frame, full: Rect, app: &mut App) {
    let area = centered_rect(70, 40, full);
    f.render_widget(Clear, area);

    let (title, body, hint) = match app.add_model_step {
        AddModelStep::Key => {
            let (title, body) = match app.add_model_provider.as_str() {
                p if p == crate::config::OLLAMA_CLOUD_PROVIDER => (
                    " add Ollama Cloud key ".to_string(),
                    "Paste your Ollama Cloud API key (generate one at \
                     https://ollama.com/settings/keys). After saving, the \
                     free-tier models (glm-4.7, gpt-oss:120b-cloud, \
                     qwen3-coder-next) become selectable in /model and you'll \
                     be switched to glm-4.7 by default.\n\n\
                     Note: glm-5.1, glm-5, deepseek, kimi, and minimax are \
                     subscription-only on Ollama Cloud — selecting them \
                     returns a 403 until you upgrade at ollama.com/upgrade.\n\n\
                     The key is stored in ~/.config/hmanlab/config.json (mode \
                     0600) and only sent to ollama.com — never to hmanlab-api."
                        .to_string(),
                ),
                p if p == crate::config::OPENCODE_PROVIDER => (
                    " add OpenCode Go key ".to_string(),
                    "Paste your OpenCode API key (generate one at \
                     https://opencode.ai/zen). This provider points at the \
                     Go subscription endpoint — requests bill against your \
                     Go plan, not pay-per-credit.\n\n\
                     After saving, the Go-tier coding models become \
                     selectable in /model: glm-5.1, glm-5, \
                     qwen3.6/3.5-plus, kimi-k2.5/k2.6, minimax-m2.5/m2.7. \
                     Default is glm-5.1.\n\n\
                     Heads-up: Free-tier models (big-pickle, *-free) live \
                     on Zen's endpoint, not Go's — they're not in this \
                     provider's catalog and would 401 ModelError if added \
                     manually. Closed-weight models (claude-*, gpt-*, \
                     gemini-*) use non-OpenAI wire shapes and aren't routed \
                     through this provider yet.\n\n\
                     The key is stored in ~/.config/hmanlab/config.json (mode \
                     0600) and only sent to opencode.ai — never to hmanlab-api."
                        .to_string(),
                ),
                p if p == crate::config::ZAI_USAGE_PROVIDER => (
                    " add z.ai (usage-based) key ".to_string(),
                    "Paste your z.ai usage-based API key. After saving, all three \
                     z.ai models (glm-4.7, glm-4.6, glm-5.1) become selectable in \
                     /model and you'll be switched to glm-4.7 by default.\n\n\
                     The key is stored in ~/.config/hmanlab/config.json (mode \
                     0600) and only sent to z.ai — never to hmanlab-api."
                        .to_string(),
                ),
                _ => (
                    " add z.ai key ".to_string(),
                    "Paste your z.ai coding-plan API key. After saving, all three \
                     z.ai models (glm-4.7, glm-4.6, glm-5.1) become selectable in \
                     /model and you'll be switched to glm-4.7 by default.\n\n\
                     The key is stored in ~/.config/hmanlab/config.json (mode \
                     0600) and only sent to z.ai — never to hmanlab-api."
                        .to_string(),
                ),
            };
            (title, body, "Enter to save  ·  Esc to cancel")
        }
    };

    let block = theme::popup_block(title.trim(), false).padding(Padding::horizontal(1));
    let inner = block.inner(area);
    let content_width = (inner.width as usize).saturating_sub(2).max(10);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(2),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .split(inner);

    f.render_widget(block, area);

    let mut lines: Vec<Line> = Vec::new();
    for paragraph in body.split('\n') {
        if paragraph.is_empty() {
            lines.push(Line::from(""));
            continue;
        }
        let segments = parse_inline_md(paragraph, Style::default());
        for spans in wrap_styled_segments(segments, content_width) {
            lines.push(Line::from(spans));
        }
    }
    f.render_widget(
        Paragraph::new(lines).style(Style::default().fg(theme::color::FG)),
        chunks[0],
    );

    app.add_model_input.set_block(
        ratatui::widgets::Block::default()
            .borders(ratatui::widgets::Borders::ALL)
            .border_type(ratatui::widgets::BorderType::Rounded)
            .border_style(Style::default().fg(theme::color::ACCENT))
            .title(Span::styled(
                " ❯ input ",
                Style::default()
                    .fg(theme::color::ACCENT)
                    .add_modifier(Modifier::BOLD),
            ))
            .padding(Padding::horizontal(1)),
    );
    f.render_widget(&app.add_model_input, chunks[1]);

    f.render_widget(
        Paragraph::new(hint).style(Style::default().fg(theme::color::FG_DIM)),
        chunks[2],
    );
}

pub(super) fn render_confirm(f: &mut Frame, full: Rect, app: &mut App) {
    // Bigger popup than before — edit_file/write_file prompts include old/new
    // string previews that easily overflow the 70x40 box the original
    // run_command-style confirm was sized for.
    let area = centered_rect(80, 60, full);
    f.render_widget(Clear, area);

    let prompt = app
        .pending_confirm
        .as_ref()
        .map(|r| r.prompt.as_str())
        .unwrap_or("(no pending request)");

    // Always treat the confirm popup as "danger" — every prompt that lands
    // here is a destructive or shell-touching action that wants user attention.
    let block = theme::popup_block("confirm action", true).padding(Padding::horizontal(1));
    let inner = block.inner(area);
    f.render_widget(block, area);

    // Reserve the bottom line for the y/n footer ALWAYS, even if the prompt
    // body would otherwise overflow it. This is the fix for "I only saw the
    // prompt, not the y/n options" on long edit_file diffs.
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),
            Constraint::Length(1), // spacer
            Constraint::Length(1), // footer
        ])
        .split(inner);
    let content_area = chunks[0];
    let footer_area = chunks[2];

    let content_width = (content_area.width as usize).saturating_sub(1).max(10);
    let mut lines: Vec<Line> = Vec::new();
    for paragraph in prompt.split('\n') {
        if paragraph.is_empty() {
            lines.push(Line::from(""));
            continue;
        }
        let segments = parse_inline_md(paragraph, Style::default());
        for spans in wrap_styled_segments(segments, content_width) {
            lines.push(Line::from(spans));
        }
    }
    // Coloured diff (only populated for edit_file / write_file). One blank
    // separator above so the header and diff body don't visually fuse.
    if let Some(req) = app.pending_confirm.as_ref() {
        if !req.diff.is_empty() {
            lines.push(Line::from(""));
            for dl in &req.diff {
                let style = match dl.kind {
                    tools::DiffLineKind::Added => Style::default().fg(theme::color::SUCCESS),
                    tools::DiffLineKind::Removed => Style::default().fg(theme::color::ERROR),
                    tools::DiffLineKind::Context => Style::default().fg(theme::color::FG_DIM),
                    tools::DiffLineKind::Summary => Style::default()
                        .fg(theme::color::WARNING)
                        .add_modifier(Modifier::BOLD),
                };
                // Wrap each diff line at content_width so long lines stay
                // inside the popup. Re-apply the colour to every wrapped
                // sub-line so a long delete doesn't turn black halfway.
                for chunk in wrap_styled_segments(vec![(dl.text.clone(), style)], content_width) {
                    lines.push(Line::from(chunk));
                }
            }
        }
    }
    // Clamp scroll so End / PgDn-past-end snaps to the last full screen.
    // `Paragraph::scroll` doesn't itself clamp, so without this an u16::MAX
    // would render an empty box.
    let total_lines = lines.len() as u16;
    let visible = content_area.height;
    let max_scroll = total_lines.saturating_sub(visible);
    if app.confirm_scroll > max_scroll {
        app.confirm_scroll = max_scroll;
    }
    let scroll = app.confirm_scroll;

    let body = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .scroll((scroll, 0));
    f.render_widget(body, content_area);

    // Footer: scroll position (if any) on the right of the action keys, so
    // long diffs make it obvious there's more below.
    let scroll_hint = if max_scroll > 0 {
        let shown_end = (scroll as usize + visible as usize).min(total_lines as usize);
        format!("  ·  ↑↓ PgUp/PgDn scroll  ·  {}/{} lines", shown_end, total_lines)
    } else {
        String::new()
    };
    let footer = Paragraph::new(Line::from(vec![
        Span::styled(
            "[y]",
            Style::default()
                .fg(theme::color::SUCCESS)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" allow   ", Style::default().fg(theme::color::FG)),
        Span::styled(
            "[n]",
            Style::default()
                .fg(theme::color::ERROR)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" deny   ", Style::default().fg(theme::color::FG)),
        Span::styled("[Esc]", Style::default().fg(theme::color::FG_DIM)),
        Span::styled(" deny", Style::default().fg(theme::color::FG_DIM)),
        Span::styled(scroll_hint, Style::default().fg(theme::color::FG_DIM)),
    ]));
    f.render_widget(footer, footer_area);
}

/// Inline autocomplete overlay — rendered floating above the input box
/// while the user is typing `/<command>` or `@<path>`. Sized to the
/// content (capped) and anchored to the input row.
pub(super) fn render_inline_popup(f: &mut Frame, input_area: Rect, app: &App) {
    // Build the rows + title from whichever popup is active.
    let (title, rows, hint) = match &app.inline_popup {
        InlinePopup::None => return,
        InlinePopup::Slash(p) => {
            if p.matches.is_empty() {
                return;
            }
            let rows: Vec<(String, String, bool)> = p
                .matches
                .iter()
                .enumerate()
                .map(|(row_idx, &cmd_idx)| {
                    let cmd = &SLASH_COMMANDS[cmd_idx];
                    (
                        format!("/{}", cmd.name),
                        cmd.desc.to_string(),
                        row_idx == p.index,
                    )
                })
                .collect();
            (
                "slash commands",
                rows,
                "↑↓ pick  •  Tab/Enter complete  •  Esc cancel",
            )
        }
        InlinePopup::File(p) => {
            if p.matches.is_empty() {
                return;
            }
            let rows: Vec<(String, String, bool)> = p
                .matches
                .iter()
                .enumerate()
                .map(|(row_idx, &file_idx)| {
                    let path = p.workspace_files[file_idx].to_string_lossy().to_string();
                    (format!("@{path}"), String::new(), row_idx == p.index)
                })
                .collect();
            (
                "workspace files",
                rows,
                "↑↓ pick  •  Tab/Enter insert  •  Esc cancel",
            )
        }
    };

    // Popup dimensions: width = input width (or capped), height = up to 8 rows + 2 chrome.
    let max_rows: u16 = 8;
    let row_count = rows.len().min(max_rows as usize) as u16;
    let popup_h = row_count + 2; // borders
    let popup_w = input_area.width.min(80);

    // Anchor above the input box. If there's no room above (rare on small
    // terms), fall back to anchoring inside the top of the input.
    let popup_y = input_area.y.saturating_sub(popup_h);
    let popup_x = input_area.x;
    let area = Rect {
        x: popup_x,
        y: popup_y,
        width: popup_w,
        height: popup_h,
    };
    f.render_widget(Clear, area);

    let footer_title = Line::from(vec![
        Span::raw(" "),
        Span::styled(
            hint,
            Style::default()
                .fg(theme::color::FG_DIM)
                .add_modifier(Modifier::ITALIC),
        ),
        Span::raw(" "),
    ]);
    let block = theme::popup_block(title, false).title_bottom(footer_title);

    let max_name_w: usize = rows
        .iter()
        .map(|(n, _, _)| n.chars().count())
        .max()
        .unwrap_or(0)
        + 2;
    let items: Vec<ListItem> = rows
        .into_iter()
        .take(max_rows as usize)
        .map(|(name, desc, selected)| {
            let style = if selected {
                Style::default()
                    .fg(ratatui::style::Color::Black)
                    .bg(theme::color::ACCENT)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme::color::FG)
            };
            let mut spans = vec![Span::styled(
                format!(" {:<w$}", name, w = max_name_w),
                style,
            )];
            if !desc.is_empty() {
                spans.push(Span::styled(
                    format!("{desc} "),
                    if selected {
                        Style::default()
                            .fg(ratatui::style::Color::Black)
                            .bg(theme::color::ACCENT)
                    } else {
                        Style::default().fg(theme::color::FG_DIM)
                    },
                ));
            }
            ListItem::new(Line::from(spans))
        })
        .collect();

    let list = List::new(items).block(block);
    f.render_widget(list, area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
