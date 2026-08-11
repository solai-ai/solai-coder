//! Session headers, onboarding guidance, and transcript cards.

use super::*;

pub(crate) const SESSION_HEADER_MAX_INNER_WIDTH: usize = 56; // Just an eyeballed value
const SOLAI_LOGO_MIN_INNER_WIDTH: usize = 45;
const SOLAI_LOGO_LINES: [&str; 6] = [
    "███████╗ ██████╗ ██╗      █████╗ ██╗",
    "██╔════╝██╔═══██╗██║     ██╔══██╗██║",
    "███████╗██║   ██║██║     ███████║██║",
    "╚════██║██║   ██║██║     ██╔══██║██║",
    "███████║╚██████╔╝███████╗██║  ██║██║",
    "╚══════╝ ╚═════╝ ╚══════╝╚═╝  ╚═╝╚═╝",
];

pub(crate) fn card_inner_width(width: u16, max_inner_width: usize) -> Option<usize> {
    if width < 4 {
        return None;
    }
    let inner_width = std::cmp::min(width.saturating_sub(4) as usize, max_inner_width);
    Some(inner_width)
}

/// Render `lines` inside a border sized to the widest span in the content.
pub(crate) fn with_border(lines: Vec<Line<'static>>) -> Vec<Line<'static>> {
    with_border_internal(lines, /*forced_inner_width*/ None)
}

/// Render `lines` inside a border whose inner width is at least `inner_width`.
///
/// This is useful when callers have already clamped their content to a
/// specific width and want the border math centralized here instead of
/// duplicating padding logic in the TUI widgets themselves.
pub(crate) fn with_border_with_inner_width(
    lines: Vec<Line<'static>>,
    inner_width: usize,
) -> Vec<Line<'static>> {
    with_border_internal(lines, Some(inner_width))
}

fn with_border_internal(
    lines: Vec<Line<'static>>,
    forced_inner_width: Option<usize>,
) -> Vec<Line<'static>> {
    let max_line_width = lines
        .iter()
        .map(|line| {
            line.iter()
                .map(|span| UnicodeWidthStr::width(span.content.as_ref()))
                .sum::<usize>()
        })
        .max()
        .unwrap_or(0);
    let content_width = forced_inner_width
        .unwrap_or(max_line_width)
        .max(max_line_width);

    let mut out = Vec::with_capacity(lines.len() + 2);
    let border_inner_width = content_width + 2;
    out.push(logo_border_line(
        "╭",
        &"─".repeat(border_inner_width),
        "╮",
        /*index*/ 0,
    ));

    for line in lines.into_iter() {
        let used_width: usize = line
            .iter()
            .map(|span| UnicodeWidthStr::width(span.content.as_ref()))
            .sum();
        let span_count = line.spans.len();
        let mut spans: Vec<Span<'static>> = Vec::with_capacity(span_count + 4);
        let border_style = crate::style::solai_logo_style(out.len());
        spans.push(Span::styled("│ ", border_style));
        spans.extend(line);
        if used_width < content_width {
            spans.push(Span::from(" ".repeat(content_width - used_width)).dim());
        }
        spans.push(Span::styled(" │", border_style));
        out.push(Line::from(spans));
    }

    out.push(logo_border_line(
        "╰",
        &"─".repeat(border_inner_width),
        "╯",
        out.len(),
    ));

    out
}

fn logo_border_line(
    left: &'static str,
    middle: &str,
    right: &'static str,
    index: usize,
) -> Line<'static> {
    let style = crate::style::solai_logo_style(index);
    Line::from(vec![
        Span::styled(left, style),
        Span::styled(middle.to_string(), style),
        Span::styled(right, style),
    ])
}

/// Return the emoji followed by a hair space (U+200A).
/// Using only the hair space avoids excessive padding after the emoji while
/// still providing a small visual gap across terminals.
pub(crate) fn padded_emoji(emoji: &str) -> String {
    format!("{emoji}\u{200A}")
}

#[derive(Debug)]
struct SolaiLinksHistoryCell;

impl HistoryCell for SolaiLinksHistoryCell {
    fn display_lines(&self, width: u16) -> Vec<Line<'static>> {
        let indent = "  ";
        let indent_width = UnicodeWidthStr::width(indent);
        let wrap_width = usize::from(width.max(1))
            .saturating_sub(indent_width)
            .max(1);
        let mut lines: Vec<Line<'static>> = Vec::new();
        append_markdown(
            "**See more:** https://solai-ai.github.io\n**buy $SOLAI:** https://pump.fun/coin/Hy9XZ4Ae4oKtXYfuFzWkoNV18teCTpvWWu5PFD9Bpump\n$SOLAI is used to create SOLAI Nodes for inference rental, power payments across the SOLAI ecosystem, and more.",
            Some(wrap_width),
            None,
            &mut lines,
        );

        prefix_lines(lines, indent.into(), indent.into())
    }

    fn raw_lines(&self) -> Vec<Line<'static>> {
        vec![
            Line::from("See more: https://solai-ai.github.io"),
            Line::from(
                "buy $SOLAI: https://pump.fun/coin/Hy9XZ4Ae4oKtXYfuFzWkoNV18teCTpvWWu5PFD9Bpump",
            ),
            Line::from(
                "$SOLAI is used to create SOLAI Nodes for inference rental, power payments across the SOLAI ecosystem, and more.",
            ),
        ]
    }
}

#[derive(Debug)]
pub struct SessionInfoCell(CompositeHistoryCell);

impl HistoryCell for SessionInfoCell {
    fn display_lines(&self, width: u16) -> Vec<Line<'static>> {
        self.0.display_lines(width)
    }

    fn desired_height(&self, width: u16) -> u16 {
        self.0.desired_height(width)
    }

    fn transcript_lines(&self, width: u16) -> Vec<Line<'static>> {
        self.0.transcript_lines(width)
    }

    fn raw_lines(&self) -> Vec<Line<'static>> {
        self.0.raw_lines()
    }
}

pub(crate) fn new_session_info(
    config: &Config,
    requested_model: &str,
    session: &ThreadSessionState,
    is_first_event: bool,
    tooltip_override: Option<String>,
    auth_plan: Option<PlanType>,
    show_fast_status: bool,
) -> SessionInfoCell {
    // Header box rendered as history (so it appears at the very top)
    let header = SessionHeaderHistoryCell::new(
        session.model.clone(),
        session.reasoning_effort.clone(),
        show_fast_status,
        config.cwd.to_path_buf(),
        CODEX_CLI_VERSION,
    )
    .with_yolo_mode(has_yolo_permissions(
        session.approval_policy,
        &session.permission_profile,
    ));
    let mut parts: Vec<Box<dyn HistoryCell>> = vec![Box::new(header)];

    if is_first_event {
        // Help lines below the header (new copy and list)
        let help_lines: Vec<Line<'static>> = vec![
            "  To get started, describe a task or try one of these commands:"
                .dim()
                .into(),
            Line::from(""),
            Line::from(vec![
                "  ".into(),
                "/init".into(),
                " - create an AGENTS.md file with instructions for SolaiAgent".dim(),
            ]),
            Line::from(vec![
                "  ".into(),
                "/status".into(),
                " - show current session configuration".dim(),
            ]),
            Line::from(vec![
                "  ".into(),
                "/permissions".into(),
                " - choose what SolaiAgent is allowed to do".dim(),
            ]),
            Line::from(vec![
                "  ".into(),
                "/model".into(),
                " - choose what model and reasoning effort to use".dim(),
            ]),
            Line::from(vec![
                "  ".into(),
                "/review".into(),
                " - review any changes and find issues".dim(),
            ]),
        ];

        parts.push(Box::new(PlainHistoryCell { lines: help_lines }));
    } else {
        if config.show_tooltips
            && (tooltip_override.is_some()
                || tooltips::get_tooltip(auth_plan, show_fast_status).is_some())
        {
            parts.push(Box::new(SolaiLinksHistoryCell));
        }
        if requested_model != session.model.as_str() {
            let lines = vec![
                "model changed:".magenta().bold().into(),
                format!("requested: {requested_model}").into(),
                format!("used: {}", session.model).into(),
            ];
            parts.push(Box::new(PlainHistoryCell { lines }));
        }
    }

    SessionInfoCell(CompositeHistoryCell { parts })
}

pub(crate) fn is_yolo_mode(config: &Config) -> bool {
    has_yolo_permissions(
        AskForApproval::from(config.permissions.approval_policy.value()),
        &config.permissions.effective_permission_profile(),
    )
}

pub(crate) fn has_yolo_permissions(
    approval_policy: AskForApproval,
    permission_profile: &PermissionProfile,
) -> bool {
    approval_policy == AskForApproval::Never
        && matches!(
            permission_profile,
            PermissionProfile::Disabled
                | PermissionProfile::Managed {
                    file_system: ManagedFileSystemPermissions::Unrestricted,
                    network: NetworkSandboxPolicy::Enabled,
                }
        )
}
#[derive(Debug)]
pub(crate) struct SessionHeaderHistoryCell {
    version: &'static str,
    model: String,
    model_style: Style,
    reasoning_effort: Option<ReasoningEffortConfig>,
    show_fast_status: bool,
    directory: PathBuf,
    yolo_mode: bool,
}

impl SessionHeaderHistoryCell {
    pub(crate) fn new(
        model: String,
        reasoning_effort: Option<ReasoningEffortConfig>,
        show_fast_status: bool,
        directory: PathBuf,
        version: &'static str,
    ) -> Self {
        Self::new_with_style(
            model,
            Style::default(),
            reasoning_effort,
            show_fast_status,
            directory,
            version,
        )
    }

    pub(crate) fn new_with_style(
        model: String,
        model_style: Style,
        reasoning_effort: Option<ReasoningEffortConfig>,
        show_fast_status: bool,
        directory: PathBuf,
        version: &'static str,
    ) -> Self {
        Self {
            version,
            model,
            model_style,
            reasoning_effort,
            show_fast_status,
            directory,
            yolo_mode: false,
        }
    }

    pub(crate) fn with_yolo_mode(mut self, yolo_mode: bool) -> Self {
        self.yolo_mode = yolo_mode;
        self
    }

    fn format_directory(&self, max_width: Option<usize>) -> String {
        Self::format_directory_inner(&self.directory, max_width)
    }

    pub(crate) fn format_directory_inner(directory: &Path, max_width: Option<usize>) -> String {
        let formatted = if let Some(rel) = relativize_to_home(directory) {
            if rel.as_os_str().is_empty() {
                "~".to_string()
            } else {
                format!("~{}{}", std::path::MAIN_SEPARATOR, rel.display())
            }
        } else {
            directory.display().to_string()
        };

        if let Some(max_width) = max_width {
            if max_width == 0 {
                return String::new();
            }
            if UnicodeWidthStr::width(formatted.as_str()) > max_width {
                return crate::text_formatting::center_truncate_path(&formatted, max_width);
            }
        }

        formatted
    }

    fn reasoning_label(&self) -> Option<&str> {
        self.reasoning_effort
            .as_ref()
            .map(ReasoningEffortConfig::as_str)
    }

    fn model_value(&self) -> String {
        let mut value = self.model.clone();
        if let Some(reasoning) = self.reasoning_label() {
            value.push(' ');
            value.push_str(reasoning);
        }
        if self.show_fast_status {
            value.push_str(" fast");
        }
        value
    }

    fn metadata_row(
        label: &'static str,
        value: String,
        inner_width: usize,
        value_style: Style,
    ) -> Line<'static> {
        let label = format!("  {label:<10} ");
        let label_width = UnicodeWidthStr::width(label.as_str());
        let value_width = inner_width.saturating_sub(label_width);
        let value = truncate_text(&value, value_width);
        Line::from(vec![
            Span::from(label).dim(),
            Span::styled(value, value_style),
        ])
    }

    fn logo_lines(inner_width: usize) -> Vec<Line<'static>> {
        if inner_width >= SOLAI_LOGO_MIN_INNER_WIDTH {
            SOLAI_LOGO_LINES
                .iter()
                .enumerate()
                .map(|(index, line)| {
                    Line::from(vec![
                        Span::from("  "),
                        Span::styled((*line).to_string(), crate::style::solai_logo_style(index)),
                    ])
                })
                .collect()
        } else {
            vec![Line::from(vec![
                Span::from("  "),
                Span::styled("SOLAI", crate::style::solai_logo_style(/*index*/ 0).bold()),
            ])]
        }
    }
}

impl HistoryCell for SessionHeaderHistoryCell {
    fn display_lines(&self, width: u16) -> Vec<Line<'static>> {
        let Some(inner_width) = card_inner_width(width, SESSION_HEADER_MAX_INNER_WIDTH) else {
            return Vec::new();
        };

        let metadata_value_width = inner_width.saturating_sub(13);
        let dir = self.format_directory(Some(metadata_value_width));

        let mut lines = Vec::new();
        lines.push(Line::from(""));
        lines.extend(Self::logo_lines(inner_width));
        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::from("  Solai Coder").bold()]));
        lines.push(Line::from(vec![
            Span::from("  Code anywhere. Just keep building.").dim(),
        ]));
        lines.push(Line::from(""));
        lines.push(Self::metadata_row(
            "model:",
            self.model_value(),
            inner_width,
            self.model_style,
        ));
        lines.push(Self::metadata_row(
            "directory:",
            dir,
            inner_width,
            Style::default(),
        ));

        if self.yolo_mode {
            lines.push(Self::metadata_row(
                "permissions:",
                "YOLO mode".to_string(),
                inner_width,
                Style::default().magenta().bold(),
            ));
        }
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::from("  > ").dim(),
            Span::styled(
                "Ready to build.",
                crate::style::solai_logo_style(/*index*/ 2).bold(),
            ),
        ]));
        lines.push(Line::from(""));

        with_border_with_inner_width(lines, inner_width)
    }

    fn raw_lines(&self) -> Vec<Line<'static>> {
        let mut lines = vec![
            Line::from(format!("Solai Coder (v{})", self.version)),
            Line::from(format!("model: {}", self.model_value())),
            Line::from(format!(
                "directory: {}",
                self.format_directory(/*max_width*/ None)
            )),
        ];
        if self.yolo_mode {
            lines.push(Line::from("permissions: YOLO mode"));
        }
        lines
    }
}
