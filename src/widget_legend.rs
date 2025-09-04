use mame::action::Binding;

use crate::action::Action;
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
        bindings: &[Binding<Action>],
        current_binding_index: Option<usize>,
        tree: &DiffTreeWidget,
    ) -> std::fmt::Result {
        let legend = if self.hide {
            mame::legend::Legend::new(&self.label_show, std::iter::empty())
        } else {
            mame::legend::Legend::new(
                &self.label_hide,
                bindings
                    .iter()
                    .enumerate()
                    .filter(|(_, b)| b.action.as_ref().is_some_and(|a| a.is_applicable(tree)))
                    .filter_map(|(i, b)| {
                        let label = b.label.as_ref()?;
                        let highlight = self.highlight_active && Some(i) == current_binding_index;
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
