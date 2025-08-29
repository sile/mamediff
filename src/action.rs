#[derive(Debug, Clone)]
pub enum Action {
    Quit,
    Recenter,
    MoveUp,
    MoveDown,
    MoveLeft,
    MoveRight,
    ToggleExpand,
    Stage,
    Discard,
    Unstage,
    ToggleLegend,
    InitLegend {
        hide: bool,
        label_show: String,
        label_hide: String,
    },
}

impl mame::action::Action for Action {}

impl<'text, 'raw> TryFrom<nojson::RawJsonValue<'text, 'raw>> for Action {
    type Error = nojson::JsonParseError;

    fn try_from(value: nojson::RawJsonValue<'text, 'raw>) -> Result<Self, Self::Error> {
        let ty = value.to_member("type")?.required()?;

        match ty.to_unquoted_string_str()?.as_ref() {
            "quit" => Ok(Self::Quit),
            "recenter" => Ok(Self::Recenter),
            "move-up" => Ok(Self::MoveUp),
            "move-down" => Ok(Self::MoveDown),
            "move-left" => Ok(Self::MoveLeft),
            "move-right" => Ok(Self::MoveRight),
            "toggle" => Ok(Self::ToggleExpand),
            "stage" => Ok(Self::Stage),
            "discard" => Ok(Self::Discard),
            "unstage" => Ok(Self::Unstage),
            "toggle-legend" => Ok(Self::ToggleLegend),
            "init-legend" => {
                let hide = value
                    .to_member("hide")?
                    .map(bool::try_from)?
                    .unwrap_or_default();
                let labels = value.to_member("labels")?.required()?;
                let label_show = labels.to_member("show")?.required()?.try_into()?;
                let label_hide = labels.to_member("hide")?.required()?.try_into()?;

                Ok(Self::InitLegend {
                    hide,
                    label_show,
                    label_hide,
                })
            }
            ty => Err(value.invalid(format!("unknown action type: {ty:?}"))),
        }
    }
}
