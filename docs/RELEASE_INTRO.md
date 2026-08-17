# Cache Manager 功能介绍

> 一款基于 Tauri 2 + Rust + Vue 3 的桌面缓存管理工具，同时支持 **Redis** 与 **Memcached**。

## 特性一览

### 🗄️ 连接管理
- 同时支持 **Redis**（单机 / 主从 / Sentinel 哨兵 / Cluster 集群）与 **Memcached**
- 支持 ACL 用户名/密码认证、TLS/SSL、连接超时配置
- 连接配置本地加密持久化，一键测试、多连接并行管理
- 连接列表**导入 / 导出**为 JSON，跨机器一键迁移，导入自动去重
- 系统托盘保存连接**快速连接**，点击即连

### 🔑 键空间浏览
- Redis：`SCAN` 游标分页、pattern 搜索、类型过滤，集群自动跨节点合并
- Memcached：`stats cachedump` 全量枚举，本地即时模糊搜索
- 键信息（类型 / TTL / 长度 / 内存）、悬停删除键、主从可切从库读

### ✏️ 值查看与编辑
- 全面支持 **String / Hash / List / Set / ZSet / Stream** 六种类型
- String 文本/Base64 切换、JSON 自动格式化高亮
- 非 UTF-8 内容自动 Base64 显示，二进制安全
- 可视化创建新键、编辑值、删除当前键

### 🖥️ 命令行控制台
- 执行任意 Redis / Memcached 命令，响应 JSON 格式化展示
- Memcached `set` 命令智能补全
- 命令历史（↑↓）、执行耗时统计

### 📊 服务器管理 & 监控
- INFO / CONFIG / CLIENT LIST / SLOWLOG，主从/集群拓扑视图
- Pub/Sub 实时订阅与发布、MONITOR 实时监控全部命令

### ⚙️ 设置与体验
- **简体中文 / 繁體中文 / English** 三语一键切换（设置页或托盘）
- 浅色 / 深色 / 跟随系统三档主题
- **自动检查新版本**，侧边栏徽标呼吸灯提醒
- 关闭窗口最小化到系统托盘常驻后台，单实例运行

---

**技术栈**：Tauri 2 · Rust · Vue 3 · Naive UI
