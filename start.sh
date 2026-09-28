#!/bin/sh
# ngal 游戏启动脚本
# 作者: NAS油条

color='\033[0m'
green='\033[0;32m'
blue='\033[0;34m'
red='\033[31m'
yellow='\033[33m'
grey='\e[37m'
pink='\033[38;5;218m'
cyan='\033[96m'

info() {
    printf '%b' "${cyan}ngal${color} "
}

br() {
    printf '\033[1;34m----------------------------\033[0m\n'
}

# ---------- 系统检测与包管理器 ----------
detect_pkg() {
    if [ -f /etc/os-release ]; then
        . /etc/os-release
    fi

    sudo_setup=""
    eatmydata_setup=""
    pkg_update="true"

    if [ -n "$TERMUX_VERSION" ]; then
        PRETTY_NAME="Termux"
        pkg_install="pkg install"
        pkg_remove="pkg remove"
        pkg_update="pkg update"
        yes_tg="-y"
    elif command -v apt-get >/dev/null 2>&1; then
        pkg_install="apt install"
        pkg_remove="apt remove"
        pkg_update="apt update"
        sudo_setup="sudo"
        yes_tg="-y"
    elif command -v dnf >/dev/null 2>&1; then
        pkg_install="dnf install"
        pkg_remove="dnf remove"
        pkg_update="dnf check-update"
        sudo_setup="sudo"
        yes_tg="-y"
    elif command -v yum >/dev/null 2>&1; then
        pkg_install="yum install"
        pkg_remove="yum remove"
        pkg_update="yum check-update"
        sudo_setup="sudo"
        yes_tg="-y"
    elif command -v pacman >/dev/null 2>&1; then
        pkg_install="pacman -S"
        pkg_remove="pacman -R"
        pkg_update="pacman -Sy"
        sudo_setup="sudo"
        yes_tg="--noconfirm"
    elif command -v zypper >/dev/null 2>&1; then
        pkg_install="zypper in -y"
        pkg_remove="zypper rm"
        pkg_update="zypper refresh"
        sudo_setup="sudo"
        yes_tg="-y"
    elif command -v apk >/dev/null 2>&1; then
        pkg_install="apk add"
        pkg_remove="apk del"
        pkg_update="apk update"
        sudo_setup="sudo"
        yes_tg=""
    elif command -v emerge >/dev/null 2>&1; then
        pkg_install="emerge -avk"
        pkg_remove="emerge -C"
        pkg_update="emerge --sync"
        sudo_setup="sudo"
        yes_tg="-y"
    elif command -v opkg >/dev/null 2>&1; then
        pkg_install="opkg install"
        pkg_remove="opkg remove"
        pkg_update="opkg update"
        sudo_setup="sudo"
        yes_tg="-y"
    elif [ "$(uname -s)" = "Darwin" ]; then
        if command -v brew >/dev/null 2>&1; then
            pkg_install="brew install"
            pkg_remove="brew uninstall"
            pkg_update="brew update"
            yes_tg=""
        else
            printf '%b' "$(info)${red}未检测到 Homebrew，请先安装 https://brew.sh${color}\n"
            exit 1
        fi
    else
        printf '%b' "$(info)${red}未检测到支持的系统。${color}\n"
        exit 1
    fi
}

# ---------- 依赖安装 ----------
ensure_cmd() {
    cmd="$1"
    if command -v "$cmd" >/dev/null 2>&1; then
        printf '%b' "$(info)${green}${cmd} 已安装，跳过${color}\n"
        return 0
    fi

    printf '%b' "$(info)正在安装 ${cmd}\n"
    if command -v eatmydata >/dev/null 2>&1; then
        eatmydata_setup="eatmydata"
    fi

    $sudo_setup $eatmydata_setup $pkg_install "$cmd" $yes_tg
    err=$?
    if [ $err -ne 0 ]; then
        printf '%b' "$(info)${red}${cmd} 安装失败，错误代码 ${err}${color}\n"
        printf '%b' "$(info)正在尝试更新软件包源\n"
        $sudo_setup $pkg_update $yes_tg
        if [ $? -ne 0 ]; then
            printf '%b' "$(info)${red}更新软件包源失败${color}\n"
            return 1
        fi
        printf '%b' "$(info)${green}更新成功，重新安装 ${cmd}${color}\n"
        $sudo_setup $eatmydata_setup $pkg_install "$cmd" $yes_tg
    else
        printf '%b' "$(info)${green}${cmd} 安装成功${color}\n"
    fi
}

# ---------- 主流程 ----------
main() {
    br

    if ! command -v ngal >/dev/null 2>&1; then
        printf '正在安装 ngal 引擎（约 6MB）\n'
        curl -L https://raw.gitcode.com/nasyt/ngal/raw/main/install.sh | sh
        if [ $? -ne 0 ]; then
            printf '%b' "$(info)${red}运行程序安装失败，请手动下载${color}\n"
            exit 1
        fi
    fi

    if ! command -v mpv >/dev/null 2>&1; then
        printf '%b' "$(info)${yellow}未检测到 mpv（音频播放）${color}\n"
        printf '%b' "$(info)不安装也能正常游玩，仅无 BGM / 语音。\n"
        printf '%b' "$(info)是否现在安装？[y/N] "
        read xz
        if [ "$xz" = "y" ] || [ "$xz" = "Y" ]; then
            ensure_cmd mpv
        else
            printf '%b' "$(info)${grey}已跳过，游戏将静音运行。${color}\n"
        fi
    fi

    br
    printf '%b' "$(info)启动游戏...\n"
    ngal .
}

detect_pkg
main