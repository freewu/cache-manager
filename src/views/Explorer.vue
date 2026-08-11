<template>
  <div class="explorer">
    <!-- 顶栏 -->
    <div class="toolbar">
      <div class="conn-info">
        <n-icon :component="connIcon" :size="16" :color="iconColor" :title="modeText" style="vertical-align: -2px" />
        <span class="conn-name">{{ connName }}</span>
      </div>

      <n-space align="center" :size="8">
        <n-select
          v-if="databases.length > 0"
          v-model:value="currentDb"
          :options="dbOptions"
          size="small"
          style="width: 110px"
          @update:value="onSwitchDb"
        />
        <n-select
          v-if="replicas.length > 0"
          v-model:value="currentReplica"
          :options="replicaOptions"
          size="small"
          style="width: 140px"
        />
        <n-button size="small" @click="go('/server/' + connId)">服务状态</n-button>
        <n-button v-if="!isMemcached" size="small" @click="go('/monitor/' + connId)">监控</n-button>
        <n-button size="small" @click="go('/console/' + connId)">命令行</n-button>
        <n-button size="small" @click="disconnect">断开</n-button>
      </n-space>
    </div>

    <!-- 主体 -->
    <div class="main">
      <!-- 键列表 -->
      <div class="key-panel">
        <div class="key-toolbar">
          <n-input v-model:value="pattern" size="small" placeholder="模糊查询，* 匹配所有" clearable @keyup.enter="onSearch">
            <template #prefix>
              <n-icon><search-icon /></n-icon>
            </template>
          </n-input>
          <n-button size="small" type="primary" :loading="loadingKeys" @click="onSearch">搜索</n-button>
          <n-button size="small" type="info" secondary @click="openCreate">新建</n-button>
        </div>

        <div class="type-tabs" v-if="!isMemcached">
          <div
            class="type-tab"
            :class="{ active: typeFilter === null }"
            @click="setTypeFilter('')"
          >
            全部
          </div>
          <div
            v-for="t in typeTabOptions"
            :key="t.value"
            class="type-tab"
            :class="{ active: typeFilter === t.value }"
            @click="setTypeFilter(t.value)"
          >
            {{ t.label }}
          </div>
        </div>

        <n-spin :show="loadingKeys">
          <div class="key-list" v-if="displayKeys.length || pattern.trim() !== '*'" @scroll="onScroll">
            <div
              v-for="k in displayKeys"
              :key="k"
              class="key-item"
              :class="{ active: k === currentKey }"
              @click="selectKey(k)"
              @dblclick="openRename(k)"
              :title="k"
            >
              <span class="key-name">{{ k }}</span>
              <span
                v-if="keyType(k)"
                class="key-type"
                :class="['kt-' + keyType(k), { active: typeFilter === keyType(k) }]"
                :title="'查看全部 ' + typeLabel(keyType(k)) + ' 数据'"
                @click.stop="toggleTypeFilter(keyType(k))"
                >{{ typeLabel(keyType(k)) }}</span
              >
              <n-popconfirm positive-text="删除" negative-text="取消" @positive-click="removeKey(k)" placement="left">
                <template #trigger>
                  <span class="key-del" title="删除键" @click.stop>
                    <n-icon :size="14"><delete-icon /></n-icon>
                  </span>
                </template>
                确认删除键 <b>{{ k }}</b>？
              </n-popconfirm>
            </div>
            <div class="key-empty muted" v-if="!displayKeys.length">没有匹配的键</div>
          </div>
          <n-empty v-else description="输入 pattern 开始搜索" style="margin-top: 60px" />
        </n-spin>

        <div class="key-footer muted">
          共 {{ totalShown }} 个
          <span v-if="scan.truncated">（已截断，继续滚动加载）</span>
        </div>
      </div>

      <!-- 值面板 -->
      <div class="value-panel">
        <template v-if="currentView">
          <ValueEditor
            :key="currentKey + ':' + currentReplica"
            :conn-id="connId"
            :view="currentView"
            @changed="onValueChanged"
          />
        </template>
        <n-empty v-else description="选择左侧的键查看 / 编辑值" style="margin-top: 120px" />
      </div>
    </div>

    <!-- 重命名弹窗 -->
    <n-modal v-model:show="renameVisible" preset="card" title="重命名键" style="width: 420px">
      <n-input v-model:value="renameTarget" placeholder="新键名" />
      <template #footer>
        <n-button type="primary" :loading="renaming" @click="doRename">确定</n-button>
      </template>
    </n-modal>

    <!-- 新建键弹窗 -->
    <n-modal v-model:show="createVisible" preset="card" title="新建键" style="width: 480px">
      <n-form label-placement="top" size="small">
        <n-form-item label="键名">
          <n-input v-model:value="createKeyName" placeholder="例如 user:1001" />
        </n-form-item>
        <n-form-item v-if="!isMemcached" label="类型">
          <n-radio-group v-model:value="createKind" size="small">
            <n-radio-button value="string">String</n-radio-button>
            <n-radio-button value="hash">Hash</n-radio-button>
            <n-radio-button value="list">List</n-radio-button>
            <n-radio-button value="set">Set</n-radio-button>
            <n-radio-button value="zset">ZSet</n-radio-button>
            <n-radio-button value="stream">Stream</n-radio-button>
          </n-radio-group>
        </n-form-item>
        <n-form-item v-if="createKind === 'hash' || createKind === 'stream'" label="字段名">
          <n-input v-model:value="createField" placeholder="字段名" />
        </n-form-item>
        <n-form-item v-if="createKind === 'zset'" label="分数">
          <n-input-number v-model:value="createScore" size="small" style="width: 140px" />
        </n-form-item>
        <n-form-item v-if="isMemcached" label="值">
          <n-input v-model:value="createValue" type="textarea" :autosize="{ minRows: 3, maxRows: 8 }" />
        </n-form-item>
        <n-form-item v-else-if="createKind === 'string'" label="值">
          <n-input v-model:value="createValue" type="textarea" :autosize="{ minRows: 3, maxRows: 8 }" />
        </n-form-item>
        <n-form-item v-else label="初始内容">
          <n-input
            v-model:value="createValue"
            :placeholder="createKind === 'list' ? '初始元素' : createKind === 'set' ? '初始成员' : createKind === 'zset' ? '初始成员' : createKind === 'stream' ? '值' : '字段值'"
          />
        </n-form-item>
      </n-form>
      <template #footer>
        <n-button type="primary" :loading="creating" @click="doCreate">创建</n-button>
      </template>
    </n-modal>
  </div>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import { NIcon, useMessage } from "naive-ui";
import { Cube, Grid, SearchOutline, TrashOutline } from "@vicons/ionicons5";
import ValueEditor from "@/components/ValueEditor.vue";
import type { ScanPage, ValueView } from "@/types";
import * as api from "@/api";
import { useConnectionStore } from "@/store";

const SearchIcon = SearchOutline;
const DeleteIcon = TrashOutline;
const route = useRoute();
const router = useRouter();
const message = useMessage();
const store = useConnectionStore();

const connId = computed(() => route.params.id as string);
const conn = computed(() => store.byId(connId.value));
const isMemcached = computed(() => conn.value?.mode === "memcached");
const connIcon = computed(() => (isMemcached.value ? Grid : Cube));
const iconColor = computed(() => (isMemcached.value ? "#00ADD8" : "#D82C20"));
const connName = computed(() => conn.value?.name || "未知连接");
const modeText = computed(() => {
  const m = conn.value?.mode;
  return m === "single" ? "单机" : m === "masterSlave" ? "主从" : m === "sentinel" ? "哨兵" : m === "memcached" ? "Memcached" : "集群";
});

const pattern = ref("*");
const typeFilter = ref<string | null>(null);
const keys = ref<string[]>([]);
const cursor = ref("0");
const loadingKeys = ref(false);
const scan = ref<ScanPage>({ cursor: "0", keys: [], truncated: false, types: {} });
const currentKey = ref("");
const currentView = ref<ValueView | null>(null);
const loadingView = ref(false);

const databases = ref<number[]>([]);
const currentDb = ref<number | null>(null);
const replicas = ref<number[]>([]);
const currentReplica = ref<number | null>(null);

const renameVisible = ref(false);
const renameTarget = ref("");
const renaming = ref(false);
const renamingKey = ref("");

const createVisible = ref(false);
const createKeyName = ref("");
const createKind = ref("string");
const createField = ref("");
const createValue = ref("");
const createScore = ref(1);
const creating = ref(false);

const typeTabOptions = [
  { label: "String", value: "string" },
  { label: "Hash", value: "hash" },
  { label: "List", value: "list" },
  { label: "Set", value: "set" },
  { label: "ZSet", value: "zset" },
  { label: "Stream", value: "stream" },
];
const dbOptions = computed(() =>
  databases.value.map((d) => ({ label: `db${d}`, value: d }))
);
const replicaOptions = computed(() => [
  { label: "主库", value: null as any },
  ...replicas.value.map((i) => ({ label: `从库 ${i}`, value: i })),
]);
const totalShown = computed(() => displayKeys.value.length);

const go = (path: string) => router.push(path);

onMounted(async () => {
  await loadTopology();
  await reloadKeys();
});

onBeforeUnmount(() => {});

watch(currentReplica, () => {
  reloadKeys();
  currentView.value = null;
});

async function loadTopology() {
  try {
    const [dbs, nodes] = await Promise.all([
      api.listDatabases(connId.value).catch(() => [] as number[]),
      api.getTopology(connId.value).catch(() => []),
    ]);
    databases.value = dbs;
    if (currentDb.value === null && dbs.length)
      currentDb.value = store.saved.find((c) => c.id === connId.value)?.database ?? dbs[0];
    const replicaNodes = nodes.filter((n) => n.role === "replica" || n.role === "slave");
    replicas.value = replicaNodes.map((_, i) => i);
  } catch (e) {
    console.warn(e);
  }
}

async function reloadKeys() {
  cursor.value = "0";
  keys.value = [];
  await loadPage(true);
}

/** 搜索：memcached 本地过滤（不调命令），redis 调 scan_keys */
async function onSearch() {
  if (isMemcached.value) return;
  await reloadKeys();
}

/** 简单 glob 匹配（* 任意多字符，? 单字符），用于 memcached 本地搜索 */
function localGlob(pattern: string, text: string): boolean {
  const p = pattern || "*";
  const pl = p.length;
  const tl = text.length;
  // 迭代匹配（支持 * 和 ?）
  let pi = 0;
  let ti = 0;
  let starPi = -1;
  let starTi = -1;
  while (ti < tl) {
    if (pi < pl && (p[pi] === "?" || p[pi] === text[ti])) {
      pi++;
      ti++;
    } else if (pi < pl && p[pi] === "*") {
      starPi = pi++;
      starTi = ti;
    } else if (starPi >= 0) {
      pi = starPi + 1;
      ti = ++starTi;
    } else {
      return false;
    }
  }
  while (pi < pl && p[pi] === "*") pi++;
  return pi === pl;
}

/** memcached 本地模糊查询：含通配符走 glob，否则子串模糊匹配（不区分大小写） */
function localMatch(pattern: string, text: string): boolean {
  const p = pattern || "*";
  if (p === "*") return true;
  if (p.includes("*") || p.includes("?")) return localGlob(p, text);
  return text.toLowerCase().includes(p.toLowerCase());
}

/** memcached 模式下键列表 = 本地过滤；redis = 分页加载的 keys */
const displayKeys = computed(() => {
  if (!isMemcached.value) return keys.value;
  const p = pattern.value.trim() || "*";
  if (p === "*") return keys.value;
  return keys.value.filter((k) => localMatch(p, k));
});

/** 删除键（删除成功后从列表移除） */
async function removeKey(k: string) {
  try {
    await api.deleteKeys(connId.value, [k]);
    message.success(`已删除 ${k}`);
    keys.value = keys.value.filter((x) => x !== k);
    if (currentKey.value === k) {
      currentKey.value = "";
      currentView.value = null;
    }
    if (isMemcached.value) {
      // 本地全量已是最新
      return;
    }
    // redis：保持当前搜索范围
    await loadPage(true);
  } catch (e) {
    message.error(String(e));
  }
}

async function loadPage(reset = false) {
  loadingKeys.value = true;
  try {
    const page = await api.scanKeys(
      connId.value,
      cursor.value,
      pattern.value || "*",
      500,
      typeFilter.value || undefined,
      currentReplica.value ?? undefined
    );
    if (reset) {
      keys.value = page.keys;
    } else {
      const seen = new Set(keys.value);
      keys.value.push(...page.keys.filter((k) => !seen.has(k)));
    }
    cursor.value = page.cursor;
    scan.value = page;
  } catch (e) {
    message.error(String(e));
  } finally {
    loadingKeys.value = false;
  }
}

function onScroll(e: Event) {
  const el = e.target as HTMLElement;
  if (el.scrollTop + el.clientHeight >= el.scrollHeight - 40 && cursor.value !== "0") {
    loadPage(false);
  }
}

/** 键类型（来自 scan 批量查询） */
const keyType = (k: string) => scan.value.types?.[k] || "";
const typeLabel = (t: string) =>
  t === "string"
    ? "String"
    : t === "hash"
      ? "Hash"
      : t === "list"
        ? "List"
        : t === "set"
          ? "Set"
          : t === "zset"
            ? "ZSet"
            : t === "stream"
              ? "Stream"
              : t;
/** 点击类型标签 → 过滤该类型；再次点击取消过滤 */
function toggleTypeFilter(t: string) {
  typeFilter.value = typeFilter.value === t ? null : t;
  reloadKeys();
}

/** tab 切换类型过滤 */
function setTypeFilter(t: string) {
  typeFilter.value = t || null;
  reloadKeys();
}

async function selectKey(key: string): Promise<boolean> {
  currentKey.value = key;
  loadingView.value = true;
  try {
    currentView.value = await api.getValue(connId.value, key, currentReplica.value ?? undefined);
    return true;
  } catch (e) {
    message.error(String(e));
    return false;
  } finally {
    loadingView.value = false;
  }
}

async function onValueChanged() {
  const k = currentKey.value;
  if (!k) return;
  const ok = await selectKey(k);
  if (!ok) {
    // 键可能已被删除：清除选择并从列表移除
    currentKey.value = "";
    currentView.value = null;
    keys.value = keys.value.filter((x) => x !== k);
  }
}

async function onSwitchDb(db: number) {
  try {
    await api.switchDatabase(connId.value, db);
    message.success(`已切换到 db${db}`);
    await store.refresh();
    await reloadKeys();
  } catch (e) {
    message.error(String(e));
  }
}

function openRename(key: string) {
  if (isMemcached.value) {
    message.warning("Memcached 不支持键重命名");
    return;
  }
  renamingKey.value = key;
  renameTarget.value = key;
  renameVisible.value = true;
}

function openCreate() {
  createKeyName.value = "";
  createKind.value = "string";
  createField.value = "";
  createValue.value = "";
  createScore.value = 1;
  createVisible.value = true;
}

async function doCreate() {
  const name = createKeyName.value.trim();
  if (!name) return message.warning("键名不能为空");
  if (createKind.value !== "string" && !createValue.value.trim())
    return message.warning("初始内容不能为空");
  creating.value = true;
  try {
    // zset 的成员放在 createValue，分数单独
    await api.createKey(
      connId.value,
      name,
      createKind.value,
      createKind.value === "zset" ? createValue.value.trim() : createField.value.trim(),
      createKind.value === "zset" ? "" : createValue.value,
      createKind.value === "zset" ? createScore.value : 1,
    );
    message.success(`已创建 ${name}`);
    createVisible.value = false;
    pattern.value = "*";
    await reloadKeys();
    await selectKey(name);
  } catch (e) {
    message.error(String(e));
  } finally {
    creating.value = false;
  }
}

async function doRename() {
  if (!renameTarget.value.trim()) return message.warning("键名不能为空");
  renaming.value = true;
  try {
    await api.renameKey(connId.value, renamingKey.value, renameTarget.value);
    message.success("已重命名");
    renameVisible.value = false;
    await reloadKeys();
    if (currentKey.value === renamingKey.value) {
      currentKey.value = renameTarget.value;
      await selectKey(renameTarget.value);
    }
  } catch (e) {
    message.error(String(e));
  } finally {
    renaming.value = false;
  }
}

async function disconnect() {
  try {
    await api.disconnectConnection(connId.value);
    message.success("已断开");
    router.push("/");
  } catch (e) {
    message.error(String(e));
  }
}
</script>

<style scoped>
.explorer {
  display: flex;
  flex-direction: column;
  height: 100%;
}
.toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 16px;
  border-bottom: 1px solid rgba(128, 128, 128, 0.15);
}
.conn-info {
  display: flex;
  align-items: center;
  gap: 8px;
}
.conn-name {
  font-weight: 600;
}
.main {
  display: flex;
  flex: 1;
  min-height: 0;
}
.key-panel {
  width: 340px;
  min-width: 240px;
  border-right: 1px solid rgba(128, 128, 128, 0.15);
  display: flex;
  flex-direction: column;
}
.key-toolbar {
  display: flex;
  gap: 6px;
  padding: 10px;
  border-bottom: 1px solid rgba(128, 128, 128, 0.1);
}
.type-tabs {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
  padding: 8px 10px;
  border-bottom: 1px solid rgba(128, 128, 128, 0.1);
}
.type-tab {
  padding: 2px 12px;
  font-size: 12px;
  line-height: 20px;
  border-radius: 4px;
  cursor: pointer;
  color: #888;
  border: 1px solid transparent;
  user-select: none;
}
.type-tab:hover {
  background: rgba(128, 128, 128, 0.12);
}
.type-tab.active {
  color: #e8590c;
  background: rgba(232, 89, 12, 0.12);
  border-color: rgba(232, 89, 12, 0.45);
}
.key-list {
  flex: 1;
  overflow: auto;
  padding: 4px;
}
.key-item {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 10px;
  border-radius: 4px;
  cursor: pointer;
  font-family: "JetBrains Mono", Consolas, monospace;
  font-size: 13px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.key-item:hover {
  background: rgba(128, 128, 128, 0.1);
}
.key-item.active {
  background: rgba(232, 89, 12, 0.15);
  color: #e8590c;
}
.key-name {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
}
.key-del {
  flex: none;
  color: #e5484d;
  opacity: 0;
  cursor: pointer;
  display: inline-flex;
  align-items: center;
  padding: 2px;
  border-radius: 4px;
  transition: opacity 0.15s;
}
.key-del:hover {
  background: rgba(229, 72, 77, 0.12);
  opacity: 1;
}
.key-item:hover .key-del {
  opacity: 0.75;
}
.key-type {
  flex-shrink: 0;
  font-size: 10px;
  font-family: "Microsoft YaHei", sans-serif;
  padding: 1px 6px;
  border-radius: 3px;
  cursor: pointer;
  background: rgba(128, 128, 128, 0.15);
  color: #888;
  border: 1px solid transparent;
}
.key-type:hover {
  opacity: 0.85;
}
.key-type.active {
  border-color: currentColor;
  box-shadow: 0 0 0 1px currentColor inset;
}
.kt-string {
  background: rgba(24, 160, 88, 0.16);
  color: #18a058;
}
.kt-hash {
  background: rgba(232, 89, 12, 0.16);
  color: #e8590c;
}
.kt-list {
  background: rgba(32, 128, 240, 0.16);
  color: #2080f0;
}
.kt-set {
  background: rgba(138, 43, 226, 0.16);
  color: #8a2be2;
}
.kt-zset {
  background: rgba(15, 159, 159, 0.16);
  color: #0f9f9f;
}
.kt-stream {
  background: rgba(208, 48, 80, 0.16);
  color: #d03050;
}
.key-empty {
  padding: 20px;
  text-align: center;
}
.key-footer {
  padding: 8px 12px;
  border-top: 1px solid rgba(128, 128, 128, 0.1);
  font-size: 12px;
}
.value-panel {
  flex: 1;
  padding: 16px;
  overflow: auto;
}
.muted {
  color: #888;
}
</style>
