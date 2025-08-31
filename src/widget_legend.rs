use crate::action::Config;
use crate::widget_diff_tree::DiffTreeWidget;

#[derive(Debug, Default)]
pub struct LegendWidget {
    label_show: String,
    label_hide: String,
    hide: bool,
}

impl LegendWidget {
    pub fn render(
        &self,
        frame: &mut mame::terminal::UnicodeTerminalFrame,
        config: &Config,
        tree: &DiffTreeWidget,
    ) -> std::fmt::Result {
        let legend = if self.hide {
            mame::legend::Legend::new(&self.label_show, std::iter::empty())
        } else {
            mame::legend::Legend::new(
                &self.label_hide,
                config
                    .current_keymap()
                    .bindings()
                    .filter(|b| b.action.as_ref().is_some_and(|a| a.is_applicable(tree)))
                    .filter_map(|b| b.label.as_ref())
                    .map(|s| format!(" {s}")),
            )
        };
        legend.render(frame)?;
        Ok(())
    }

    pub fn init(&mut self, label_show: String, label_hide: String, hide: bool) {
        self.label_show = label_show;
        self.label_hide = label_hide;
        self.hide = hide;
    }

    pub fn toggle_hide(&mut self) {
        self.hide = !self.hide;
    }
}
