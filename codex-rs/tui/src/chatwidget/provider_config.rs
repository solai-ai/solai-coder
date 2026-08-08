use super::*;

impl ChatWidget {
    pub(super) fn show_provider_config_prompt(&mut self) {
        let tx = self.app_event_tx.clone();
        let view = CustomPromptView::new(
            "Configure provider".to_string(),
            "Enter ip:port, for example 127.0.0.1:11434".to_string(),
            String::new(),
            /*context_label*/ None,
            Box::new(move |address: String| {
                tx.send(AppEvent::ConfigureProvider { address });
            }),
        );

        self.bottom_pane.show_view(Box::new(view));
    }
}
