# ─────────────────────────────────────────────
#  Cache Manager — 开发常用命令
#  需要先安装 just: https://github.com/casey/just
#  安装:  Windows scoop/choco/cargo · macOS brew install just · Linux apt/cargo
#  跨平台: macOS / Linux / Windows 均可执行（release 依赖 sh）
#
#  若 Windows 报错 "could not find the shell sh":
#   1) 安装 Git for Windows (https://git-scm.com)
#   2) 确认 Git\bin 在 PATH（系统/用户环境变量）
#   3) 重开终端（PATH 变更需新终端才生效）
# ─────────────────────────────────────────────

# 显式指定 shell：Unix 原生 sh；Windows 依赖 Git Bash 提供的 sh.exe
set shell := ["sh", "-cu"]

# 显示可用命令
default:
    @just --list

# 安装前端依赖
install:
    npm install

# 调试运行（开发模式，热重载）
dev:
    npm run tauri dev

# 发布构建（免安装单文件 exe，产物在 src-tauri/target/release/cache-manager.exe）
build:
    npm run tauri build

# 发布：exe 加版本号 + 源码打包 source-<版本号>.tar.gz + 生成 .md5/.sha1 hash + git tag v<版本号>
# （跨平台 POSIX sh 脚本：macOS / Linux / Windows Git Bash）
release:
    sh scripts/release.sh

# 前端单独构建（vite 预构建，tauri build 内部也会执行）
build-fe:
    npm run build

# 前端类型检查（vue-tsc）
typecheck:
    npx vue-tsc --noEmit

# 后端单元测试（含 URL 构建等）
test:
    cargo test

# 后端冒烟测试（需本地 Redis 127.0.0.1:6379）
test-smoke:
    cargo test --test smoke

# 后端主从模式测试（master:6380, replica:6381）
test-master-slave:
    cargo test --test master_slave

# 后端哨兵模式测试（sentinel:26379, master:6380）
test-sentinel:
    cargo test --test sentinel

# 后端集群模式测试（3 节点 7000-7002）
test-cluster:
    cargo test --test cluster

# 从 logo.png 重新生成全套应用图标（src-tauri/icons）
icon:
    npx tauri icon logo.png
