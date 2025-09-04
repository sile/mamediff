use crate::action::ActionBindingSystem;
use crate::widget_diff_tree::DiffTreeWidget;

#[derive(Debug, Default)]
pub struct LegendWidget {
    pub label_show: String,
    pub label_hide: String,
    pub hide: bool,
    pub highlight_active: bool,
}

impl LegendWidget {
    pub fn render(
        &self,
        frame: &mut mame::terminal::UnicodeTerminalFrame,
        bindings: &ActionBindingSystem,
        tree: &DiffTreeWidget,
    ) -> std::fmt::Result {
        let legend = if self.hide {
            mame::legend::Legend::new(&self.label_show, std::iter::empty())
        } else {
            mame::legend::Legend::new(
                &self.label_hide,
                bindings
                    .current_bindings()
                    .iter()
                    .filter(|b| b.action.as_ref().is_some_and(|a| a.is_applicable(tree)))
                    .filter_map(|b| {
                        let label = b.label.as_ref()?;
                        let highlight = self.highlight_active && bindings.is_last_binding(b);
                        Some(if highlight {
                            let bold = tuinix::TerminalStyle::new().bold();
                            let reset = tuinix::TerminalStyle::RESET;
                            format!(" {bold}{label}{reset}")
                        } else {
                            format!(" {label}")
                        })
                    }),
            )
        };
        legend.render(frame)?;
        Ok(())
    }

    pub fn toggle_hide(&mut self) {
        self.hide = !self.hide;
    }
}
