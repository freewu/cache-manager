<template>
  <div class="split" ref="splitWrap">
    <!-- ============ 左栏：连接列表 ============ -->
    <div class="left-pane" :style="{ width: panelWidth + 'px' }">
      <div class="left-header">
        <div class="left-title-wrap">
          <img class="app-logo" :src="appLogo" alt="Cache Manager" />
          <span class="left-title">{{ t("connections.title") }}</span>
        </div>
        <div class="left-actions">
          <n-button size="small" type="primary" @click="openCreate">
            <template #icon><n-icon><AddOutline /></n-icon></template>
            {{ t("common.create") }}
          </n-button>
        </div>
      </div>

      <n-input v-model:value="keyword" :placeholder="t('connections.searchPlaceholder')" clearable size="small">
        <template #prefix><n-icon><SearchOutline /></n-icon></template>
      </n-input>

      <div class="mode-filter">
        <n-radio-group v-model:value="modeFilter" size="tiny">
          <n-radio-button value="all">{{ t("common.all") }}</n-radio-button>
          <n-radio-button value="single">{{ t("mode.single") }}</n-radio-button>
          <n-radio-button value="masterSlave">{{ t("mode.masterSlave") }}</n-radio-button>
          <n-radio-button value="sentinel">{{ t("mode.sentinel") }}</n-radio-button>
          <n-radio-button value="cluster">{{ t("mode.cluster") }}</n-radio-button>
          <n-radio-button value="memcached">{{ t("mode.memcached") }}</n-radio-button>
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
          :title="t('connections.dblclickHint')"
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
          :description="t('connections.empty')"
          size="small"
          style="margin-top: 60px"
        />
      </div>

      <div class="left-footer">
        <span class="muted">{{ t("connections.savedCount", { n: filtered.length }) }}</span>
        <n-space v-if="connectedCount > 0" align="center" :size="4">
          <span class="dot dot-ok"></span>
          <n-button
            size="tiny"
            type="warning"
            quaternary
            :loading="disconnectingAll"
            @click="disconnectAll"
          >
            {{ t("connections.disconnectAll") }}
          </n-button>
        </n-space>
      </div>

      <!-- 导入 / 导出 -->
      <input
        ref="importInput"
        type="file"
        accept=".json,application/json"
        style="display: none"
        @change="onImportFile"
      />
      <div class="conn-import-export">
        <span class="conn-import-icon" :title="t('connections.exportTitle')" @click="doExport">
          <svg width="18" height="18" viewBox="0 0 512 512" fill="currentColor"><path d="M382.56 233.376c-5.12-5.12-13.44-5.12-18.56 0L264 333.568V56c0-7.36-5.98-13.28-13.28-13.28S237.44 48.64 237.44 56v277.568L137.28 233.376c-5.12-5.12-13.44-5.12-18.56 0s-5.12 13.44 0 18.56l122.88 122.88c2.56 2.56 5.92 3.84 9.28 3.84s6.72-1.28 9.28-3.84l122.88-122.88c5.12-5.12 5.12-13.44 0-18.56zM432 352v96H80v-96c0-7.36-5.98-13.28-13.28-13.28S53.44 344.64 53.44 352v104c0 4.8 2.08 9.12 5.44 12.16A13 13 0 0 0 70.72 472h370.56a13 13 0 0 0 11.84-7.84 13 13 0 0 0 1.44-4.8V352c0-7.36-5.98-13.28-13.28-13.28S432 344.64 432 352z"/></svg>
        </span>
        <span class="conn-import-icon" :title="t('connections.importTitle')" @click="triggerImport">
          <svg width="18" height="18" viewBox="0 0 512 512" fill="currentColor"><path d="M320 96h64v-17.07C384 60.4 358.27 64 332.27 64h-18.67C280 64 256 48 256 32 232 48 208 64 176 64h-20C130 64 128 60.4 128 78.93V96h64zM256 256c57.47 0 104-46.53 104-104v-16c0-13.64-13.4-24-32-24H208c-18.6 0-32 10.36-32 24v16c0 57.47 46.53 104 104 104zM80 205.33c-7.36 0-13.33 5.97-13.33 13.33 0 3.6.7 6.4 1.33 9.33H54.4c-5.2 0-10.4.94-14.4 3.2v53.87c0 7.36 5.97 13.33 13.33 13.33H64v48c-13.33 0-21.33 8-24 24h112v-72h24v-42.13c0-7.36-5.97-13.33-13.33-13.33H32c-2.13 0-5.33-2.29-5.33-6.4v-37.87c0-4.84 5.33-9.33 0-13.33-1.28-1.6-2.67-2.67-4-3.47-1.07-.67-2.4-1.2-3.73-1.6H80zM337.78 283.31c-5.65-3.6-13.17-3.97-18.42-2.13L288 304.76V288c0-8.84-8.28-16-18.33-16H242.33C232.28 272 224 279.16 224 288v16.76l0 0-31.36-23.58c-5.25-1.84-12.77-1.47-18.42 2.13C148.48 301 140.9 317.47 139.4 338.5c-2.15 36.7 2.13 49.47 4.26 57.09 3.2 11.43 7.03 25.12 7.03 25.12 4.8 16.74 20.76 20.29 26.3 20.29h4.6c8.38 0 15.16-6.34 15.16-14.61 0-3.56-9.6.26-20 0 9.94-2.44 10.72-8.41 10.72-14.16 0-7.69-4.93-11.77-9.89-13.67-1.78-.68-3.52-1.32-3.52-1.32h67.46c1.24 0 4.97-.08 5.31 1.31 4.53 18.69 15.7 20.33 21.4 20.33 4.75 0 4.69-1.43 4.69.53 0 1.28-1.23 2.15-1.23 4.09 0 10.84 13.04 13.6 13.04 22.48 0 6.46-4.2 10.7-9.31 12.69-3.69 1.44-1.63 3.01-4.16 4.69-4 2.66-8.24 7.91-8.24 9.3 0 4.02 12.8 9.16 12.8 21.65 0 2.32-.13 4.41-.13 6.08 0 3.29 2.82 5.24 6.11 5.24h11.01c4.5 0 6.54-.87 6.54-4.05 0-2.44-.49-7.5-.84-12.26h32.84c.99 14.8-.34 32.92.64 38.16.5 2.67 1.86 3.44 2.21 4.61 0 0 1.53 1.67 3.87 1.67h10.61c5.75 0 10.72-3.37 10.72-9.5 0-3.92-.18-8.34-.06-12.11 18.85 2.16 37.69-2.58 47.79-14.35 19.2-22.37 42.67-103.85 42.67-159.66 0-45.9-27.15-82.92-72.56-96.7zM256 370.67a48 48 0 1 1 0-96 48 48 0 0 1 0 96zm74.67-85.34a21.33 21.33 0 1 1 0-42.66 21.33 21.33 0 0 1 0 42.66z"/></svg>
        </span>
      </div>
    </div>

    <!-- 可拖动分隔条：连接信息与详情间动态调宽 -->
    <div class="splitter" ref="splitRef" title="拖动调整宽度" @mousedown="startDrag"></div>

    <!-- ============ 右栏：详情面板 ============ -->
    <div class="right-pane">
      <n-empty
        v-if="!selected && !loading"
        :description="t('connections.emptyDetail')"
        style="margin-top: 120px"
      >
        <template #extra>
          <n-button type="primary" @click="openCreate">{{ t("connections.newConnection") }}</n-button>
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
            <n-button size="small" :loading="testing" @click="test(selected)">{{ t("connections.test") }}</n-button>
            <n-button size="small" type="primary" :loading="connectingId === selected.id" @click="connect(selected)">
              {{ isConnected(selected.id) ? t("connections.open") : t("connections.connect") }}
            </n-button>
            <n-button size="small" @click="openEdit(selected)">{{ t("common.edit") }}</n-button>
            <n-popconfirm @positive-click="remove(selected.id)">
              <template #trigger>
                <n-button size="small" type="error" quaternary>{{ t("common.delete") }}</n-button>
              </template>
              {{ t("connections.deleteConfirm") }}
            </n-popconfirm>
          </div>
        </div>

        <n-descriptions bordered :column="2" size="small" label-placement="left" class="detail-desc">
          <n-descriptions-item :label="t('connections.field.mode')">
            {{ modeText(selected.mode) }}<span class="muted">（{{ selected.mode }}）</span>
          </n-descriptions-item>
          <n-descriptions-item :label="t('connections.field.masterAddr')">
            <span v-if="selected.mode === 'sentinel'">{{ t("connections.sentinelGroup", { name: selected.serviceName || "-" }) }}</span>
            <span v-else>{{ selected.host }}:{{ selected.port }}</span>
          </n-descriptions-item>
          <n-descriptions-item :label="t('connections.field.username')">
            {{ selected.username || t("common.none") }}
          </n-descriptions-item>
          <n-descriptions-item :label="t('connections.field.password')">
            <span v-if="selected.password">
              <span class="password-mask" @click="showPwd = !showPwd">
                {{ showPwd ? selected.password : "••••••••" }}
              </span>
            </span>
            <span v-else>{{ t("common.none") }}</span>
          </n-descriptions-item>
          <n-descriptions-item :label="t('connections.field.database')">
            {{ selected.database ?? (selected.mode === "cluster" || selected.mode === "memcached" ? "-" : "0") }}
          </n-descriptions-item>
          <n-descriptions-item :label="t('connections.field.tls')">
            {{ selected.tls ? t("common.enabled") : t("common.disabled") }}
          </n-descriptions-item>
          <n-descriptions-item :label="t('connections.field.timeout')">
            {{ (selected.connectTimeoutMs / 1000).toFixed(1) }} {{ t("common.seconds") }}
          </n-descriptions-item>
        </n-descriptions>

        <div class="detail-section">
          <div class="detail-section-title">{{ t("connections.nodeList") }}</div>
          <n-table size="small" :bordered="false">
            <thead>
              <tr>
                <th>{{ t("common.address") }}</th>
                <th>{{ t("common.role") }}</th>
                <th>{{ t("common.status") }}</th>
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
          <div class="detail-section-title">{{ t("connections.liveNodes") }}</div>
          <n-table size="small" :bordered="false">
            <thead>
              <tr>
                <th>{{ t("common.address") }}</th>
                <th>{{ t("common.role") }}</th>
                <th>{{ t("common.status") }}</th>
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
          <div class="detail-section-title">{{ t("connections.recentErrors") }}</div>
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
import { AddOutline, Cube, Grid, SearchOutline } from "@vicons/ionicons5";
import ConnectionForm from "@/components/ConnectionForm.vue";
import * as api from "@/api";
import type { ConnConfig } from "@/types";
import appLogo from "@/assets/logo.png";
import { useConnectionStore } from "@/store";
import { t } from "@/i18n";

const store = useConnectionStore();
const router = useRouter();
const message = useMessage();

// ===== 左右分隔：连接信息与详情间动态调宽 =====
const PANEL_MIN = 240;
const PANEL_MAX_RATIO = 0.55;
const PANEL_KEY = "cache-manager:connections-panel-width";
const splitWrap = ref<HTMLElement | null>(null);
const panelWidth = ref<number>(
  Math.max(PANEL_MIN, Number(localStorage.getItem(PANEL_KEY)) || 320),
);

function startDrag(e: MouseEvent) {
  if (e.button !== 0) return;
  const onMove = (ev: MouseEvent) => {
    if (!splitWrap.value) return;
    const rect = splitWrap.value.getBoundingClientRect();
    const max = Math.round(rect.width * PANEL_MAX_RATIO);
    const w = Math.round(ev.clientX - rect.left);
    panelWidth.value = Math.min(max, Math.max(PANEL_MIN, w));
  };
  const onUp = () => {
    localStorage.setItem(PANEL_KEY, String(panelWidth.value));
    window.removeEventListener("mousemove", onMove);
    window.removeEventListener("mouseup", onUp);
    document.body.style.cursor = "";
    document.body.style.userSelect = "";
  };
  window.addEventListener("mousemove", onMove);
  window.addEventListener("mouseup", onUp);
  document.body.style.cursor = "col-resize";
  document.body.style.userSelect = "none";
}
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
  return t("status." + s);
};
const statusTag = (id: string) => {
  const s = store.byId(id)?.status;
  return s === "connected" ? "success" : s === "connecting" ? "warning" : s === "error" ? "error" : "default";
};
const statusDot = (id: string) => {
  const s = store.byId(id)?.status;
  return s === "connected" ? "dot-ok" : s === "connecting" ? "dot-busy" : s === "error" ? "dot-err" : "dot-off";
};
const statusTextOf = (s: string) => t("status." + s);
const roleText = (r: string) =>
  r === "master"
    ? t("role.master")
    : r === "replica" || r === "slave"
      ? t("role.replica")
      : r === "sentinel"
        ? t("role.sentinel")
        : r === "seed"
          ? t("role.seed")
          : r || "-";
const errorTextOf = (id: string) => store.byId(id)?.error || "";
const modeText = (m: string) => t("mode." + m);

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

// ===== 导入 / 导出（连接列表最下方）=====
const importInput = ref<HTMLInputElement | null>(null);

/** 导出连接列表到 JSON 文件 */
async function doExport() {
  try {
    const path = await api.exportConnections();
    message.success(t("connections.exported", { n: store.saved.length, path }));
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
      message.success(
        t("connections.imported", { n: res.imported }) +
          (res.duplicated > 0
            ? t("connections.importedDuplicated", { n: res.duplicated })
            : ""),
      );
      // 重新加载连接列表（store.refresh 只刷新状态，需 loadSaved 重新拉取配置）
      await store.loadSaved();
      await store.refresh();
    } else if (res.duplicated > 0) {
      message.warning(t("connections.allDuplicated", { n: res.duplicated }));
    } else {
      message.warning(t("connections.noImportable"));
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
    message.success(t("connections.testOk"));
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
    message.success(t("connections.disconnectedAll"));
  } catch (e) {
    message.error(String(e));
  } finally {
    disconnectingAll.value = false;
  }
}

async function remove(id: string) {
  await store.removeSaved(id);
  message.success(t("connections.deleted"));
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
  overflow: hidden;
}

/* 可拖动分隔条 */
.splitter {
  flex: none;
  width: 6px;
  cursor: col-resize;
  background: transparent;
  user-select: none;
}
.splitter:hover {
  background: rgba(125, 197, 235, 0.14);
}

/* 左栏 */
.left-pane {
  width: 320px;
  min-width: 240px;
  max-width: 55%;
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
  background: rgba(125, 197, 235, 0.1);
  border-color: #7dc5eb55;
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

/* 连接列表最下方的导入 / 导出（图标形式） */
.conn-import-export {
  margin-top: auto;
  display: flex;
  align-items: center;
  justify-content: flex-start;
  gap: 6px;
  padding: 10px 0 0;
  border-top: 1px solid rgba(128, 128, 128, 0.12);
}
.conn-import-export .conn-import-icon {
  cursor: pointer;
  color: var(--n-text-color-3, #a0a0a0);
  transition: color 0.15s ease;
  background: transparent;
  border: none;
  padding: 2px;
  border-radius: 6px;
}
.conn-import-export .conn-import-icon:hover {
  color: var(--brand, #7dc5eb);
  background: rgba(125, 197, 235, 0.1);
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
