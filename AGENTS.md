# 项目开发规则

## 自动提交 Git

- 每完成一次用户要求的操作（功能实现、修复、文档更新等）后，**自动执行 `git add -A` + `git commit`**，提交信息用简洁中文概括本次改动（如 `feat:` / `fix:` / `docs:` 前缀）
- 若改动属于同一任务的多个连续步骤，可在任务完成时统一提交一次
- **commit 完成后自动 `git push` 推送到远程仓库（origin）**，推送失败（如认证/网络）时提示用户并保留本地提交
- **push 必须在 Windows 侧执行**（`cmd.exe /c "cd /d 项目路径 && git push origin main"`）——WSL 侧 git 无 Git Credential Manager 凭据会卡认证；Windows 侧 Git for Windows 已有凭据缓存
- 构建产物（`release/*.exe`、`src-tauri/target`）已被 `.gitignore` 忽略，无需特殊处理
