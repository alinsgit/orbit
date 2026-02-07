pub mod apache;
pub mod cache;
pub mod composer;
pub mod config;
pub mod database;
pub mod download;
pub mod hosts;
pub mod logs;
pub mod mailpit;
pub mod mariadb;
pub mod nginx;
pub mod php_registry;
pub mod phpmyadmin;
pub mod process;
pub mod registry;
pub mod site_store;
pub mod sites;
pub mod ssl;
pub mod templates;
pub mod validation;
pub mod versions;

use std::process::Command;

/// Creates a Command that runs without a visible console window on Windows.
pub fn hidden_command(program: impl AsRef<std::ffi::OsStr>) -> Command {
    let mut cmd = Command::new(program);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}
