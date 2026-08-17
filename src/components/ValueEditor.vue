<template>
  <div class="value-editor">
    <!-- 元信息 -->
    <div class="meta-bar">
      <n-space align="center" :size="12">
        <n-tag size="small" type="info" :bordered="false">{{ typeText(view.type) }}</n-tag>
        <span class="muted">{{ t("valueEditor.length", { n: view.length }) }}</span>
        <span class="muted">{{ t("valueEditor.ttl", { ttl: ttlText }) }}</span>
        <n-input-number
          v-if="showTtlInput"
          v-model:value="newTtl"
          size="tiny"
          :min="-1"
          style="width: 130px"
          :placeholder="t('valueEditor.ttlPlaceholder')"
        />
        <n-button v-if="showTtlInput" size="tiny" @click="applyTtl">{{ t("valueEditor.setTtl") }}</n-button>
        <n-button size="tiny" @click="refresh">{{ t("valueEditor.refresh") }}</n-button>
        <n-popconfirm :positive-text="t('common.delete')" :negative-text="t('common.cancel')" @positive-click="deleteKey">
          <template #trigger>
            <n-button size="tiny" type="error" quaternary>{{ t("valueEditor.deleteKey") }}</n-button>
          </template>
          <span v-html="t('valueEditor.confirmDelete', { key: props.view.key })"></span>
        </n-popconfirm>
      </n-space>
    </div>

    <!-- String -->
    <div v-if="view.type === 'string' && payload.kind === 'string'" class="editor-body">
      <!-- JSON 格式化视图（检测到合法 JSON 时） -->
      <div v-if="isJson && !showRaw" class="json-view-wrap">
        <pre class="json-view" :class="{ dark: isDark }" v-html="jsonHighlight"></pre>
        <div class="json-hint muted">{{ t("valueEditor.jsonHint") }}</div>
      </div>
      <n-input
        v-else
        v-model:value="stringDraft"
        type="textarea"
        :autosize="{ minRows: 6, maxRows: 20 }"
        :placeholder="t('valueEditor.valuePlaceholder')"
      />
      <div class="editor-actions">
        <n-space>
          <n-button size="small" type="primary" :loading="saving" @click="saveString">{{ t("valueEditor.save") }}</n-button>
          <template v-if="isJson">
            <n-button size="small" @click="showRaw = !showRaw">
              {{ showRaw ? t("valueEditor.viewFormatted") : t("valueEditor.viewRaw") }}
            </n-button>
            <n-button v-if="!showRaw" size="small" @click="formatDraft">
              {{ t("valueEditor.editFormatted") }}
            </n-button>
          </template>
          <n-button size="small" @click="toggleEncoding">{{ t("valueEditor.switchEncoding", { enc: stringEncoding }) }}</n-button>
          <n-tag v-if="payload.value.encoding === 'base64'" size="small" type="warning">{{ t("valueEditor.binaryValue") }}</n-tag>
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
          <n-button size="small" @click="showAddField = true">{{ t("valueEditor.addField") }}</n-button>
          <n-button size="small" :disabled="!selectedFields.length" type="error" @click="deleteFields">
            {{ t("valueEditor.deleteSelected", { n: selectedFields.length }) }}
          </n-button>
        </n-space>
      </div>
    </div>

    <!-- List -->
    <div v-else-if="view.type === 'list' && payload.kind === 'list'">
      <n-data-table size="small" :columns="listColumns" :data="listRows" :max-height="480" />
      <div class="editor-actions">
        <n-space align="center">
          <n-input v-model:value="newListValue" size="small" :placeholder="t('valueEditor.elementPlaceholder')" style="width: 240px" />
          <n-button size="small" @click="pushList(true)">{{ t("valueEditor.rpushTail") }}</n-button>
          <n-button size="small" @click="pushList(false)">{{ t("valueEditor.lpushHead") }}</n-button>
        </n-space>
      </div>
    </div>

    <!-- Set -->
    <div v-else-if="view.type === 'set' && payload.kind === 'set'">
      <n-data-table size="small" :columns="setColumns" :data="setRows" :max-height="480" />
      <div class="editor-actions">
        <n-space align="center">
          <n-input v-model:value="newMember" size="small" :placeholder="t('valueEditor.newMember')" style="width: 240px" />
          <n-button size="small" type="primary" @click="addMember">{{ t("valueEditor.addMember") }}</n-button>
        </n-space>
      </div>
    </div>

    <!-- ZSet -->
    <div v-else-if="view.type === 'zset' && payload.kind === 'zset'">
      <n-data-table size="small" :columns="zsetColumns" :data="payload.members" :max-height="480" />
      <div class="editor-actions">
        <n-space align="center">
          <n-input v-model:value="newZMember" size="small" :placeholder="t('valueEditor.member')" style="width: 240px" />
          <n-input-number v-model:value="newZScore" size="small" style="width: 140px" :placeholder="t('valueEditor.score')" />
          <n-button size="small" type="primary" @click="addZMember">{{ t("valueEditor.add") }}</n-button>
        </n-space>
      </div>
    </div>

    <!-- Stream -->
    <div v-else-if="view.type === 'stream' && payload.kind === 'stream'">
      <n-data-table size="small" :columns="streamColumns" :data="payload.entries" :max-height="480" :scroll-x="900" />
      <div class="editor-actions">
        <n-space align="center">
          <n-input v-model:value="newStreamId" size="small" :placeholder="t('valueEditor.idAuto')" style="width: 140px" />
          <n-input v-model:value="newStreamField" size="small" :placeholder="t('valueEditor.field')" style="width: 160px" />
          <n-input v-model:value="newStreamValue" size="small" :placeholder="t('valueEditor.value')" style="width: 160px" />
          <n-button size="small" type="primary" @click="addStreamEntry">{{ t("valueEditor.xadd") }}</n-button>
        </n-space>
      </div>
    </div>

    <n-alert v-else type="warning" :title="t('valueEditor.unsupportedTitle')">{{ t("valueEditor.unsupportedDesc") }}</n-alert>

    <!-- 新增 Hash 字段弹窗 -->
    <n-modal v-model:show="showAddField" preset="card" :title="t('valueEditor.addField')" style="width: 440px">
      <n-form label-placement="top" size="small">
        <n-form-item :label="t('valueEditor.fieldName')">
          <n-input v-model:value="newFieldName" />
        </n-form-item>
        <n-form-item :label="t('valueEditor.value')">
          <n-input v-model:value="newFieldValue" type="textarea" :autosize="{ minRows: 3 }" />
        </n-form-item>
      </n-form>
      <template #footer>
        <n-button type="primary" :loading="saving" @click="addField">{{ t("valueEditor.add") }}</n-button>
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
import { t } from "@/i18n";

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
  message.success(t("valueEditor.formatFilled"));
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
  const ttl = props.view.ttl;
  if (ttl < 0) return t("valueEditor.permanent");
  if (ttl === 0) return t("valueEditor.expired");
  if (ttl < 60) return `${ttl}s`;
  if (ttl < 3600) return `${Math.floor(ttl / 60)}m${ttl % 60}s`;
  return `${Math.floor(ttl / 3600)}h${Math.floor((ttl % 3600) / 60)}m`;
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
    message.success(t("valueEditor.deletedKey", { key: props.view.key }));
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
      message.warning(t("valueEditor.invalidBase64"));
    }
  }
}

async function saveString() {
  saving.value = true;
  try {
    await api.setStringValue(props.connId, props.view.key, stringDraft.value, newTtl.value, stringEncoding.value);
    message.success(t("valueEditor.saved"));
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
    message.success(t("valueEditor.ttlUpdated"));
    emit("changed");
  } catch (e) {
    message.error(String(e));
  }
}

// ---------- Hash ----------
const hashColumns: DataTableColumns<HashField> = [
  {
    title: t("valueEditor.col.field"),
    key: "field",
    render: (r) => display(r.field),
    ellipsis: true,
  },
  {
    title: t("valueEditor.col.value"),
    key: "value",
    render: (r) => display(r.value),
    ellipsis: true,
  },
  {
    title: t("valueEditor.col.op"),
    key: "op",
    width: 80,
    render: (r) =>
      h(
        NButton,
        { size: "tiny", type: "error", quaternary: true, onClick: () => deleteField(r) },
        { default: () => t("common.delete") }
      ),
  },
];

async function addField() {
  if (!newFieldName.value.trim()) return message.warning(t("valueEditor.fieldNameEmpty"));
  saving.value = true;
  try {
    await api.setHashField(props.connId, props.view.key, newFieldName.value, newFieldValue.value);
    message.success(t("valueEditor.added"));
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
    message.success(t("valueEditor.deleted"));
    emit("changed");
  } catch (e) {
    message.error(String(e));
  }
}

async function deleteFields() {
  try {
    await api.deleteHashFields(props.connId, props.view.key, selectedFields.value);
    selectedFields.value = [];
    message.success(t("valueEditor.deleted"));
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
  { title: t("valueEditor.col.index"), key: "index", width: 60 },
  { title: t("valueEditor.col.value"), key: "value", render: (r: any) => display(r.value), ellipsis: true },
];

async function pushList(tail: boolean) {
  if (!newListValue.value) return message.warning(t("valueEditor.needElement"));
  try {
    await api.pushList(props.connId, props.view.key, [newListValue.value], tail);
    newListValue.value = "";
    message.success(t("valueEditor.pushed"));
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
  { title: t("valueEditor.col.member"), key: "value", render: (r: any) => display(r.value), ellipsis: true },
  {
    title: t("valueEditor.col.op"),
    key: "op",
    width: 80,
    render: (r: any) =>
      h(NPopconfirm, { onPositiveClick: () => removeMember(r.value.value) }, {
        trigger: () => h(NButton, { size: "tiny", type: "error", quaternary: true }, { default: () => t("common.delete") }),
        default: () => t("valueEditor.confirmDeleteMember"),
      }),
  },
];

async function addMember() {
  if (!newMember.value.trim()) return message.warning(t("valueEditor.needMember"));
  try {
    await api.addSetMembers(props.connId, props.view.key, [newMember.value]);
    newMember.value = "";
    message.success(t("valueEditor.added"));
    emit("changed");
  } catch (e) {
    message.error(String(e));
  }
}

async function removeMember(member: string) {
  try {
    await api.removeSetMembers(props.connId, props.view.key, [member]);
    message.success(t("valueEditor.deleted"));
    emit("changed");
  } catch (e) {
    message.error(String(e));
  }
}

// ---------- ZSet ----------
const zsetColumns: DataTableColumns = [
  { title: t("valueEditor.col.member"), key: "member", render: (r: any) => display(r.member), ellipsis: true },
  { title: t("valueEditor.col.score"), key: "score", width: 120 },
  {
    title: t("valueEditor.col.op"),
    key: "op",
    width: 80,
    render: (r: any) =>
      h(NPopconfirm, { onPositiveClick: () => removeZMember(r.member.value) }, {
        trigger: () => h(NButton, { size: "tiny", type: "error", quaternary: true }, { default: () => t("common.delete") }),
        default: () => t("valueEditor.confirmDeleteMember"),
      }),
  },
];

async function addZMember() {
  if (!newZMember.value.trim()) return message.warning(t("valueEditor.needMember"));
  try {
    await api.addZsetMembers(props.connId, props.view.key, [[newZScore.value, newZMember.value]]);
    newZMember.value = "";
    message.success(t("valueEditor.added"));
    emit("changed");
  } catch (e) {
    message.error(String(e));
  }
}

async function removeZMember(member: string) {
  try {
    await api.removeZsetMembers(props.connId, props.view.key, [member]);
    message.success(t("valueEditor.deleted"));
    emit("changed");
  } catch (e) {
    message.error(String(e));
  }
}

// ---------- Stream ----------
const streamColumns: DataTableColumns = [
  { title: t("valueEditor.col.id"), key: "id", width: 140 },
  {
    title: t("valueEditor.col.field"),
    key: "fields",
    render: (r: any) =>
      r.fields.map((f: HashField) => `${display(f.field)}=${display(f.value)}`).join(", ") || t("valueEditor.emptyFields"),
    ellipsis: true,
  },
  {
    title: t("valueEditor.col.op"),
    key: "op",
    width: 80,
    render: (r: any) =>
      h(NPopconfirm, { onPositiveClick: () => deleteStreamEntry(r.id) }, {
        trigger: () => h(NButton, { size: "tiny", type: "error", quaternary: true }, { default: () => t("common.delete") }),
        default: () => t("valueEditor.confirmDeleteEntry"),
      }),
  },
];

async function addStreamEntry() {
  if (!newStreamField.value.trim()) return message.warning(t("valueEditor.needFieldName"));
  try {
    await api.xaddStream(props.connId, props.view.key, newStreamId.value, [[newStreamField.value, newStreamValue.value]]);
    newStreamField.value = "";
    newStreamValue.value = "";
    message.success(t("valueEditor.added"));
    emit("changed");
  } catch (e) {
    message.error(String(e));
  }
}

async function deleteStreamEntry(id: string) {
  try {
    await api.xdelStream(props.connId, props.view.key, [id]);
    message.success(t("valueEditor.deleted"));
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
