#!/bin/bash
myuname=$(uname | tr 'A-Z' 'a-z');arch=$(uname -m)
if [[ -n $PREFIX ]]; then arch=aarch64;else [[ $arch == "x86_64" ]] && arch="amd64"; [[ $arch == "aarch64" ]] && arch="arm64";fi
[[ -n $PREFIX ]] && bin=$PREFIX/bin || bin=/usr/bin
echo "正在获取最新版本信息";tag_name=$(curl -sL 'https://api.gitcode.com/api/v5/repos/nasyt/ngal/releases/latest' | grep -m1 -o '"tag_name":"[^"]*"' | cut -d'"' -f4)
dow_url="https://gitcode.com/nasyt/ngal/releases/download/$tag_name/$tag_name-$myuname-$arch"
echo "正在下载文件";curl --progress-bar -o ngal -L $dow_url;if [[ $? -ne 0 ]]; then echo "文件下载失败,错误代码 $?";exit 1;fi
chmod +x ngal;mv ngal $bin;echo "ngal安装完成";echo "输入ngal进入"