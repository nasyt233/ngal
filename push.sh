#!/bin/sh
set -e

VERSION="$1"
if [ -z "$VERSION" ]; then
    echo "用法: $0 <版本号>"
    exit 1
fi

echo "正在更新lock"
# 更新 lockfile
cargo generate-lockfile >/dev/null 2>&1 || cargo check >/dev/null 2>&1

# 提交
echo "正在提交仓库"
git add .
git commit -m "release: $VERSION"
git push origin main

echo "正在设置标签"
# 打 tag
git tag "ngal-$VERSION"
git push origin "ngal-$VERSION"

echo "已推送 tag ngal-$VERSION"