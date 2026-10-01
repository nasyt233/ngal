use anyhow::Result;
use std::path::{Path, PathBuf};

use ngal::{args, commands, edit, runner};

fn main() -> Result<()> {
    let parsed_args = args::Args::parse();

    match parsed_args.command {
        args::Command::Help => {
            args::Args::print_help();
            Ok(())
        }
        args::Command::Version => {
            args::Args::print_version();
            Ok(())
        }
        args::Command::New(dir) => commands::new_project(dir),
        args::Command::Status(dir) => commands::show_status(dir),
        args::Command::Build { dir, output } => commands::build_project(dir, output),
        args::Command::RunPacked { file, debug } => commands::run_packed(&file, debug),
        args::Command::Update => commands::update(),
        args::Command::Edit(path) => {
            let file_path = match path {
                Some(p) => {
                    if p.is_dir() {
                        p.join("assets/dialog/dialogue.ng")
                    } else {
                        p
                    }
                }
                None => PathBuf::from("assets/dialog/dialogue.ng"),
            };
            let mut ed = edit::Editor::new(file_path)?;
            ed.run()
        }
        args::Command::Run(game_dir) => {
            if !game_dir.exists() {
                eprintln!("目录不存在: {}", game_dir.display());
                return Ok(());
            }
        
            let check_dir = if game_dir == Path::new(".") {
                std::env::current_dir()?
            } else {
                game_dir.clone()
            };
        
            if !check_dir.join("assets/game.json").exists() {
                return ngal::scanner::run_scanner(&check_dir);
            }
        
            // 确认是文件系统游戏 → 用 FS 模式
            ngal::assets::set_filesystem();
        
            if game_dir != Path::new(".") {
                std::env::set_current_dir(&game_dir)?;
            }
        
            let result = runner::run_game();
            ngal::assets::cleanup_cache();
            result
        }
    }
}