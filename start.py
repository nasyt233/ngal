#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# 安装与启动程序 / 作者: NAS油条
import os
import sys
import shutil
import subprocess
import platform

R  = '\033[0m'
G  = '\033[0;32m'
B  = '\033[0;34m'
RE = '\033[31m'
Y  = '\033[33m'
P  = '\033[38;5;218m'
C  = '\033[96m'

def info():
    return f"[{C}INFO{R}]"

def br():
    print(f"\033[1;34m----------------------------{R}")

def esc():
    print(f"{info()} 按{G}回车键{R}{B}返回{R},按{Y} Ctrl+C{R}{RE}退出{R}")
    try:
        input()
    except KeyboardInterrupt:
        sys.exit(0)

def run(cmd):
    return subprocess.run(cmd, shell=True,
                          stdout=subprocess.DEVNULL,
                          stderr=subprocess.DEVNULL).returncode

PKG_INSTALL = ''
PKG_REMOVE  = ''
PKG_UPDATE  = ''
SUDO        = ''
YES         = ''

def check_pkg_install():
    global PKG_INSTALL, PKG_REMOVE, PKG_UPDATE, SUDO, YES

    if os.environ.get('TERMUX_VERSION'):
        PKG_INSTALL = 'pkg install'
        PKG_REMOVE  = 'pkg remove'
        PKG_UPDATE  = 'pkg update'
        YES = '-y'
        return

    if shutil.which('apt-get'):
        PKG_INSTALL, PKG_REMOVE, PKG_UPDATE = 'apt install', 'apt remove', 'apt update'
        SUDO, YES = 'sudo', '-y'
    elif shutil.which('dnf'):
        PKG_INSTALL, PKG_REMOVE, PKG_UPDATE = 'dnf install', 'dnf remove', 'dnf check-update'
        SUDO, YES = 'sudo', '-y'
    elif shutil.which('yum'):
        PKG_INSTALL, PKG_REMOVE, PKG_UPDATE = 'yum install', 'yum remove', 'yum check-update'
        SUDO, YES = 'sudo', '-y'
    elif shutil.which('pacman'):
        PKG_INSTALL, PKG_REMOVE, PKG_UPDATE = 'pacman -S', 'pacman -R', 'pacman -Sy'
        SUDO, YES = 'sudo', '-y'
    elif shutil.which('zypper'):
        PKG_INSTALL, PKG_REMOVE, PKG_UPDATE = 'zypper in -y', 'zypper rm', 'zypper refresh'
        SUDO, YES = 'sudo', '-y'
    elif shutil.which('apk'):

        try:
            f = '/etc/apk/repositories'
            if os.path.isfile(f):
                s = open(f).read()
                s = s.replace('dl-cdn.alpinelinux.org/alpine',
                              'mirrors.tuna.tsinghua.edu.cn/alpine')
                open(f, 'w').write(s)
        except Exception as e:
            print(f"{info()} 修改 Alpine 源失败: {e}")
        PKG_INSTALL, PKG_REMOVE, PKG_UPDATE = 'apk add', 'apk del', 'apk update'
        SUDO, YES = 'sudo', ''
    elif shutil.which('emerge'):
        PKG_INSTALL, PKG_REMOVE, PKG_UPDATE = 'emerge -avk', 'emerge -C', 'emerge --sync'
        SUDO, YES = 'sudo', '-y'
    elif shutil.which('opkg'):
        PKG_INSTALL, PKG_REMOVE, PKG_UPDATE = 'opkg install', 'opkg remove', 'opkg update'
        SUDO, YES = 'sudo', '-y'
    elif platform.system() == 'Darwin':
        if not shutil.which('brew'):
            print(f"{info()} 未检测到 brew，正在安装 Homebrew...")
            run('/bin/bash -c "$(curl -fsSL '
                'https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"')
        PKG_INSTALL, PKG_REMOVE, PKG_UPDATE = 'brew install', 'brew uninstall', 'brew update'
        SUDO, YES = 'sudo', '-y'
        print("抱歉，目前没有完全适配 MacOS 系统")
    else:
        print(f"{info()} >_<未检测到支持的系统。")
        sys.exit(1)

def test_install(*pkgs):
    for pkg in pkgs:
        if shutil.which(pkg):
            print(f"{info()} {G}{pkg}已安装,跳过安装{R}")
            continue

        print(f"{info()} 正在安装{pkg}")
        eat = 'eatmydata' if shutil.which('eatmydata') else ''
        cmd = ' '.join(filter(None, (SUDO, eat, PKG_INSTALL, pkg, YES)))

        rc = run(cmd)
        if rc != 0:
            print(f"{info()} {RE}{pkg}安装失败。{R}")
            print(f"{info()} {RE}错误代码 {rc}{R}")
            print(f"{info()} 正在尝试更新软件包")
            if run(' '.join(filter(None, (SUDO, PKG_UPDATE, YES)))) != 0:
                print(f"{info()} {RE}更新软件包失败{R}")
                esc()
            else:
                print(f"{info()} {G}更新软件包成功,正在尝试重新安装。{R}")
                run(cmd)
        else:
            print(f"{info()} {G}{pkg}安装成功。{R}")

def test_remove(*pkgs):
    for pkg in pkgs:
        if shutil.which(pkg):
            if run(' '.join(filter(None, (SUDO, PKG_REMOVE, pkg, YES)))) != 0:
                print(f"{info()} {RE}{pkg} 软件包卸载失败,按回车键继续。{R}")
                input()
        else:
            print(f"{info()} {G}不存在这个软件包，无需卸载{R}")

def index():
    os.environ['PATH'] = os.environ.get('PATH', '') + ':' + os.getcwd()
    if not shutil.which('ngal'):
        br()
        print("正在安装 ngal 引擎(约6MB)")
        if run('bash -c "$(curl -L '
               'https://raw.gitcode.com/nasyt/ngal/raw/main/install.sh)"') != 0:
            print("运行程序安装失败，请手动下载")

    if not shutil.which('mpv'):
        try:
            xz = input("是否安装音频播放服务(y/n): ").strip().lower()
        except EOFError:
            xz = 'n'
        if xz == 'y':
            test_install('mpv')

    os.execvp('ngal', ['ngal', '.'])

def main():
    try:
        check_pkg_install()
        index()
    except KeyboardInterrupt:
        print("\n已退出。")
        sys.exit(0)

if __name__ == '__main__':
    main()