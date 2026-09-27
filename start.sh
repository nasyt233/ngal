# 安装与启动程序
# 作者: NAS油条

check_pkg_install() {
    if [ -f /etc/os-release ]; then
        source /etc/os-release #加载变量
    fi
    if [[ -n $TERMUX_VERSION ]]; then
        PRETTY_NAME="Termux"
        pkg_install="pkg install"
        pkg_remove="pkg remove"
        yes_tg="-y" 
    elif command -v apt-get >/dev/null 2>&1; then
        pkg_install="apt install"
        pkg_remove="apt remove"
        sudo_setup="sudo"
        yes_tg="-y"
    elif command -v dnf >/dev/null 2>&1; then
        pkg_install="dnf install"
        pkg_remove="dnf remove"
        sudo_setup="sudo"
        yes_tg="-y"
    elif command -v yum >/dev/null 2>&1; then
        pkg_install="yum install"
        pkg_remove="yum remove"
        sudo_setup="sudo"
        yes_tg="-y"
    elif command -v pacman >/dev/null 2>&1; then
        pkg_install="pacman -S"
        pkg_remove="pacman -R"
        sudo_setup="sudo"
        yes_tg="-y"
    elif command -v zypper >/dev/null 2>&1; then
        pkg_install="zypper in -y"
        pkg_remove="zypper rm"
        sudo_setup="sudo"
        yes_tg="-y"
    elif command -v apk >/dev/null 2>&1; then
        sed -i 's#https\?://dl-cdn.alpinelinux.org/alpine#https://mirrors.tuna.tsinghua.edu.cn/alpine#g' /etc/apk/repositories
        pkg_install="apk add"
        pkg_remove="apk del"
        sudo_setup="sudo"
        yes_tg=""
    elif command -v emerge >/dev/null 2>&1; then
        pkg_install="emerge -avk"
        pkg_remove="emerge -C"
        sudo_setup="sudo"
        yes_tg="-y"
    elif command -v opkg >/dev/null 2>&1; then
        pkg_install="opkg install"
        pkg_remove="opkg remove"
        sudo_setup="sudo"
        yes_tg="-y"
    elif [[ "$(uname -s)" == "Darwin" ]]; then
        brew_install #brew安装检测
        pkg_install="brew install"
        sudo_setup="sudo"
        yes_tg="-y"
        read -p "抱歉，目前没有完全适配MacOS系统"
    else
        echo -e "$(info) >_<未检测到支持的系统。"
    fi
}

test_install() {
    if command -v $* >/dev/null 2>&1; then
        echo -e "$(info) $green $*已安装,跳过安装$color"
    else
        echo -e "$(info) 正在安装$*"
        if command -v eatmydata >/dev/null 2>&1; then
            eatmydata_setup=eatmydata
        fi
        $sudo_setup $eatmydata_setup $pkg_install $* $yes_tg
        install_error=$?
        if [ $install_error -ne 0 ]; then
            echo -e "$(info) $red $*安装失败。$color"
            echo -e "$(info) $red 错误代码$install_error $color"
            echo -e "$(info) 正在尝试更新软件包"
            $sudo_setup $pkg_update $yes_tg
            if [ $? -ne 0 ]; then
                echo -e "$(info) $red 更新软件包失败$color"
                esc
            else
                echo -e "$(info) $green 更新软件包成功,正在尝试重新安装。$color"
                $sudo_setup $eatmydata_setup $pkg_install $* $yes_tg
            fi
        else
            echo -e "$(info) $green $*安装成功。$color"
        fi
    fi
}

test_remove() {
    if command -v $* >/dev/null 2>&1; then
        $sudo_setup $pkg_remove $* $yes_tg
        remove_error=$?
        if [[ $remove_error -ne 0 ]]; then
            echo -e "$(info) $red $* 软件包卸载失败,按回车键继续。 $color";read
        fi
    else
        echo -e "$(info) $green 不存在这个软件包，无需卸载$color"
    fi
}

color_variable() {
    color='\033[0m'
    green='\033[0;32m'
    blue='\033[0;34m'
    red='\033[31m'
    yellow='\033[33m'
    grey='\e[37m'
    pink='\033[38;5;218m'
    cyan='\033[96m'
}
esc() {
    echo -e "$(info) 按$green回车键$color$blue返回$color,按$yellow Ctrl+C$color$red退出$color";read
}
br() {
    echo -e "\e[1;34m----------------------------\e[0m"
}
index() {
    PATH=$PATH:$PWD
    if ! command -v ngal >/dev/null 2>&1; then
        br
        echo "正在安装ngal引擎(约6MB)"
        bash -c "$(curl -L https://raw.gitcode.com/nasyt/ngal/raw/main/install.sh)"
        [ $? -ne 0 ] && echo "运行程序安装失败，请手动下载"
    fi
    if ! command -v mpv >/dev/null 2>&1; then
        read -p "是否安装音频播放服务(y/n)" xz
        if [[ $xz == y ]]; then
            test_install mpv
        fi
    fi
    ngal .
}
main() {
    color_variable
    check_pkg_install
    index
}
main