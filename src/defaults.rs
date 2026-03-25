//! 默认配置和剧情文件

pub const DEFAULT_GAME_CONFIG: &str = r#"{
  "title": "原神 VS 鸣朝",
  "footer": "按回车继续 | q 返回主菜单 | H 历史 | A 自动播放",
  "index": "dialog/dialogue.txt"
}"#;

pub const DEFAULT_DIALOGUE: &str = r#"[welcome]
music:music.mp3
img:NAS油条.png
NAS油条:本项目由Rust语言开发\n按回车键继续:nas_intro.mp3
img:
NAS油条:哪个游戏牛逼?:gamenb.mp3
choose:原神牛逼:ysnb|鸣朝牛逼:mcnb|终末地牛逼:zmd

[ysnb]
img:鸣朝.png
鸣朝:鸣朝才牛逼😡:mcnb.mp3
鸣朝:原神不牛逼🤓:ys_no_nb.mp3
load:ytnb

[mcnb]
img:原神.png
原神:原神才牛逼🤓👍:ysnb.mp3
原神:鸣朝不牛逼😡:mc_no_nb.mp3
load:ytnb

[zmd]
img:终末地.png
终末地:我最牛逼
end

[ytnb]
img:我.png
我:😋他们产的片才牛逼😋:ysmcnb.mp3
NAS油条:游戏结束
end"#;