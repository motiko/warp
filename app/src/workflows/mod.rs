use serde::{Deserialize, Serialize};
use pathfinder_color::ColorU;
use warp_core::ui::color::ContrastingColor;
use warp_core::ui::color::contrast::MinimumAllowedContrast;
use warp_core::ui::theme::Fill;
use warpui::AppContext;

pub mod arguments;
pub mod categories;
pub mod command_parser;
pub mod info_box;
pub mod local_workflows;
pub mod workflow;

pub use categories::{CategoriesView, CategoriesViewEvent, WorkflowsViewAction};
use workflow::Workflow;

use crate::appearance::Appearance;

pub fn init(app: &mut AppContext) {
    categories::init(app);
}

#[derive(Copy, Clone, Debug, Deserialize, Serialize, Eq, PartialEq, Hash)]
pub enum WorkflowSource {
    Global,
    Local,
    Project,
    WarpAI,
    /// A command block in a markdown file.
    Notebook,

    /// A hardcoded workflow type that allows Warp to surface features as Workflows (e.g.
    /// a command to see our network log)
    App,
}

#[derive(Copy, Clone, Debug, Deserialize, Serialize, Eq, PartialEq, Hash, PartialOrd)]
pub enum WorkflowSelectionSource {
    CommandPalette,
    UniversalSearch,
    Voltron,
    Notebook,
    SlashMenu,
    UpArrowHistory,
    Undefined,
}

/// Wrapper type for a workflow and where it came from.
#[derive(Clone, Debug, PartialEq)]
pub enum WorkflowType {
    /// Saved workflows sourced from local, global, project, app collections, saved locally.
    Local(Workflow),
    /// Ephemeral workflows generated from a natural-language query.
    AIGenerated(Workflow),
    /// A workflow that's part of a markdown file.
    Notebook(Workflow),
}

impl WorkflowType {
    pub fn as_workflow(&self) -> &Workflow {
        match self {
            WorkflowType::Local(workflow) => workflow,
            WorkflowType::AIGenerated(workflow) => workflow,
            WorkflowType::Notebook(workflow) => workflow,
        }
    }

    /// Returns the contained [`Workflow`], consuming `self`.
    pub fn take_workflow(self) -> Workflow {
        match self {
            WorkflowType::Local(workflow) => workflow,
            WorkflowType::AIGenerated(workflow) => workflow,
            WorkflowType::Notebook(workflow) => workflow,
        }
    }
}

/// The accent color for workflow icons.
pub fn workflow_icon_color(appearance: &Appearance) -> ColorU {
    let color: Fill = appearance.theme().terminal_colors().normal.red.into();
    color
        .on_background(
            appearance.theme().surface_1(),
            MinimumAllowedContrast::NonText,
        )
        .into()
}
