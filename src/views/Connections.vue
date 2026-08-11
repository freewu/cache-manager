<template>
  <div class="split">
    <!-- ============ 左栏：连接列表 ============ -->
    <div class="left-pane">
      <div class="left-header">
        <div class="left-title-wrap">
          <img class="app-logo" :src="appLogo" alt="Cache Manager" />
          <span class="left-title">连接</span>
        </div>
        <div class="left-actions">
          <n-button size="small" secondary @click="doExport" title="导出连接列表到 JSON">
            <template #icon><n-icon><CloudDownloadOutline /></n-icon></template>
            导出
          </n-button>
          <n-button size="small" secondary @click="triggerImport" title="从 JSON 导入连接列表">
            <template #icon><n-icon><CloudUploadOutline /></n-icon></template>
            导入
          </n-button>
          <input ref="importInput" type="file" accept=".json,application/json" style="display: none" @change="onImportFile" />
          <n-button size="small" type="primary" @click="openCreate">
            <template #icon><n-icon><AddOutline /></n-icon></template>
            新建
          </n-button>
        </div>
      </div>

      <n-input v-model:value="keyword" placeholder="搜索名称 / 地址 / 端口" clearable size="small">
        <template #prefix><n-icon><SearchOutline /></n-icon></template>
      </n-input>

      <div class="mode-filter">
        <n-radio-group v-model:value="modeFilter" size="tiny">
          <n-radio-button value="all">全部</n-radio-button>
          <n-radio-button value="single">单机</n-radio-button>
          <n-radio-button value="masterSlave">主从</n-radio-button>
          <n-radio-button value="sentinel">哨兵</n-radio-button>
          <n-radio-button value="cluster">集群</n-radio-button>
          <n-radio-button value="memcached">Memcached</n-radio-button>
        </n-radio-group>
      </div>

      <div class="conn-list">
        <div
          v-for="cfg in filtered"
          :key="cfg.id"
          class="conn-item"
          :class="{ active: cfg.id === selectedId, connected: isConnected(cfg.id) }"
          @click="select(cfg.id)"
          @dblclick="connect(cfg)"
          title="双击连接"
        >
          <span class="dot" :class="statusDot(cfg.id)"></span>
          <div class="conn-item-body">
            <div class="conn-item-name">{{ cfg.name }}</div>
            <div class="conn-item-url">{{ displayUrl(cfg) }}</div>
          </div>
          <n-icon :component="connIcon(cfg)" :size="16" :color="iconColor(cfg)" :title="modeText(cfg.mode)" class="conn-type-icon" />
        </div>
        <n-empty
          v-if="filtered.length === 0 && !loading"
          description="没有连接配置"
          size="small"
          style="margin-top: 60px"
        />
      </div>

      <div class="left-footer">
        <span class="muted">已保存 {{ filtered.length }}</span>
        <n-space v-if="connectedCount > 0" align="center" :size="4">
          <span class="dot dot-ok"></span>
          <n-button
            size="tiny"
            type="warning"
            quaternary
            :loading="disconnectingAll"
            @click="disconnectAll"
          >
            全部断开
          </n-button>
        </n-space>
      </div>
    </div>

    <!-- ============ 右栏：详情面板 ============ -->
    <div class="right-pane">
      <n-empty
        v-if="!selected && !loading"
        description="选择左侧连接查看详情，或新建一个连接"
        style="margin-top: 120px"
      >
        <template #extra>
          <n-button type="primary" @click="openCreate">新建连接</n-button>
        </template>
      </n-empty>

      <template v-else-if="selected">
        <div class="detail-header">
          <div class="detail-title">
            <h3>
              {{ selected.name }}
              <n-icon :component="connIcon(selected)" :size="17" :color="iconColor(selected)" :title="modeText(selected.mode)" style="vertical-align: -3px; margin-left: 6px" />
            </h3>
            <n-tag :type="statusTag(selected.id)" size="small">{{ statusText(selected.id) }}</n-tag>
          </div>
          <div class="detail-actions">
            <n-button size="small" :loading="testing" @click="test(selected)">测试</n-button>
            <n-button size="small" type="primary" :loading="connectingId === selected.id" @click="connect(selected)">
              {{ isConnected(selected.id) ? "打开" : "连接" }}
            </n-button>
            <n-button size="small" @click="openEdit(selected)">编辑</n-button>
            <n-popconfirm @positive-click="remove(selected.id)">
              <template #trigger>
                <n-button size="small" type="error" quaternary>删除</n-button>
              </template>
              删除该连接配置？
            </n-popconfirm>
          </div>
        </div>

        <n-descriptions bordered :column="2" size="small" label-placement="left" class="detail-desc">
          <n-descriptions-item label="模式">
            {{ modeText(selected.mode) }}<span class="muted">（{{ selected.mode }}）</span>
          </n-descriptions-item>
          <n-descriptions-item label="主地址">
            <span v-if="selected.mode === 'sentinel'">Sentinel 组：{{ selected.serviceName || "-" }}</span>
            <span v-else>{{ selected.host }}:{{ selected.port }}</span>
          </n-descriptions-item>
          <n-descriptions-item label="用户名">
            {{ selected.username || "（无）" }}
          </n-descriptions-item>
          <n-descriptions-item label="密码">
            <span v-if="selected.password">
              <span class="password-mask" @click="showPwd = !showPwd">
                {{ showPwd ? selected.password : "••••••••" }}
              </span>
            </span>
            <span v-else>（无）</span>
          </n-descriptions-item>
          <n-descriptions-item label="数据库">
            {{ selected.database ?? (selected.mode === "cluster" || selected.mode === "memcached" ? "-" : "0") }}
          </n-descriptions-item>
          <n-descriptions-item label="TLS">
            {{ selected.tls ? "启用" : "关闭" }}
          </n-descriptions-item>
          <n-descriptions-item label="连接超时">
            {{ (selected.connectTimeoutMs / 1000).toFixed(1) }} 秒
          </n-descriptions-item>
        </n-descriptions>

        <div class="detail-section">
          <div class="detail-section-title">节点列表</div>
          <n-table size="small" :bordered="false">
            <thead>
              <tr>
                <th>地址</th>
                <th>角色</th>
                <th>状态</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="n in nodeRows" :key="`${n.host}:${n.port}`">
                <td>{{ n.host }}:{{ n.port }}</td>
                <td>{{ roleText(n.role) }}</td>
                <td>
                  <n-tag :type="n.status === 'connected' ? 'success' : n.status === 'error' ? 'error' : 'default'" size="tiny">
                    {{ statusTextOf(n.status) }}
                  </n-tag>
                  <span v-if="n.extra" class="muted"> {{ n.extra }}</span>
                </td>
              </tr>
            </tbody>
          </n-table>
        </div>

        <div class="detail-section" v-if="liveNodes.length">
          <div class="detail-section-title">实时节点状态</div>
          <n-table size="small" :bordered="false">
            <thead>
              <tr>
                <th>地址</th>
                <th>角色</th>
                <th>状态</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="n in liveNodes" :key="`live-${n.host}:${n.port}`">
                <td>{{ n.host }}:{{ n.port }}</td>
                <td>{{ roleText(n.role) }}</td>
                <td>
                  <n-tag :type="n.status === 'connected' ? 'success' : n.status === 'error' ? 'error' : 'default'" size="tiny">
                    {{ statusTextOf(n.status) }}
                  </n-tag>
                  <span v-if="n.extra" class="muted"> {{ n.extra }}</span>
                </td>
              </tr>
            </tbody>
          </n-table>
        </div>

        <div class="detail-section" v-if="errorTextOf(selected.id)">
          <div class="detail-section-title">最近错误</div>
          <n-alert type="error" size="small">{{ errorTextOf(selected.id) }}</n-alert>
        </div>
      </template>
    </div>

    <ConnectionForm :show="formVisible" :config="editing" @update:show="formVisible = $event" @saved="onSaved" />
  </div>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useRouter } from "vue-router";
import { NIcon, useMessage } from "naive-ui";
import { AddOutline, CloudDownloadOutline, CloudUploadOutline, Cube, Grid, SearchOutline } from "@vicons/ionicons5";
import ConnectionForm from "@/components/ConnectionForm.vue";
import * as api from "@/api";
import type { ConnConfig } from "@/types";
import appLogo from "@/assets/logo.png";
import { useConnectionStore } from "@/store";

const store = useConnectionStore();
const router = useRouter();
const message = useMessage();

const formVisible = ref(false);
const editing = ref<ConnConfig | null>(null);
const connectingId = ref("");
const testing = ref(false);
const disconnectingAll = ref(false);
const loading = ref(true);
const keyword = ref("");
const modeFilter = ref("all");
const selectedId = ref("");
const showPwd = ref(false);

let timer: number | undefined;

onMounted(async () => {
  await store.init();
  loading.value = false;
  if (store.saved.length > 0) selectedId.value = store.saved[0].id;
  timer = window.setInterval(() => store.refresh(), 3000);
});

onBeforeUnmount(() => {
  if (timer) window.clearInterval(timer);
});

// 配置变化后保证有选中项
watch(
  () => store.saved.length,
  () => {
    if (!selectedId.value || !store.saved.some((c) => c.id === selectedId.value)) {
      selectedId.value = store.saved[0]?.id || "";
    }
  },
);

const connectedCount = computed(() => store.connectedIds.length);

const filtered = computed(() => {
  const kw = keyword.value.trim().toLowerCase();
  return store.saved.filter((cfg) => {
    if (modeFilter.value !== "all" && cfg.mode !== modeFilter.value) return false;
    if (!kw) return true;
    return (
      cfg.name.toLowerCase().includes(kw) ||
      cfg.host.toLowerCase().includes(kw) ||
      String(cfg.port).includes(kw) ||
      (cfg.username || "").toLowerCase().includes(kw)
    );
  });
});

const selected = computed(() => store.saved.find((c) => c.id === selectedId.value) || null);

/** 配置里的节点（哨兵模式显示 sentinels） */
interface ConfigNodeRow {
  host: string;
  port: number;
  role: string;
  status: string;
  extra?: string | null;
}
const nodeRows = computed<ConfigNodeRow[]>(() => {
  if (!selected.value) return [];
  const cfg = selected.value;
  if (cfg.mode === "sentinel") {
    return cfg.nodes.map((n) => ({ host: n.host, port: n.port, role: "sentinel", status: "config" }));
  }
  if (cfg.mode === "cluster") {
    return [{ host: cfg.host, port: cfg.port, role: "seed", status: "config" }, ...cfg.nodes.map((n) => ({ host: n.host, port: n.port, role: "seed", status: "config" }))];
  }
  if (cfg.mode === "masterSlave") {
    return [
      { host: cfg.host, port: cfg.port, role: "master", status: "config" },
      ...cfg.nodes.map((n) => ({ host: n.host, port: n.port, role: "replica", status: "config" })),
    ];
  }
  return [{ host: cfg.host, port: cfg.port, role: "master", status: "config" }];
});

/** 实时状态（已连接才有） */
const liveNodes = computed(() => {
  const info = store.byId(selectedId.value);
  return info?.nodes || [];
});

const displayUrl = (cfg: ConnConfig) =>
  `${cfg.mode === "cluster" ? "redis-cluster" : "redis"}://${cfg.host}:${cfg.port}${cfg.mode !== "cluster" && cfg.database !== null && cfg.database !== undefined ? "/" + cfg.database : ""}`;

const isConnected = (id: string) => store.byId(id)?.status === "connected";
const statusText = (id: string) => {
  const s = store.byId(id)?.status;
  return s === "connected" ? "已连接" : s === "connecting" ? "连接中" : s === "error" ? "错误" : "未连接";
};
const statusTag = (id: string) => {
  const s = store.byId(id)?.status;
  return s === "connected" ? "success" : s === "connecting" ? "warning" : s === "error" ? "error" : "default";
};
const statusDot = (id: string) => {
  const s = store.byId(id)?.status;
  return s === "connected" ? "dot-ok" : s === "connecting" ? "dot-busy" : s === "error" ? "dot-err" : "dot-off";
};
const statusTextOf = (s: string) =>
  s === "connected" ? "已连接" : s === "connecting" ? "连接中" : s === "error" ? "错误" : "未连接";
const roleText = (r: string) =>
  r === "master" ? "主节点" : r === "replica" || r === "slave" ? "从节点" : r === "sentinel" ? "哨兵" : r === "seed" ? "种子节点" : r || "-";
const errorTextOf = (id: string) => store.byId(id)?.error || "";
const modeText = (m: string) =>
  m === "single" ? "单机" : m === "masterSlave" ? "主从" : m === "sentinel" ? "哨兵" : m === "memcached" ? "Memcached" : "集群";

const connIcon = (cfg: ConnConfig) => (cfg.mode === "memcached" ? Grid : Cube);
const iconColor = (cfg: ConnConfig) => (cfg.mode === "memcached" ? "#00ADD8" : "#D82C20");

function select(id: string) {
  selectedId.value = id;
  showPwd.value = false;
}

function openCreate() {
  editing.value = null;
  formVisible.value = true;
}

// ============ 导入 / 导出 ============
const importInput = ref<HTMLInputElement | null>(null);

/** 导出连接列表到 JSON 文件 */
async function doExport() {
  try {
    const path = await api.exportConnections();
    message.success(`已导出 ${store.saved.length} 个连接到\n${path}`);
  } catch (e) {
    message.error(String(e));
  }
}

function triggerImport() {
  importInput.value?.click();
}

/** 选择导入文件后读取并合并 */
async function onImportFile(ev: Event) {
  const input = ev.target as HTMLInputElement;
  const file = input.files?.[0];
  input.value = "";
  if (!file) return;
  try {
    const text = await file.text();
    const res = await api.importConnections(text);
    if (res.imported > 0) {
      message.success(`导入 ${res.imported} 个连接` + (res.duplicated > 0 ? `，跳过 ${res.duplicated} 个重复（host:port 一致）` : ""));
      // 重新加载连接列表（store.refresh 只刷新状态，需 loadSaved 重新拉取配置）
      await store.loadSaved();
      await store.refresh();
    } else if (res.duplicated > 0) {
      message.warning(`全部 ${res.duplicated} 个连接已存在（host:port 一致），未导入`);
    } else {
      message.warning("文件中没有可导入的连接");
    }
  } catch (e) {
    message.error(String(e));
  }
}

function openEdit(cfg: ConnConfig) {
  editing.value = cfg;
  formVisible.value = true;
}

async function test(cfg: ConnConfig) {
  testing.value = true;
  try {
    await store.test(cfg);
    message.success("连接测试通过");
  } catch (e) {
    message.error(String(e));
  } finally {
    testing.value = false;
  }
}

async function connect(cfg: ConnConfig) {
  if (isConnected(cfg.id)) {
    router.push(`/server/${cfg.id}`);
    return;
  }
  connectingId.value = cfg.id;
  try {
    await store.connect(cfg);
    // 连接成功后默认展示服务器信息页
    router.push(`/server/${cfg.id}`);
  } catch (e) {
    message.error(String(e));
  } finally {
    connectingId.value = "";
  }
}

async function disconnectAll() {
  disconnectingAll.value = true;
  try {
    await store.disconnectAll();
    message.success("已断开全部连接");
  } catch (e) {
    message.error(String(e));
  } finally {
    disconnectingAll.value = false;
  }
}

async function remove(id: string) {
  await store.removeSaved(id);
  message.success("已删除");
}

function onSaved() {
  store.refresh();
  if (store.saved.length > 0) selectedId.value = store.saved[0].id;
}
</script>

<style scoped>
.split {
  display: flex;
  height: 100%;
  min-height: calc(100vh);
}

/* 左栏 */
.left-pane {
  width: 320px;
  min-width: 260px;
  display: flex;
  flex-direction: column;
  border-right: 1px solid var(--n-border-color, #eee);
  padding: 16px 12px;
  gap: 10px;
  overflow: hidden;
}
.left-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.left-title-wrap {
  display: flex;
  align-items: center;
  gap: 8px;
}
.app-logo {
  width: 22px;
  height: 22px;
  border-radius: 5px;
}
.left-title {
  font-weight: 700;
  font-size: 15px;
}
.left-actions {
  display: flex;
  gap: 6px;
  align-items: center;
}
.mode-filter {
  display: flex;
  flex-wrap: wrap;
}
.conn-list {
  flex: 1;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.conn-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px;
  border-radius: 6px;
  cursor: pointer;
  border: 1px solid transparent;
}
.conn-item:hover {
  background: rgba(128, 128, 128, 0.08);
}
.conn-item.active {
  background: rgba(232, 89, 12, 0.1);
  border-color: #e8590c55;
}
.conn-item.connected {
  border-color: #18a05855;
}
.conn-item-body {
  flex: 1;
  min-width: 0;
}
.conn-item-name {
  font-weight: 600;
  font-size: 13px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.conn-item-url {
  font-size: 11px;
  color: #888;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
}
.dot-ok {
  background: #18a058;
}
.dot-busy {
  background: #f0a020;
}
.dot-err {
  background: #d03050;
}
.dot-off {
  background: #bbb;
}
.left-footer {
  display: flex;
  justify-content: space-between;
  align-items: center;
  border-top: 1px solid rgba(128, 128, 128, 0.15);
  padding-top: 10px;
}

/* 右栏 */
.right-pane {
  flex: 1;
  overflow-y: auto;
  padding: 20px 24px;
}
.detail-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  flex-wrap: wrap;
  gap: 12px;
  margin-bottom: 16px;
}
.detail-title {
  display: flex;
  align-items: center;
  gap: 10px;
}
.detail-title h3 {
  margin: 0;
}
.detail-actions {
  display: flex;
  gap: 8px;
}
.detail-desc {
  max-width: 720px;
}
.password-mask {
  cursor: pointer;
  font-family: monospace;
  letter-spacing: 1px;
}
.detail-section {
  margin-top: 20px;
  max-width: 720px;
}
.detail-section-title {
  font-weight: 600;
  font-size: 13px;
  margin-bottom: 8px;
}
.muted {
  color: #888;
  font-size: 12px;
}
</style>
