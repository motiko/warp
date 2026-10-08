use warpui::keymap::{BindingDescription, PerPlatformKeystroke};

use super::StaticCommand;

pub enum DefaultSlashCommandBinding {
    None,
    Single(&'static str),
    PerPlatform(PerPlatformKeystroke),
}

pub fn default_binding_for_command(name: &'static str) -> DefaultSlashCommandBinding {
    match name {
        _ => DefaultSlashCommandBinding::None,
    }
}

pub fn binding_description(command: &StaticCommand) -> BindingDescription {
    BindingDescription::new_preserve_case(format!("Slash command: {}", command.name))
}
