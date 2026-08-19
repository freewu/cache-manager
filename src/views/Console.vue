<template>
  <div class="console">
    <div class="toolbar">
      <n-icon :component="connIcon" :size="15" :color="iconColor" :title="typeTitle" style="vertical-align: -2px" />
      <span class="conn-name">{{ connName }}</span>
      <n-select
        v-if="replicas.length > 0"
        v-model:value="currentReplica"
        :options="replicaOptions"
        size="small"
        style="width: 140px"
      />
      <n-button size="small" @click="go('/explorer/' + connId)">{{ t("console.viewData") }}</n-button>
      <n-button size="small" @click="clearOutput">{{ t("console.clear") }}</n-button>
    </div>

    <div class="output" ref="outputRef">
      <div v-for="(r, i) in history" :key="i" class="entry">
        <div class="cmd-line">
          <span class="prompt">{{ r.prompt }}</span>
          <span class="cmd-text">{{ r.command }}</span>
          <span class="elapsed muted">{{ r.result.elapsedMs.toFixed(1) }}ms</span>
        </div>
        <div v-if="!r.result.ok" class="err">{{ r.result.error }}</div>
        <div v-else class="resp">
          <span v-if="r.result.text !== null && r.result.text !== undefined" class="text-resp">
            {{ r.result.text }}
          </span>
          <pre v-else-if="r.result.value !== null && r.result.value !== undefined">{{ formatJson(r.result.value) }}</pre>
          <span v-else class="muted">(empty)</span>
        </div>
      </div>
      <div v-if="executing" class="entry muted">{{ t("console.executing") }}</div>
    </div>

    <div class="input-bar">
      <div v-if="suggestions.length > 0" class="suggest-bar">
        <span
          v-for="(s, i) in suggestions"
          :key="s"
          class="suggest-item"
          :class="{ active: i === suggestIdx }"
          @mouseenter="suggestIdx = i"
          @click="applySuggestion(s)"
        >{{ s }}</span>
        <span class="suggest-hint muted">Tab {{ t("console.complete") }}</span>
      </div>
      <n-input
        ref="inputRef"
        v-model:value="commandLine"
        type="textarea"
        :autosize="{ minRows: 1, maxRows: 6 }"
        :placeholder="t('console.placeholder')"
        @update:value="onInput"
        @keydown="onKeydown"
      />
      <n-button type="primary" :loading="executing" style="margin-top: 6px" @click="run">
        {{ t("console.execute") }}
      </n-button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import { useMessage } from "naive-ui";
import { Cube, Grid } from "@vicons/ionicons5";
import type { CommandResult } from "@/types";
import * as api from "@/api";
import { matchCommands } from "@/console-commands";
import { addExecHistory } from "@/history";
import { useConnectionStore } from "@/store";
import { t } from "@/i18n";

const route = useRoute();
const router = useRouter();
const message = useMessage();
const store = useConnectionStore();

const connId = computed(() => route.params.id as string);
const connName = computed(() => store.byId(connId.value)?.name || t("console.unknownConn"));
const connMode = computed(() => store.byId(connId.value)?.mode);
const connIcon = computed(() => (connMode.value === "memcached" ? Grid : Cube));
const iconColor = computed(() => (connMode.value === "memcached" ? "#00ADD8" : "#D82C20"));
const typeTitle = computed(() => {
  const m = connMode.value;
  return t("mode." + m);
});

const commandLine = ref("");
const executing = ref(false);
const history = ref<{ prompt: string; command: string; result: CommandResult }[]>([]);
const cmdHistory = ref<string[]>([]);
const historyIdx = ref(-1);
const outputRef = ref<HTMLElement>();
const inputRef = ref();

// ===== console 会话持久化：切换页面后再回来内容不丢失 =====
const consoleStorageKey = () => `cm.console.${connId.value}`;
const HISTORY_KEEP = 100;

function saveConsoleState() {
  try {
    localStorage.setItem(
      consoleStorageKey(),
      JSON.stringify({
        history: history.value.slice(0, HISTORY_KEEP),
        commandLine: commandLine.value,
      }),
    );
  } catch {
    // 存储失败（如超出配额）静默忽略
  }
}

function restoreConsoleState() {
  try {
    const raw = localStorage.getItem(consoleStorageKey());
    if (!raw) return;
    const data = JSON.parse(raw);
    if (Array.isArray(data.history)) history.value = data.history;
    if (typeof data.commandLine === "string") commandLine.value = data.commandLine;
  } catch {
    // 解析失败忽略
  }
}

watch(history, saveConsoleState, { deep: true });
watch(commandLine, saveConsoleState);

// ===== 命令输入提示（Redis / Memcached 区分）=====
const suggestions = ref<string[]>([]);
const suggestIdx = ref(0);

/** 根据当前行第一个 token 匹配命令提示 */
function updateSuggestions() {
  const line = commandLine.value;
  const m = line.match(/^\s*(\S*)/);
  if (!m) {
    suggestions.value = [];
    return;
  }
  suggestions.value = matchCommands(connMode.value, m[1]);
  suggestIdx.value = 0;
}

function onInput() {
  updateSuggestions();
}

/** 将建议命令填入输入框（替换第一个 token，保留后续输入） */
function applySuggestion(cmd: string) {
  const line = commandLine.value;
  const m = line.match(/^(\s*)\S*/);
  commandLine.value = m ? m[1] + cmd + line.slice(m[0].length) : cmd;
  suggestions.value = [];
  inputRef.value?.focus?.();
}

const replicas = ref<number[]>([]);
const currentReplica = ref<number | null>(null);
const replicaOptions = computed(() => [
  { label: t("console.master"), value: null as any },
  ...replicas.value.map((i) => ({ label: t("console.replica", { i }), value: i })),
]);

const go = (p: string) => router.push(p);

onMounted(async () => {
  // 恢复上次会话内容（切换页面后回来不丢失）
  restoreConsoleState();
  // 从执行历史“重新执行”跳转过来时，预填命令（优先于恢复的草稿）
  const preset = route.query.cmd;
  if (typeof preset === "string" && preset.trim()) {
    commandLine.value = preset;
    updateSuggestions();
    await nextTick();
    inputRef.value?.focus?.();
  }
  try {
    const nodes = await api.getTopology(connId.value).catch(() => []);
    replicas.value = nodes.filter((n) => n.role === "replica" || n.role === "slave").map((_, i) => i);
  } catch {
    replicas.value = [];
  }
});

async function run() {
  const line = commandLine.value.trim();
  if (!line || executing.value) return;
  // 本地控制台命令：/clear 清空输出（不发送到服务器，不记录执行历史）
  if (line === "/clear" || line === "/cls") {
    history.value = [];
    commandLine.value = "";
    history.value.push({
      prompt: ">",
      command: line,
      result: { command: line, elapsedMs: 0, ok: true, text: t("console.cleared") },
    });
    scrollToBottom();
    return;
  }
  executing.value = true;
  try {
    const result = await api.executeCommand(
      connId.value,
      line,
      currentReplica.value ?? undefined
    );
    history.value.push({ prompt: connMode.value === "cluster" ? "cluster>" : "127.0.0.1>", command: line, result });
    if (line !== cmdHistory.value[cmdHistory.value.length - 1]) {
      cmdHistory.value.push(line);
    }
    historyIdx.value = -1;
    commandLine.value = "";
    suggestions.value = [];
    // 记录执行历史（成功/失败都记，可回溯）
    addExecHistory({
      time: Date.now(),
      connId: connId.value,
      connName: connName.value,
      mode: connMode.value ?? "",
      command: line,
      ok: result.ok,
      elapsedMs: result.elapsedMs,
    });
    await nextTick();
    scrollToBottom();
  } catch (e) {
    message.error(String(e));
  } finally {
    executing.value = false;
  }
}

function formatJson(v: unknown): string {
  return JSON.stringify(v, null, 2);
}

function onKeydown(e: KeyboardEvent) {
  if (e.key === "Enter" && !e.shiftKey) {
    e.preventDefault();
    run();
  } else if (e.key === "Tab") {
    // 命令补全：Tab 选择当前建议
    if (suggestions.value.length > 0) {
      e.preventDefault();
      applySuggestion(suggestions.value[suggestIdx.value] ?? suggestions.value[0]);
    }
  } else if (e.key === "Escape") {
    suggestions.value = [];
  } else if (e.key === "ArrowUp" && !e.shiftKey) {
    e.preventDefault();
    if (cmdHistory.value.length === 0) return;
    if (historyIdx.value === -1) historyIdx.value = cmdHistory.value.length - 1;
    else historyIdx.value = Math.max(0, historyIdx.value - 1);
    commandLine.value = cmdHistory.value[historyIdx.value];
  } else if (e.key === "ArrowDown" && !e.shiftKey) {
    e.preventDefault();
    if (historyIdx.value === -1) return;
    historyIdx.value += 1;
    if (historyIdx.value >= cmdHistory.value.length) {
      historyIdx.value = -1;
      commandLine.value = "";
    } else {
      commandLine.value = cmdHistory.value[historyIdx.value];
    }
  }
}

function clearOutput() {
  history.value = [];
  saveConsoleState();
}

function scrollToBottom() {
  outputRef.value?.scrollTo({ top: outputRef.value.scrollHeight, behavior: "smooth" });
}
</script>

<style scoped>
.console {
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
.output {
  flex: 1;
  overflow: auto;
  padding: 12px 16px;
  font-family: "JetBrains Mono", Consolas, monospace;
  font-size: 13px;
}
.entry {
  margin-bottom: 14px;
}
.cmd-line {
  display: flex;
  gap: 8px;
  align-items: baseline;
}
.prompt {
  color: #18a058;
}
.cmd-text {
  font-weight: 600;
}
.elapsed {
  font-size: 11px;
}
.resp {
  margin-top: 4px;
  padding-left: 8px;
  border-left: 3px solid rgba(128, 128, 128, 0.25);
  white-space: pre-wrap;
  word-break: break-all;
}
.err {
  margin-top: 4px;
  color: #e74c3c;
  padding-left: 8px;
  border-left: 3px solid #e74c3c;
  white-space: pre-wrap;
}
.text-resp {
  color: #7dc5eb;
}
.input-bar {
  padding: 10px 16px;
  border-top: 1px solid rgba(128, 128, 128, 0.15);
  display: flex;
  flex-direction: column;
}
.suggest-bar {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 6px;
  margin-bottom: 6px;
  padding: 6px 8px;
  border: 1px solid rgba(128, 128, 128, 0.2);
  border-radius: 6px;
  background: rgba(128, 128, 128, 0.08);
}
.suggest-item {
  padding: 2px 8px;
  border-radius: 4px;
  font-family: "JetBrains Mono", Consolas, monospace;
  font-size: 12px;
  cursor: pointer;
  color: #7dc5eb;
}
.suggest-item:hover,
.suggest-item.active {
  background: rgba(125, 197, 235, 0.18);
  color: #fff;
}
.suggest-hint {
  margin-left: auto;
  font-size: 11px;
}
.muted {
  color: #888;
}
</style>
