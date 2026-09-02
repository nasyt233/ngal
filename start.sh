PATH=$PATH:$PWD
if ! command -v ngal >/dev/null 2>&1; then
    echo "正在安装ngal引擎"
    bash -c "$(curl -L https://raw.gitcode.com/nasyt/ngal/raw/main/install.sh)"
    [ $? -ne 0 ] && echo "运行程序安装失败"
fi
ngal .