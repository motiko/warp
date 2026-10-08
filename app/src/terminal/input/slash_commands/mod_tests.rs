use crate::search::slash_command_menu::static_commands::{SlashCommandKind, commands};

#[test]
fn commands_have_typed_identities() {
    for (command, expected) in [
        (&*commands::EDIT, SlashCommandKind::Edit),
        (&*commands::RENAME_TAB, SlashCommandKind::RenameTab),
        (&*commands::SET_TAB_COLOR, SlashCommandKind::SetTabColor),
        (&commands::OPEN_CODE_REVIEW, SlashCommandKind::OpenCodeReview),
        (&commands::OPEN_SETTINGS_FILE, SlashCommandKind::OpenSettingsFile),
        (&commands::CHANGELOG, SlashCommandKind::Changelog),
        (&*commands::FEEDBACK, SlashCommandKind::Feedback),
    ] {
        assert_eq!(
            command.kind, expected,
            "{} should have its typed command identity",
            command.name
        );
    }
}

#[cfg(all(feature = "local_fs", windows))]
mod windows {
    use std::sync::Arc;

    use super::super::*;
    use crate::terminal::ShellLaunchData;
    use crate::terminal::model::session::SessionInfo;
    use crate::terminal::model::session::command_executor::testing::TestCommandExecutor;
    use crate::terminal::shell::ShellType;

    fn wsl_session() -> Session {
        Session::new(
            SessionInfo::new_for_test()
                .with_shell_type(ShellType::Bash)
                .with_home_dir("/home/ubuntu".to_owned()),
            Arc::new(TestCommandExecutor::default()),
        )
        .with_shell_launch_data(ShellLaunchData::WSL {
            distro: "Ubuntu".to_owned(),
        })
    }

    #[test]
    fn open_file_command_converts_wsl_paths_to_host_paths() {
        let session = wsl_session();
        let cases = [
            (
                "/home/ubuntu",
                "subdir/test.txt",
                r"\\WSL$\Ubuntu\home\ubuntu\subdir\test.txt",
                None,
            ),
            (
                "/home/ubuntu/project",
                "../test.txt",
                r"\\WSL$\Ubuntu\home\ubuntu\test.txt",
                None,
            ),
            (
                "/home/ubuntu",
                "subdir/file\\ name.txt",
                r"\\WSL$\Ubuntu\home\ubuntu\subdir\file name.txt",
                None,
            ),
            (
                "/home/ubuntu",
                "subdir/test.txt:4:2",
                r"\\WSL$\Ubuntu\home\ubuntu\subdir\test.txt",
                Some(LineAndColumnArg {
                    line_num: 4,
                    column_num: Some(2),
                }),
            ),
            (
                "/tmp",
                "~/subdir/file\\ name.txt:4:2",
                r"\\WSL$\Ubuntu\home\ubuntu\subdir\file name.txt",
                Some(LineAndColumnArg {
                    line_num: 4,
                    column_num: Some(2),
                }),
            ),
        ];

        for (current_dir, raw_arg, expected_path, expected_line_col) in cases {
            let (path, line_col) = open_file_command_path(&session, current_dir, raw_arg);

            assert_eq!(path, PathBuf::from(expected_path));
            assert_eq!(line_col, expected_line_col);
        }
    }
}
