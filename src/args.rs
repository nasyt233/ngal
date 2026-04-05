use std::env;
use std::path::PathBuf;

pub struct Args {
    pub help: bool,
    pub game_dir: PathBuf,
}

impl Args {
    pub fn parse() -> Self {
        let args: Vec<String> = env::args().collect();
        let mut help = false;
        let mut game_dir = PathBuf::from(".");

        for arg in &args[1..] {
            match arg.as_str() {
                "-h" | "--help" | "help" => {
                    help = true;
                }
                path => {
                    game_dir = PathBuf::from(path);
                }
            }
        }

        Args { help, game_dir }
    }

    pub fn print_help() {
        println!("ngal - 终端视觉小说引擎");
        println!();
        println!("用法:");
        println!("  ngal                    在当前目录运行游戏");
        println!("  ngal <目录>             在指定目录运行游戏");
        println!("  ngal -h | --help | help 显示此帮助信息");
        println!();
        println!("说明:");
        println!("  首次运行会在游戏目录下自动创建 assets/ 和 save/ 文件夹");
        println!("  游戏配置文件: assets/game.json");
        println!("  剧情文件: assets/dialog/dialogue.txt (由 game.json 中的 index 指定)");
        println!("  存档文件: save/slot1.json ~ save/slot10.json");
    }
}