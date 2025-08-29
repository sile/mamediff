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
        tree: &DiffTreeWidget,
    ) -> std::fmt::Result {
        let title = if self.hide {
            &self.label_show
        } else {
            &self.label_hide
        };
        let items = if self.hide {
            &[][..]
        } else {
            &[
                Some("(q)uit [ESC,C-c]"),
                (tree.cursor_row() != 0).then_some("(r)ecenter [C-l]"),
                tree.can_cursor_up().then_some("(↑)      [k,C-p]"),
                tree.can_cursor_down().then_some("(↓)      [j,C-n]"),
                tree.can_cursor_left().then_some("(←)      [h,C-b]"),
                tree.can_cursor_right().then_some("(→)      [l,C-f]"),
                tree.can_toggle().then_some("(t)oggle   [TAB]"),
                tree.can_stage_or_discard().then_some("(s)tage         "),
                tree.can_stage_or_discard().then_some("(D)iscard       "),
                tree.can_unstage().then_some("(u)nstage       "),
            ][..]
        };
        let legend = mame::legend::Legend::new(title, items.iter().filter_map(|x| *x));
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
