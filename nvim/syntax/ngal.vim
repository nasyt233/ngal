" ngal引擎剧情脚本 语法高亮 *.ng
if exists("b:current_syntax")
  finish
endif

" 注释 //
syntax match ngalComment "//.*"
highlight ngalComment guifg=#555555

" 指令：行首可选空格 + 关键字 music bg img load choose end input
syntax match ngalKeyword /^\s*\(music\|bg\|img\|load\|choose\|end\|input\)\>/
highlight ngalKeyword guifg=#4298e8

" 标签 [xxx]
syntax match ngalTag /\[[^]]*\]/
highlight ngalTag guifg=#e85555

" 变量 {xxx}
syntax match ngalVariable /{[^}]*}/
highlight ngalVariable guifg=#f5a623

" 百分比数字 100%
syntax match ngalNumber /\d\+%/
highlight ngalNumber guifg=#b5cea8

" 文件 xxx.xxx
syntax match ngalFile /\w\+\.\w\+/
highlight ngalFile guifg=#63c76a

" 角色名 中文+冒号 小明:
syntax match ngalChar /\v[\u4e00-\u9fa5]\+:/
highlight ngalChar guifg=#c277d8

" 分隔符 : |
syntax match ngalSep /[:|]/
highlight ngalSep guifg=#5fc8d8

" 冒号之后对话正文
syntax match ngalDialog /:\zs.*/
highlight ngalDialog guifg=#dcdcdc

let b:current_syntax = "ngal"
