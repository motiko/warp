mod legacy;
mod static_prompt_suggestions;

pub use legacy::{
    PassiveSuggestionsEvent as LegacyPassiveSuggestionsEvent,
    PassiveSuggestionsModel as LegacyPassiveSuggestionsModel,
};
use warpui::ModelHandle;

#[derive(Clone)]
pub struct PassiveSuggestionsModels {
    pub legacy: ModelHandle<LegacyPassiveSuggestionsModel>,
}
