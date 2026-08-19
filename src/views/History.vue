<template>
  <div class="history-page">
    <div class="toolbar">
      <span class="page-title">{{ t("history.title") }}</span>
      <n-input
        v-model:value="keyword"
        :placeholder="t('history.searchPlaceholder')"
        size="small"
        clearable
        style="width: 260px"
      >
        <template #prefix><n-icon><SearchOutline /></n-icon></template>
      </n-input>
      <span class="muted">{{ t("history.count", { n: filtered.length }) }}</span>
      <n-button size="small" type="error" quaternary :disabled="filtered.length === 0" @click="confirmClear">
        {{ t("history.clear") }}
      </n-button>
    </div>

    <n-data-table
      size="small"
      :columns="columns"
      :data="filtered"
      :max-height="640"
      :row-key="(r: any) => r.id"
      :loading="false"
    />
    <n-empty v-if="filtered.length === 0" :description="t('history.empty')" style="margin-top: 80px" />
  </div>
</template>

<script setup lang="ts">
import { computed, h, onMounted, ref } from "vue";
import { useRouter } from "vue-router";
import { NButton, NIcon, NTag, useDialog, useMessage } from "naive-ui";
import { CopyOutline, PlayOutline, SearchOutline, TrashOutline } from "@vicons/ionicons5";
import type { DataTableColumns } from "naive-ui";
import { loadExecHistory, clearExecHistory, removeExecHistory, type ExecHistoryItem } from "@/history";
import { t } from "@/i18n";

const router = useRouter();
const message = useMessage();
const dialog = useDialog();

const keyword = ref("");
const items = ref<ExecHistoryItem[]>([]);

const filtered = computed(() => {
  const k = keyword.value.trim().toLowerCase();
  if (!k) return items.value;
  return items.value.filter(
    (it) =>
      it.command.toLowerCase().includes(k) ||
      it.connName.toLowerCase().includes(k),
  );
});

function fmtTime(ts: number): string {
  const d = new Date(ts);
  const p = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())} ${p(d.getHours())}:${p(d.getMinutes())}:${p(d.getSeconds())}`;
}

async function copyCommand(cmd: string) {
  try {
    await navigator.clipboard.writeText(cmd);
    message.success(t("history.copied"));
  } catch {
    message.error(t("history.copyFailed"));
  }
}

function reRun(it: ExecHistoryItem) {
  router.push(`/console/${it.connId}?cmd=${encodeURIComponent(it.command)}`);
}

function removeItem(id: string) {
  items.value = items.value.filter((it) => it.id !== id);
  removeExecHistory(id);
}

function confirmClear() {
  dialog.warning({
    title: t("history.clearTitle"),
    content: t("history.clearContent"),
    positiveText: t("common.confirm"),
    negativeText: t("common.cancel"),
    onPositiveClick: () => {
      items.value = [];
      clearExecHistory();
      message.success(t("history.cleared"));
    },
  });
}

const columns = computed<DataTableColumns>(() => [
  { title: t("history.col.time"), key: "time", width: 150, render: (r: any) => fmtTime(r.time) },
  {
    title: t("history.col.conn"),
    key: "connName",
    width: 140,
    ellipsis: true,
    render: (r: any) => h("span", { class: "conn-name" }, r.connName),
  },
  {
    title: t("history.col.mode"),
    key: "mode",
    width: 90,
    render: (r: any) =>
      h(
        NTag,
        { size: "small", bordered: false, type: r.mode === "memcached" ? "info" : "error" },
        { default: () => t("mode." + (r.mode || "single")) },
      ),
  },
  {
    title: t("history.col.command"),
    key: "command",
    ellipsis: true,
    render: (r: any) =>
      h("span", { class: "cmd-text", title: r.command }, r.command),
  },
  {
    title: t("history.col.result"),
    key: "ok",
    width: 80,
    render: (r: any) =>
      h(
        NTag,
        { size: "small", bordered: false, type: r.ok ? "success" : "error" },
        { default: () => (r.ok ? t("history.ok") : t("history.fail")) },
      ),
  },
  {
    title: t("history.col.elapsed"),
    key: "elapsedMs",
    width: 90,
    render: (r: any) => `${r.elapsedMs.toFixed(1)}ms`,
  },
  {
    title: t("history.col.action"),
    key: "action",
    width: 130,
    render: (r: any) =>
      h("div", { class: "actions" }, [
        h(
          NButton,
          { size: "tiny", quaternary: true, title: t("history.copy"), onClick: () => copyCommand(r.command) },
          { icon: () => h(NIcon, null, { default: () => h(CopyOutline) }) },
        ),
        h(
          NButton,
          { size: "tiny", quaternary: true, title: t("history.reRun"), onClick: () => reRun(r) },
          { icon: () => h(NIcon, null, { default: () => h(PlayOutline) }) },
        ),
        h(
          NButton,
          { size: "tiny", quaternary: true, title: t("history.delete"), onClick: () => removeItem(r.id) },
          { icon: () => h(NIcon, { color: "#e74c3c" }, { default: () => h(TrashOutline) }) },
        ),
      ]),
  },
]);

onMounted(() => {
  items.value = loadExecHistory();
});
</script>

<style scoped>
.history-page {
  padding: 16px 20px;
  height: 100%;
  overflow: auto;
}
.toolbar {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 14px;
}
.page-title {
  font-size: 16px;
  font-weight: 700;
  margin-right: auto;
}
.conn-name {
  font-weight: 600;
}
.cmd-text {
  font-family: "JetBrains Mono", Consolas, monospace;
  font-size: 12px;
}
.actions {
  display: flex;
  gap: 2px;
}
.muted {
  color: #888;
}
</style>
