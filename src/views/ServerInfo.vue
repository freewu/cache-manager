<template>
  <div class="server-info">
    <div class="toolbar">
      <n-icon :component="connIcon" :size="15" :color="iconColor" :title="modeText" style="vertical-align: -2px" />
      <span class="conn-name">{{ connName }}</span>
      <n-button size="small" @click="go('/explorer/' + connId)">查看数据</n-button>
      <n-button size="small" @click="go('/console/' + connId)">命令行</n-button>
      <n-button size="small" type="primary" :loading="loading" @click="refreshAll">刷新</n-button>
    </div>

    <n-tabs type="line" animated style="flex: 1; min-height: 0" @update:value="onTabChange">
      <!-- INFO / Memcached stats -->
      <n-tab-pane :name="isMemcached ? 'stats' : 'info'" :tab="isMemcached ? '统计 (stats)' : '概览 (INFO)'">
        <div class="tab-body">
          <n-tabs v-if="info" type="segment" size="small">
            <n-tab-pane
              v-for="sec in info.sections"
              :key="sec.name"
              :name="sec.name"
              :tab="sec.name"
            >
              <n-data-table
                size="small"
                :columns="infoColumns"
                :data="sec.fields.map(([k, v]) => ({ key: k, value: v }))"
                :max-height="560"
              />
            </n-tab-pane>
          </n-tabs>
          <n-empty v-else description="暂无数据" style="margin-top: 60px" />
        </div>
      </n-tab-pane>

      <!-- CONFIG（仅 Redis） -->
      <n-tab-pane v-if="!isMemcached" name="config" tab="配置 (CONFIG)">
        <div class="tab-body">
          <div class="inline-bar">
            <n-input v-model:value="configPattern" size="small" placeholder="* 全部" style="width: 260px" clearable @keyup.enter="loadConfig" />
            <n-button size="small" @click="loadConfig">查询</n-button>
            <n-button size="small" :disabled="!selectedConfig" @click="configDialogVisible = true">修改配置</n-button>
          </div>
          <n-data-table
            size="small"
            :columns="configColumns"
            :data="configRows"
            :max-height="540"
            :row-key="(r: any) => r.key"
            @update:checked-row-keys="(k: any) => (selectedConfig = k[0] || '')"
          />
        </div>
      </n-tab-pane>

      <!-- CLIENTS -->
      <n-tab-pane v-if="!isMemcached" name="clients" tab="客户端">
        <div class="tab-body">
          <n-data-table size="small" :columns="clientColumns" :data="clients" :max-height="560" />
        </div>
      </n-tab-pane>

      <!-- SLOWLOG -->
      <n-tab-pane v-if="!isMemcached" name="slowlog" tab="慢日志">
        <div class="tab-body">
          <n-data-table size="small" :columns="slowlogColumns" :data="slowlog" :max-height="560" />
        </div>
      </n-tab-pane>

      <!-- TOPOLOGY -->
      <n-tab-pane v-if="!isMemcached" name="topology" tab="拓扑">
        <div class="tab-body">
          <div class="topo-list">
            <n-card
              v-for="(n, i) in topology"
              :key="i"
              size="small"
              class="topo-card"
              :class="n.status === 'up' ? 'up' : 'down'"
            >
              <div class="topo-row">
                <n-tag size="small" :type="roleTag(n.role)" :bordered="false">{{ roleText(n.role) }}</n-tag>
                <span class="topo-host">{{ n.host }}:{{ n.port }}</span>
                <n-tag size="small" :type="n.status === 'up' ? 'success' : 'error'">{{ n.status }}</n-tag>
              </div>
              <div v-if="n.extra" class="topo-extra muted">{{ n.extra }}</div>
            </n-card>
          </div>
          <n-empty v-if="topology.length === 0" description="暂无拓扑信息" style="margin-top: 40px" />
        </div>
      </n-tab-pane>
    </n-tabs>

    <!-- 修改配置弹窗 -->
    <n-modal v-model:show="configDialogVisible" preset="card" title="修改配置" style="width: 420px">
      <n-form label-placement="top" size="small">
        <n-form-item label="配置项">
          <n-input v-model:value="configKey" disabled />
        </n-form-item>
        <n-form-item label="新值">
          <n-input v-model:value="configValue" />
        </n-form-item>
      </n-form>
      <template #footer>
        <n-button type="primary" :loading="savingConfig" @click="saveConfig">确定</n-button>
      </template>
    </n-modal>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useRoute, useRouter } from "vue-router";
import { useMessage } from "naive-ui";
import { Cube, Grid } from "@vicons/ionicons5";
import type { DataTableColumns } from "naive-ui";
import type { NodeStatus, ServerInfo } from "@/types";
import * as api from "@/api";
import { useConnectionStore } from "@/store";

const route = useRoute();
const router = useRouter();
const message = useMessage();
const store = useConnectionStore();

const connId = computed(() => route.params.id as string);
const connName = computed(() => store.byId(connId.value)?.name || "未知连接");
const isMemcached = computed(() => store.byId(connId.value)?.mode === "memcached");
const connIcon = computed(() => (isMemcached.value ? Grid : Cube));
const iconColor = computed(() => (isMemcached.value ? "#00ADD8" : "#D82C20"));
const modeText = computed(() => {
  const m = store.byId(connId.value)?.mode;
  return m === "single" ? "单机" : m === "masterSlave" ? "主从" : m === "sentinel" ? "哨兵" : m === "memcached" ? "Memcached" : "集群";
});

const loading = ref(false);
const info = ref<ServerInfo | null>(null);
const configPattern = ref("*");
const configRows = ref<{ key: string; value: string }[]>([]);
const selectedConfig = ref("");
const clients = ref<Record<string, string>[]>([]);
const slowlog = ref<Record<string, unknown>[]>([]);
const topology = ref<NodeStatus[]>([]);

const configDialogVisible = ref(false);
const configKey = ref("");
const configValue = ref("");
const savingConfig = ref(false);

const go = (p: string) => router.push(p);

const infoColumns: DataTableColumns = [
  { title: "参数", key: "key", width: 260, ellipsis: true },
  { title: "值", key: "value", ellipsis: true },
];

const configColumns: DataTableColumns = [
  { type: "selection", width: 40 },
  { title: "配置项", key: "key", width: 260, ellipsis: true },
  { title: "值", key: "value", ellipsis: true },
];

const clientColumns: DataTableColumns = [
  { title: "ID", key: "id", width: 80 },
  { title: "地址", key: "addr", width: 170 },
  { title: "名称", key: "name", width: 110 },
  { title: "DB", key: "db", width: 60 },
  { title: "命令", key: "cmd", width: 100 },
  { title: "状态", key: "flags", ellipsis: true },
  { title: "空闲(s)", key: "idle", width: 80 },
  { title: "年龄(s)", key: "age", width: 80 },
  { title: "用户", key: "user", width: 90 },
  { title: "RESP", key: "resp", width: 60 },
];

const slowlogColumns: DataTableColumns = [
  { title: "ID", key: "id", width: 70 },
  { title: "耗时(µs)", key: "durationUs", width: 100 },
  {
    title: "命令",
    key: "args",
    render: (r: any) => r.args.join(" "),
    ellipsis: true,
  },
  { title: "客户端", key: "clientAddr", width: 170 },
];

onMounted(async () => {
  await refreshAll();
});

async function refreshAll() {
  loading.value = true;
  if (isMemcached.value) {
    try {
      info.value = await api.getServerInfo(connId.value, null);
    } catch (e) {
      message.error(String(e));
    }
    loading.value = false;
    return;
  }
  const [i, cfg, c, s, t] = await Promise.all([
    api.getServerInfo(connId.value, null),
    api.getServerConfig(connId.value, configPattern.value || "*"),
    api.getClients(connId.value).catch(() => []),
    api.getSlowlog(connId.value, 100).catch(() => []),
    api.getTopology(connId.value).catch(() => []),
  ]);
  info.value = i;
  configRows.value = cfg.map(([key, value]) => ({ key, value }));
  clients.value = c;
  slowlog.value = s;
  topology.value = t;
  loading.value = false;
}

async function onTabChange(name: string) {
  if (name === "config") await loadConfig();
  else if (name === "clients") await loadClients();
  else if (name === "slowlog") await loadSlowlog();
  else if (name === "topology") await loadTopology();
}

async function loadConfig() {
  try {
    const cfg = await api.getServerConfig(connId.value, configPattern.value || "*");
    configRows.value = cfg.map(([key, value]) => ({ key, value }));
  } catch (e) {
    message.error(String(e));
  }
}

async function loadClients() {
  try {
    clients.value = await api.getClients(connId.value);
  } catch (e) {
    message.error(String(e));
  }
}

async function loadSlowlog() {
  try {
    slowlog.value = await api.getSlowlog(connId.value, 100);
  } catch (e) {
    message.error(String(e));
  }
}

async function loadTopology() {
  try {
    topology.value = await api.getTopology(connId.value);
  } catch (e) {
    message.error(String(e));
  }
}

async function saveConfig() {
  savingConfig.value = true;
  try {
    await api.setServerConfig(connId.value, configKey.value, configValue.value);
    message.success("已修改（部分配置可能需要 CONFIG REWRITE 或重启生效）");
    configDialogVisible.value = false;
    await loadConfig();
  } catch (e) {
    message.error(String(e));
  } finally {
    savingConfig.value = false;
  }
}

const roleText = (r: string) =>
  r === "master" ? "主" : r === "replica" || r === "slave" ? "从" : r === "sentinel" ? "哨兵" : r;
const roleTag = (r: string) =>
  r === "master" ? "error" : r === "replica" || r === "slave" ? "info" : r === "sentinel" ? "warning" : "default";
</script>

<style scoped>
.server-info {
  display: flex;
  flex-direction: column;
  height: 100%;
}
.toolbar {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 16px;
  border-bottom: 1px solid rgba(128, 128, 128, 0.15);
}
.conn-name {
  font-weight: 600;
  margin-right: auto;
}
.tab-body {
  padding: 12px 16px;
}
.inline-bar {
  display: flex;
  gap: 8px;
  margin-bottom: 10px;
}
.topo-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
  max-width: 720px;
}
.topo-card.up {
  border-color: #18a05833;
}
.topo-card.down {
  border-color: #e74c3c66;
}
.topo-row {
  display: flex;
  align-items: center;
  gap: 10px;
}
.topo-host {
  font-family: Consolas, monospace;
  font-weight: 600;
}
.topo-extra {
  margin-top: 6px;
  font-size: 11px;
  word-break: break-all;
}
.muted {
  color: #888;
}
</style>
