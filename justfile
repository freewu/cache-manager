# ─────────────────────────────────────────────
#  Cache Manager — 开发常用命令
#  需要先安装 just: https://github.com/casey/just
#  Windows:  scoop install just / choco install just / cargo install just
#  WSL:      apt install just（npm 相关命令建议在 Windows 侧 cmd 运行）
# ─────────────────────────────────────────────

set shell := ["cmd.exe", "/c"]

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

# 发布：exe 加版本号 + 源码打包 source-<版本号>.tar.gz + 生成 .md5/.sha1 hash
release:
    powershell -NoProfile -ExecutionPolicy Bypass -File scripts/release.ps1

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
