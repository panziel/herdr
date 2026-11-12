//! 浮动窗格（floating pane）相关的 API 参数与结果。

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::panes::{PaneDirection, PaneInfo};
use crate::cli::pane::bindings as binding;
use crate::cli::projection::{cli_params, Field, Input};

/// How `tab.floating.set` changes the floating layer of a tab.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema, Default,
)]
#[serde(rename_all = "snake_case")]
pub enum FloatingLayerMode {
    #[default]
    Toggle,
    Show,
    Hide,
}

/// Toggle, set or clear a per-pane floating flag.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema, Default,
)]
#[serde(rename_all = "snake_case")]
pub enum FloatingFlagMode {
    #[default]
    Toggle,
    On,
    Off,
}

fn tab_option() -> Input<Option<String>> {
    Input::value("tab", "TAB_ID", |slot, value| {
        *slot = Some(crate::cli::normalize_tab_id(value));
        Ok(())
    })
}

fn flag_mode() -> Field<FloatingFlagMode> {
    Field::new(FloatingFlagMode::Toggle)
        .input(
            Input::flag("toggle", |slot, _| {
                *slot = FloatingFlagMode::Toggle;
                Ok(())
            })
            .configure(|arg| arg.conflicts_with_all(["on", "off"])),
        )
        .input(
            Input::flag("on", |slot, _| {
                *slot = FloatingFlagMode::On;
                Ok(())
            })
            .configure(|arg| arg.conflicts_with("off")),
        )
        .input(Input::flag("off", |slot, _| {
            *slot = FloatingFlagMode::Off;
            Ok(())
        }))
}

cli_params! {
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema, Default)]
    pub struct TabFloatingParams => TabFloatingProjection {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub tab_id: Option<String> => Field::new(None).input(tab_option()),
        #[serde(default)]
        pub mode: FloatingLayerMode => Field::new(FloatingLayerMode::Toggle)
            .input(Input::flag("toggle", |slot, _| { *slot = FloatingLayerMode::Toggle; Ok(()) }).configure(|arg| arg.conflicts_with_all(["show", "hide"])))
            .input(Input::flag("show", |slot, _| { *slot = FloatingLayerMode::Show; Ok(()) }).configure(|arg| arg.conflicts_with("hide")))
            .input(Input::flag("hide", |slot, _| { *slot = FloatingLayerMode::Hide; Ok(()) })),
        /// Open a floating pane when showing a layer that has none.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        pub create_if_empty: bool => Field::new(false).input(binding::boolean("create", true)),
    }
}

cli_params! {
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
    pub struct PaneFloatingCreateParams => PaneFloatingCreateProjection {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub tab_id: Option<String> => Field::new(None).input(tab_option()),
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub cwd: Option<String> => Field::new(None).input(binding::optional_text("cwd", "PATH").path()),
        #[serde(default = "default_true")]
        pub focus: bool => Field::new(true).input(binding::boolean("focus", true)).input(binding::boolean("no-focus", false)),
        #[serde(default, skip_serializing_if = "HashMap::is_empty")]
        pub env: HashMap<String, String> => Field::new(HashMap::new()).input(binding::environment()),
    }
}

fn default_true() -> bool {
    true
}

cli_params! {
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema, Default)]
    pub struct PaneFloatingSetParams => PaneFloatingSetProjection {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub pane_id: Option<String> => binding::target(true),
        #[serde(default)]
        pub mode: FloatingFlagMode => flag_mode(),
    }
}

cli_params! {
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema, Default)]
    pub struct PanePinParams => PanePinProjection {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub pane_id: Option<String> => binding::target(true),
        #[serde(default)]
        pub mode: FloatingFlagMode => flag_mode(),
    }
}

cli_params! {
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
    pub struct PaneFloatingMoveParams => PaneFloatingMoveProjection {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub pane_id: Option<String> => binding::target(false),
        pub direction: PaneDirection => Field::new(PaneDirection::Left).input(binding::direction()),
        /// Distance in terminal cells. Defaults to a few cells.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub cells: Option<u16> => Field::new(None).input(cells()),
    }
}

fn cells() -> Input<Option<u16>> {
    Input::value("cells", "N", |slot, value| {
        let cells = crate::cli::parse_u32_flag("--cells", value)?;
        *slot = Some(u16::try_from(cells).unwrap_or(u16::MAX));
        Ok(())
    })
}

/// Absolute floating pane geometry in thousandths of the tab area.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct FloatingGeometry {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct PaneFloatingGeometryParams {
    pub pane_id: String,
    pub geometry: FloatingGeometry,
}

/// Floating state of a pane in `PaneInfo`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct PaneFloatingInfo {
    pub pinned: bool,
    /// Drawn on screen: the floating layer is shown or the pane is pinned.
    pub visible: bool,
    pub geometry: FloatingGeometry,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct TabFloatingResult {
    pub tab_id: String,
    pub changed: bool,
    pub visible: bool,
    pub focused_pane_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_pane: Option<Box<PaneInfo>>,
}
