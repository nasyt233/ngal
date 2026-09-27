use std::env;
use std::path::PathBuf;

pub enum Command {
    Run(PathBuf),
    New(Option<PathBuf>),
    Status(Option<PathBuf>),
    Build { dir: PathBuf, output: Option<PathBuf> },
    RunPacked(PathBuf),
    Edit(Option<PathBuf>),
    Help,
    Version,
}

pub struct Args {
    pub command: Command,
}

impl Args {
    pub fn parse() -> Self {
        let args: Vec<String> = env::args().collect();
        if args.len() == 1 {
            return Args {
                command: Command::Run(PathBuf::from(".")),
            };
        }

        match args[1].as_str() {
            "-h" | "--help" => Args { command: Command::Help },
            "-V" | "--version" => Args { command: Command::Version },
            "new" => {
                let dir = if args.len() > 2 {
                    Some(expand_path(&args[2]))
                } else {
                    None
                };
                Args { command: Command::New(dir) }
            }
            "status" => {
                let dir = if args.len() > 2 {
                    Some(expand_path(&args[2]))
                } else {
                    None
                };
                Args { command: Command::Status(dir) }
            }
            "build" => {
                let mut dir = PathBuf::from(".");
                let mut output = None;

                if args.len() > 2 {
                    let arg1 = expand_path(&args[2]);
                    if arg1.is_dir() {
                        dir = arg1;
                        if args.len() > 3 {
                            output = Some(PathBuf::from(&args[3]));
                        }
                    } else {
                        output = Some(PathBuf::from(&args[2]));
                    }
                }

                Args { command: Command::Build { dir, output } }
            }
            "edit" => {
                let path = if args.len() > 2 {
                    Some(expand_path(&args[2]))
                } else {
                    None
                };
                Args { command: Command::Edit(path) }
            }
            other => {
                let path = expand_path(other);
                if other.ends_with(".ngal") {
                    Args { command: Command::RunPacked(path) }
                } else {
                    Args { command: Command::Run(path) }
                }
            }
        }
    }

    pub fn print_help() {
        println!("ngal - 终端视觉小说引擎");
        println!();
        println!("用法:");
        println!("  ngal                    在当前目录运行游戏");
        println!("  ngal <目录>             在指定目录运行游戏");
        println!("  ngal <文件.ngal>        运行打包好的游戏");
        println!("  ngal new [目录]         创建新项目（默认当前目录）");
        println!("  ngal status [目录]      查看项目资源状态");
        println!("  ngal build [目录] [名]  打包游戏为 .ngal 文件");
        println!("  ngal edit [目录/文件]   图形化编辑剧情文件");
        println!("  ngal -h | --help        显示此帮助信息");
        println!("  ngal -V | --version     显示版本信息");
    }

    pub fn print_version() {
        println!("ngal version {}", env!("CARGO_PKG_VERSION"));
    }
}

pub fn expand_path(path: &str) -> PathBuf {
    if path.starts_with("~/") || path == "~" {
        let home = env::var("HOME").or_else(|_| env::var("USERPROFILE")).ok();
        if let Some(home) = home {
            let rest = if path == "~" { "" } else { &path[2..] };
            return PathBuf::from(home).join(rest);
        }
    }
    PathBuf::from(path)
}