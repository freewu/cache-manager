<template>
  <div class="value-editor">
    <!-- 元信息 -->
    <div class="meta-bar">
      <n-space align="center" :size="12">
        <n-tag size="small" type="info" :bordered="false">{{ typeText(view.type) }}</n-tag>
        <span class="muted">长度: {{ view.length }}</span>
        <span class="muted">TTL: {{ ttlText }}</span>
        <n-input-number
          v-if="showTtlInput"
          v-model:value="newTtl"
          size="tiny"
          :min="-1"
          style="width: 130px"
          placeholder="TTL 秒 (-1 永久)"
        />
        <n-button v-if="showTtlInput" size="tiny" @click="applyTtl">设置 TTL</n-button>
        <n-button size="tiny" @click="refresh">刷新</n-button>
        <n-popconfirm positive-text="删除" negative-text="取消" @positive-click="deleteKey">
          <template #trigger>
            <n-button size="tiny" type="error" quaternary>删除键</n-button>
          </template>
          确认删除键 <b>{{ props.view.key }}</b>？
        </n-popconfirm>
      </n-space>
    </div>

    <!-- String -->
    <div v-if="view.type === 'string' && payload.kind === 'string'" class="editor-body">
      <!-- JSON 格式化视图（检测到合法 JSON 时） -->
      <div v-if="isJson && !showRaw" class="json-view-wrap">
        <pre class="json-view" :class="{ dark: isDark }" v-html="jsonHighlight"></pre>
        <div class="json-hint muted">JSON 格式 · 已格式化展示，保存仍按原文写入</div>
      </div>
      <n-input
        v-else
        v-model:value="stringDraft"
        type="textarea"
        :autosize="{ minRows: 6, maxRows: 20 }"
        placeholder="值内容"
      />
      <div class="editor-actions">
        <n-space>
          <n-button size="small" type="primary" :loading="saving" @click="saveString">保存</n-button>
          <template v-if="isJson">
            <n-button size="small" @click="showRaw = !showRaw">
              {{ showRaw ? "查看格式化 JSON" : "查看原始文本" }}
            </n-button>
            <n-button v-if="!showRaw" size="small" @click="formatDraft">
              以格式化文本编辑
            </n-button>
          </template>
          <n-button size="small" @click="toggleEncoding">切换编码 (当前: {{ stringEncoding }})</n-button>
          <n-tag v-if="payload.value.encoding === 'base64'" size="small" type="warning">二进制值 (Base64)</n-tag>
        </n-space>
      </div>
    </div>

    <!-- Hash -->
    <div v-else-if="view.type === 'hash' && payload.kind === 'hash'">
      <n-data-table
        size="small"
        :columns="hashColumns"
        :data="payload.fields"
        :max-height="480"
        :scroll-x="700"
      />
      <div class="editor-actions">
        <n-space>
          <n-button size="small" @click="showAddField = true">新增字段</n-button>
          <n-button size="small" :disabled="!selectedFields.length" type="error" @click="deleteFields">
            删除选中 ({{ selectedFields.length }})
          </n-button>
        </n-space>
      </div>
    </div>

    <!-- List -->
    <div v-else-if="view.type === 'list' && payload.kind === 'list'">
      <n-data-table size="small" :columns="listColumns" :data="listRows" :max-height="480" />
      <div class="editor-actions">
        <n-space align="center">
          <n-input v-model:value="newListValue" size="small" placeholder="元素值" style="width: 240px" />
          <n-button size="small" @click="pushList(true)">RPUSH 尾部</n-button>
          <n-button size="small" @click="pushList(false)">LPUSH 头部</n-button>
        </n-space>
      </div>
    </div>

    <!-- Set -->
    <div v-else-if="view.type === 'set' && payload.kind === 'set'">
      <n-data-table size="small" :columns="setColumns" :data="setRows" :max-height="480" />
      <div class="editor-actions">
        <n-space align="center">
          <n-input v-model:value="newMember" size="small" placeholder="新成员" style="width: 240px" />
          <n-button size="small" type="primary" @click="addMember">添加成员</n-button>
        </n-space>
      </div>
    </div>

    <!-- ZSet -->
    <div v-else-if="view.type === 'zset' && payload.kind === 'zset'">
      <n-data-table size="small" :columns="zsetColumns" :data="payload.members" :max-height="480" />
      <div class="editor-actions">
        <n-space align="center">
          <n-input v-model:value="newZMember" size="small" placeholder="成员" style="width: 240px" />
          <n-input-number v-model:value="newZScore" size="small" style="width: 140px" placeholder="分数" />
          <n-button size="small" type="primary" @click="addZMember">添加</n-button>
        </n-space>
      </div>
    </div>

    <!-- Stream -->
    <div v-else-if="view.type === 'stream' && payload.kind === 'stream'">
      <n-data-table size="small" :columns="streamColumns" :data="payload.entries" :max-height="480" :scroll-x="900" />
      <div class="editor-actions">
        <n-space align="center">
          <n-input v-model:value="newStreamId" size="small" placeholder="ID (* 自动)" style="width: 140px" />
          <n-input v-model:value="newStreamField" size="small" placeholder="字段" style="width: 160px" />
          <n-input v-model:value="newStreamValue" size="small" placeholder="值" style="width: 160px" />
          <n-button size="small" type="primary" @click="addStreamEntry">XADD</n-button>
        </n-space>
      </div>
    </div>

    <n-alert v-else type="warning" title="暂不支持的类型">该键类型暂不支持编辑</n-alert>

    <!-- 新增 Hash 字段弹窗 -->
    <n-modal v-model:show="showAddField" preset="card" title="新增字段" style="width: 440px">
      <n-form label-placement="top" size="small">
        <n-form-item label="字段名">
          <n-input v-model:value="newFieldName" />
        </n-form-item>
        <n-form-item label="值">
          <n-input v-model:value="newFieldValue" type="textarea" :autosize="{ minRows: 3 }" />
        </n-form-item>
      </n-form>
      <template #footer>
        <n-button type="primary" :loading="saving" @click="addField">添加</n-button>
      </template>
    </n-modal>
  </div>
</template>

<script setup lang="ts">
import { computed, h, ref, watch } from "vue";
import { NButton, NPopconfirm, useMessage, useOsTheme } from "naive-ui";
import type { DataTableColumns } from "naive-ui";
import type { EncodedValue, HashField, ValueView } from "@/types";
import * as api from "@/api";
import { themeState } from "@/theme";

const props = defineProps<{ connId: string; view: ValueView }>();
const emit = defineEmits<{ (e: "changed"): void }>();
const message = useMessage();

const saving = ref(false);
const newTtl = ref<number | null>(null);
const showTtlInput = ref(false);

const stringDraft = ref("");
const stringEncoding = ref<"utf8" | "base64">("utf8");

const showRaw = ref(false);
const showAddField = ref(false);

/** 是否为合法 JSON（顶层为对象/数组） */
const isJson = computed(() => {
  if (stringEncoding.value !== "utf8") return false;
  try {
    const v = JSON.parse(stringDraft.value);
    return v !== null && typeof v === "object";
  } catch {
    return false;
  }
});

/** 格式化后的 JSON 文本（2 空格缩进） */
const jsonPretty = computed(() => {
  try {
    return JSON.stringify(JSON.parse(stringDraft.value), null, 2);
  } catch {
    return stringDraft.value;
  }
});

/** 轻量 JSON 语法高亮 HTML（无第三方依赖） */
const jsonHighlight = computed(() => {
  const esc = (s: string) => s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
  return esc(jsonPretty.value).replace(
    /("(?:\\.|[^"\\])*"\s*:)|("(?:\\.|[^"\\])*")|\b(true|false)\b|\bnull\b|(-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?)/g,
    (m, key, str, bool, nul, num) => {
      if (key) return `<span class="jv-key">${m}</span>`;
      if (str) return `<span class="jv-str">${m}</span>`;
      if (bool) return `<span class="jv-bool">${m}</span>`;
      if (nul) return `<span class="jv-null">${m}</span>`;
      if (num) return `<span class="jv-num">${m}</span>`;
      return m;
    },
  );
});

/** 把格式化文本写入草稿（保存时按此写入 Redis） */
function formatDraft() {
  stringDraft.value = jsonPretty.value;
  message.success("已填充格式化文本（保存将写入缩进后的内容）");
}

// 深色主题适配（与 App.vue 相同的判定逻辑）
const osTheme = useOsTheme();
const isDark = computed(() =>
  themeState.mode === "auto" ? osTheme.value === "dark" : themeState.mode === "dark",
);
const newFieldName = ref("");
const newFieldValue = ref("");

const newListValue = ref("");
const newMember = ref("");
const newZMember = ref("");
const newZScore = ref(1);
const newStreamId = ref("*");
const newStreamField = ref("");
const newStreamValue = ref("");

const selectedFields = ref<string[]>([]);

const payload = computed(() => props.view.payload as any);

watch(
  () => props.view,
  (v) => {
    if (v.payload.kind === "string") {
      stringDraft.value = v.payload.value.value;
      stringEncoding.value = v.payload.value.encoding;
    }
    showRaw.value = false;
    newTtl.value = v.ttl >= 0 ? v.ttl : null;
    showTtlInput.value = true;
  },
  { immediate: true, deep: true }
);

const ttlText = computed(() => {
  const t = props.view.ttl;
  if (t < 0) return "永久";
  if (t === 0) return "已过期";
  if (t < 60) return `${t}s`;
  if (t < 3600) return `${Math.floor(t / 60)}m${t % 60}s`;
  return `${Math.floor(t / 3600)}h${Math.floor((t % 3600) / 60)}m`;
});

const typeText = (t: string) =>
  t === "string" ? "String" : t === "hash" ? "Hash" : t === "list" ? "List" : t === "set" ? "Set" : t === "zset" ? "ZSet" : t === "stream" ? "Stream" : t;

function display(e: EncodedValue): string {
  return e.encoding === "base64" ? `[base64] ${e.value}` : e.value;
}

async function refresh() {
  emit("changed");
}

/** 删除当前键 */
async function deleteKey() {
  try {
    await api.deleteKeys(props.connId, [props.view.key]);
    message.success(`已删除 ${props.view.key}`);
    emit("changed");
  } catch (e) {
    message.error(String(e));
  }
}

// ---------- String ----------
function toggleEncoding() {
  if (stringEncoding.value === "utf8") {
    stringEncoding.value = "base64";
    stringDraft.value = btoa(unescape(encodeURIComponent(stringDraft.value)));
  } else {
    try {
      stringDraft.value = decodeURIComponent(escape(atob(stringDraft.value)));
      stringEncoding.value = "utf8";
    } catch {
      message.warning("不是有效的 Base64 内容");
    }
  }
}

async function saveString() {
  saving.value = true;
  try {
    await api.setStringValue(props.connId, props.view.key, stringDraft.value, newTtl.value, stringEncoding.value);
    message.success("已保存");
    emit("changed");
  } catch (e) {
    message.error(String(e));
  } finally {
    saving.value = false;
  }
}

// ---------- TTL ----------
async function applyTtl() {
  try {
    await api.setTtl(props.connId, props.view.key, newTtl.value ?? -1);
    message.success("TTL 已更新");
    emit("changed");
  } catch (e) {
    message.error(String(e));
  }
}

// ---------- Hash ----------
const hashColumns: DataTableColumns<HashField> = [
  {
    title: "字段",
    key: "field",
    render: (r) => display(r.field),
    ellipsis: true,
  },
  {
    title: "值",
    key: "value",
    render: (r) => display(r.value),
    ellipsis: true,
  },
  {
    title: "操作",
    key: "op",
    width: 80,
    render: (r) =>
      h(
        NButton,
        { size: "tiny", type: "error", quaternary: true, onClick: () => deleteField(r) },
        { default: () => "删除" }
      ),
  },
];

async function addField() {
  if (!newFieldName.value.trim()) return message.warning("字段名不能为空");
  saving.value = true;
  try {
    await api.setHashField(props.connId, props.view.key, newFieldName.value, newFieldValue.value);
    message.success("已添加");
    showAddField.value = false;
    newFieldName.value = "";
    newFieldValue.value = "";
    emit("changed");
  } catch (e) {
    message.error(String(e));
  } finally {
    saving.value = false;
  }
}

async function deleteField(f: HashField) {
  try {
    await api.deleteHashFields(props.connId, props.view.key, [f.field.value]);
    message.success("已删除");
    emit("changed");
  } catch (e) {
    message.error(String(e));
  }
}

async function deleteFields() {
  try {
    await api.deleteHashFields(props.connId, props.view.key, selectedFields.value);
    selectedFields.value = [];
    message.success("已删除");
    emit("changed");
  } catch (e) {
    message.error(String(e));
  }
}

// ---------- List ----------
const listRows = computed(() =>
  (props.view.payload as any).values.map((v: EncodedValue, i: number) => ({ index: i, value: v }))
);

const listColumns: DataTableColumns = [
  { title: "#", key: "index", width: 60 },
  { title: "值", key: "value", render: (r: any) => display(r.value), ellipsis: true },
];

async function pushList(tail: boolean) {
  if (!newListValue.value) return message.warning("请输入元素值");
  try {
    await api.pushList(props.connId, props.view.key, [newListValue.value], tail);
    newListValue.value = "";
    message.success("已推入");
    emit("changed");
  } catch (e) {
    message.error(String(e));
  }
}

// ---------- Set ----------
const setRows = computed(() =>
  (props.view.payload as any).values.map((v: EncodedValue, i: number) => ({ id: i, value: v }))
);
const setColumns: DataTableColumns = [
  { title: "成员", key: "value", render: (r: any) => display(r.value), ellipsis: true },
  {
    title: "操作",
    key: "op",
    width: 80,
    render: (r: any) =>
      h(NPopconfirm, { onPositiveClick: () => removeMember(r.value.value) }, {
        trigger: () => h(NButton, { size: "tiny", type: "error", quaternary: true }, { default: () => "删除" }),
        default: () => "删除该成员？",
      }),
  },
];

async function addMember() {
  if (!newMember.value.trim()) return message.warning("请输入成员");
  try {
    await api.addSetMembers(props.connId, props.view.key, [newMember.value]);
    newMember.value = "";
    message.success("已添加");
    emit("changed");
  } catch (e) {
    message.error(String(e));
  }
}

async function removeMember(member: string) {
  try {
    await api.removeSetMembers(props.connId, props.view.key, [member]);
    message.success("已删除");
    emit("changed");
  } catch (e) {
    message.error(String(e));
  }
}

// ---------- ZSet ----------
const zsetColumns: DataTableColumns = [
  { title: "成员", key: "member", render: (r: any) => display(r.member), ellipsis: true },
  { title: "分数", key: "score", width: 120 },
  {
    title: "操作",
    key: "op",
    width: 80,
    render: (r: any) =>
      h(NPopconfirm, { onPositiveClick: () => removeZMember(r.member.value) }, {
        trigger: () => h(NButton, { size: "tiny", type: "error", quaternary: true }, { default: () => "删除" }),
        default: () => "删除该成员？",
      }),
  },
];

async function addZMember() {
  if (!newZMember.value.trim()) return message.warning("请输入成员");
  try {
    await api.addZsetMembers(props.connId, props.view.key, [[newZScore.value, newZMember.value]]);
    newZMember.value = "";
    message.success("已添加");
    emit("changed");
  } catch (e) {
    message.error(String(e));
  }
}

async function removeZMember(member: string) {
  try {
    await api.removeZsetMembers(props.connId, props.view.key, [member]);
    message.success("已删除");
    emit("changed");
  } catch (e) {
    message.error(String(e));
  }
}

// ---------- Stream ----------
const streamColumns: DataTableColumns = [
  { title: "ID", key: "id", width: 140 },
  {
    title: "字段",
    key: "fields",
    render: (r: any) =>
      r.fields.map((f: HashField) => `${display(f.field)}=${display(f.value)}`).join(", ") || "(空)",
    ellipsis: true,
  },
  {
    title: "操作",
    key: "op",
    width: 80,
    render: (r: any) =>
      h(NPopconfirm, { onPositiveClick: () => deleteStreamEntry(r.id) }, {
        trigger: () => h(NButton, { size: "tiny", type: "error", quaternary: true }, { default: () => "删除" }),
        default: () => "删除该条目？",
      }),
  },
];

async function addStreamEntry() {
  if (!newStreamField.value.trim()) return message.warning("请输入字段名");
  try {
    await api.xaddStream(props.connId, props.view.key, newStreamId.value, [[newStreamField.value, newStreamValue.value]]);
    newStreamField.value = "";
    newStreamValue.value = "";
    message.success("已添加");
    emit("changed");
  } catch (e) {
    message.error(String(e));
  }
}

async function deleteStreamEntry(id: string) {
  try {
    await api.xdelStream(props.connId, props.view.key, [id]);
    message.success("已删除");
    emit("changed");
  } catch (e) {
    message.error(String(e));
  }
}
</script>

<style scoped>
.value-editor {
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.meta-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  flex-wrap: wrap;
  gap: 8px;
}
.editor-body {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.editor-actions {
  margin-top: 8px;
}
.muted {
  color: #888;
  font-size: 12px;
}

/* JSON 格式化视图 */
.json-view-wrap {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.json-view {
  margin: 0;
  padding: 12px;
  border-radius: 6px;
  border: 1px solid rgba(128, 128, 128, 0.2);
  background: rgba(128, 128, 128, 0.06);
  font-family: "Cascadia Mono", Consolas, "Courier New", monospace;
  font-size: 12.5px;
  line-height: 1.6;
  max-height: 480px;
  overflow: auto;
  white-space: pre;
  word-break: break-all;
}
.json-hint {
  font-size: 12px;
}
.jv-key {
  color: #8250df;
}
.jv-str {
  color: #0a7d33;
}
.jv-num {
  color: #0550ae;
}
.jv-bool {
  color: #b35900;
}
.jv-null {
  color: #8b949e;
}
/* 深色主题 */
.json-view.dark {
  background: rgba(255, 255, 255, 0.04);
  border-color: rgba(255, 255, 255, 0.12);
}
.json-view.dark .jv-key {
  color: #d2a8ff;
}
.json-view.dark .jv-str {
  color: #7ee2a8;
}
.json-view.dark .jv-num {
  color: #79c0ff;
}
.json-view.dark .jv-bool {
  color: #ffa657;
}
.json-view.dark .jv-null {
  color: #6e7681;
}
</style>
