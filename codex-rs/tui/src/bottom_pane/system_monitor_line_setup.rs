//! Configuration view for the monitor-backed second status line.

use std::collections::HashSet;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use strum::IntoEnumIterator;

use crate::app_event::AppEvent;
use crate::app_event_sender::AppEventSender;
use crate::bottom_pane::CancellationEvent;
use crate::bottom_pane::bottom_pane_view::BottomPaneView;
use crate::bottom_pane::multi_select_picker::MultiSelectItem;
use crate::bottom_pane::multi_select_picker::MultiSelectPicker;
use crate::keymap::ListKeymap;
use crate::render::renderable::Renderable;
use crate::system_monitor::MonitorLineItem;

pub(crate) struct SystemMonitorLineSetupView {
    picker: MultiSelectPicker,
}

impl SystemMonitorLineSetupView {
    pub(crate) fn new(
        status_line_items: Option<&[String]>,
        use_theme_colors: bool,
        app_event_tx: AppEventSender,
        list_keymap: ListKeymap,
    ) -> Self {
        let mut used_ids = HashSet::new();
        let mut items = vec![MultiSelectItem {
            id: "status-line-2-use-theme-colors".to_string(),
            name: "Use theme colors".to_string(),
            description: Some("Apply semantic colors to the monitor line".to_string()),
            enabled: use_theme_colors,
            orderable: false,
            section_break_after: true,
        }];

        if let Some(selected_items) = status_line_items.as_ref() {
            for id in *selected_items {
                let Ok(item) = id.parse::<MonitorLineItem>() else {
                    continue;
                };
                let item_id = item.to_string();
                if !used_ids.insert(item_id.clone()) {
                    continue;
                }
                items.push(Self::monitor_item(item, true));
            }
        }

        for item in MonitorLineItem::iter() {
            let item_id = item.to_string();
            if used_ids.contains(&item_id) {
                continue;
            }
            items.push(Self::monitor_item(item, false));
        }

        Self {
            picker: MultiSelectPicker::builder(
                "Configure Monitor Status Line".to_string(),
                Some(
                    "Select which monitor items to display in the second status line.".to_string(),
                ),
                app_event_tx,
            )
            .list_keymap(list_keymap)
            .items(items)
            .enable_ordering()
            .on_confirm(|ids, app_event| {
                let use_theme_colors = ids.iter().any(|id| id == "status-line-2-use-theme-colors");
                let items = ids
                    .iter()
                    .filter_map(|id| id.parse::<MonitorLineItem>().ok())
                    .map(|item| item.to_string())
                    .collect::<Vec<_>>();
                app_event.send(AppEvent::StatusLine2Setup {
                    items,
                    use_theme_colors,
                });
            })
            .on_cancel(|app_event| {
                app_event.send(AppEvent::StatusLine2SetupCancelled);
            })
            .build(),
        }
    }

    fn monitor_item(item: MonitorLineItem, enabled: bool) -> MultiSelectItem {
        MultiSelectItem {
            id: item.to_string(),
            name: monitor_item_name(item).to_string(),
            description: Some(item.description().to_string()),
            enabled,
            orderable: true,
            section_break_after: false,
        }
    }
}

impl BottomPaneView for SystemMonitorLineSetupView {
    fn handle_key_event(&mut self, key_event: crossterm::event::KeyEvent) {
        self.picker.handle_key_event(key_event);
    }

    fn is_complete(&self) -> bool {
        self.picker.complete
    }

    fn on_ctrl_c(&mut self) -> CancellationEvent {
        self.picker.close();
        CancellationEvent::Handled
    }
}

impl Renderable for SystemMonitorLineSetupView {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        self.picker.render(area, buf)
    }

    fn desired_height(&self, width: u16) -> u16 {
        self.picker.desired_height(width)
    }
}

fn monitor_item_name(item: MonitorLineItem) -> &'static str {
    match item {
        MonitorLineItem::Cpu => "CPU",
        MonitorLineItem::Ram => "RAM",
        MonitorLineItem::Swap => "Swap",
        MonitorLineItem::Gpu => "GPU",
        MonitorLineItem::GpuModel => "GPU model",
        MonitorLineItem::Vram => "VRAM",
        MonitorLineItem::GpuTemp => "GPU temp",
        MonitorLineItem::Status => "Status",
        MonitorLineItem::Context => "Context size",
        MonitorLineItem::GpuCpu => "GPU x CPU",
    }
}
