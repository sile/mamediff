use mame::action::{BindingConfig, BindingContextName};
use orfail::OrFail;
use tuinix::{Terminal, TerminalEvent};

use crate::{
    action::Action, canvas::Canvas, widget_diff_tree::DiffTreeWidget, widget_legend::LegendWidget,
};

#[derive(Debug)]
pub struct App {
    terminal: Terminal,
    config: BindingConfig<Action>,
    context: BindingContextName,
    current_binding_index: Option<usize>,
    exit: bool,
    frame_row_start: usize,
    tree: DiffTreeWidget,
    legend: LegendWidget,
    preview: Option<mame::preview::TextPreview>,
}

impl App {
    pub fn new(config: BindingConfig<Action>) -> orfail::Result<Self> {
        let terminal = Terminal::new().or_fail()?;
        let tree = DiffTreeWidget::new(terminal.size()).or_fail()?;
        Ok(Self {
            terminal,
            context: config.initial_context().clone(),
            config,
            current_binding_index: None,
            exit: false,
            frame_row_start: 0,
            tree,
            legend: LegendWidget::default(),
            preview: None,
        })
    }

    pub fn run(mut self) -> orfail::Result<()> {
        if let Some(action) = self.config.setup_action().cloned() {
            self.handle_action(action).or_fail()?;
        }
        self.render().or_fail()?;

        while !self.exit {
            let Some(event) = self.terminal.poll_event(&[], &[], None).or_fail()? else {
                continue;
            };
            self.handle_event(event).or_fail()?;
        }

        Ok(())
    }

    fn render(&mut self) -> orfail::Result<()> {
        if self.terminal.size().is_empty() {
            return Ok(());
        }

        let mut canvas = Canvas::new(self.frame_row_start, self.terminal.size());
        self.tree.render(&mut canvas);

        let mut frame = canvas.into_frame();
        if let Some(preview) = &mut self.preview {
            preview.render(&mut frame).or_fail()?;
        }
        if let Some(bindings) = self.config.get_bindings(&self.context) {
            self.legend
                .render(&mut frame, bindings, self.current_binding_index, &self.tree)
                .or_fail()?;
        }

        self.terminal.draw(frame).or_fail()?;

        Ok(())
    }

    fn handle_event(&mut self, event: TerminalEvent) -> orfail::Result<()> {
        match event {
            TerminalEvent::Resize(size) => {
                let cursor_row = self.tree.cursor_row();
                let rows = size.rows;
                self.frame_row_start = cursor_row.saturating_sub(rows / 2);
                self.render().or_fail()
            }
            TerminalEvent::Input(input) => {
                let bindings = self.config.get_bindings(&self.context).or_fail()?;
                if let Some((index, binding)) =
                    bindings.iter().enumerate().find(|(_, b)| b.matches(input))
                {
                    let next_context = binding.context.clone();
                    let action = binding.action.clone();

                    if let Some(action) = action {
                        if self.legend.highlight_active_binding {
                            self.current_binding_index = Some(index);
                            self.render().or_fail()?;
                            self.current_binding_index = None;
                        }
                        self.handle_action(action).or_fail()?;
                    }

                    if let Some(context) = next_context {
                        self.context = context;
                    }
                    self.render().or_fail()?;
                }
                Ok(())
            }
            _ => Err(orfail::Failure::new(format!("unexpected event: {event:?}"))),
        }
    }

    fn handle_action(&mut self, action: Action) -> orfail::Result<()> {
        match action {
            Action::Quit => {
                self.exit = true;
            }
            Action::Recenter => {
                self.recenter();
            }
            Action::MoveUp => {
                if self.tree.cursor_up().or_fail()? {
                    self.scroll_if_need();
                }
            }
            Action::MoveDown => {
                if self.tree.cursor_down().or_fail()? {
                    self.scroll_if_need();
                }
            }
            Action::MoveLeft => {
                if self.tree.cursor_left() {
                    self.scroll_if_need();
                }
            }
            Action::MoveRight => {
                if self.tree.cursor_right().or_fail()? {
                    self.scroll_if_need();
                }
            }
            Action::ToggleExpand => {
                self.tree.toggle().or_fail()?;
            }
            Action::Stage => {
                if self.tree.stage().or_fail()? {
                    self.scroll_if_need();
                }
            }
            Action::Discard => {
                if self.tree.discard().or_fail()? {
                    self.scroll_if_need();
                }
            }
            Action::Unstage => {
                if self.tree.unstage().or_fail()? {
                    self.scroll_if_need();
                }
            }
            Action::ToggleLegend => {
                self.legend.toggle_hide();
            }
            Action::InitLegend {
                hide,
                label_show,
                label_hide,
                highlight_active_binding,
            } => {
                self.legend.label_show = label_show;
                self.legend.label_hide = label_hide;
                self.legend.hide = hide;
                self.legend.highlight_active_binding = highlight_active_binding;
            }
            Action::ExecuteCommand(a) => {
                self.execute_command(&a).or_fail()?;
            }
        }
        Ok(())
    }

    fn execute_command(&mut self, command: &mame::command::ExternalCommand) -> orfail::Result<()> {
        let executing_pane = mame::preview::TextPreviewPane::new(
            "executing",
            &format!("$ {}", command.command_line()),
        );
        self.preview = Some(mame::preview::TextPreview::new(Some(executing_pane), None));
        self.render().or_fail()?;

        let output = command.execute().or_fail()?;

        if output.status.success() {
            self.tree.reload().or_fail()?;
        }

        let stdout_pane =
            mame::preview::TextPreviewPane::new("stdout", &String::from_utf8_lossy(&output.stdout));
        let stderr_pane =
            mame::preview::TextPreviewPane::new("stderr", &String::from_utf8_lossy(&output.stderr));
        self.preview = Some(mame::preview::TextPreview::new(
            Some(stdout_pane),
            Some(stderr_pane),
        ));
        Ok(())
    }

    fn scroll_if_need(&mut self) {
        let cursor_row = self.tree.cursor_row();
        let terminal_rows = self.terminal.size().rows;
        let frame_row_end = self.frame_row_start + terminal_rows;

        if !(self.frame_row_start..frame_row_end).contains(&cursor_row) {
            self.frame_row_start = cursor_row.saturating_sub(terminal_rows / 2);
        }
    }

    fn recenter(&mut self) {
        if self.terminal.size().is_empty() {
            return;
        }

        let current = self.frame_row_start;
        let cursor_row = self.tree.cursor_row();
        let top = cursor_row;
        let bottom = cursor_row.saturating_sub(self.terminal.size().rows - 1);
        let center = cursor_row.saturating_sub(self.terminal.size().rows / 2);
        self.frame_row_start = if current != center && current != top {
            center
        } else if current == center {
            top
        } else {
            bottom
        };
    }
}
