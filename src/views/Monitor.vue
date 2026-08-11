<template>
  <div class="monitor">
    <div class="toolbar">
      <n-icon :component="connIcon" :size="15" :color="iconColor" :title="typeTitle" style="vertical-align: -2px" />
      <span class="conn-name">{{ connName }}</span>
      <n-button size="small" @click="go('/explorer/' + connId)">查看数据</n-button>
      <n-button size="small" @click="go('/console/' + connId)">命令行</n-button>
      <n-button size="small" @click="clearAll">清空</n-button>
      <n-switch v-model:value="autoScroll" size="small" />
      <span class="muted">自动滚动</span>
    </div>

    <div v-if="isMemcached" style="margin: 60px auto; text-align: center">
      <n-empty description="Memcached 不支持实时监控（Pub/Sub / MONITOR）" />
    </div>

    <n-tabs v-else type="line" animated style="flex: 1; min-height: 0">
      <!-- 订阅 -->
      <n-tab-pane name="pubsub" tab="Pub/Sub 订阅">
        <div class="tab-body">
          <div class="pubsub-control">
            <n-input
              v-model:value="channelsInput"
              size="small"
              placeholder="频道，逗号分隔，如 news,events"
              style="flex: 1"
              clearable
            />
            <n-input
              v-model:value="patternsInput"
              size="small"
              placeholder="模式（可选），如 news.*"
              style="flex: 1"
              clearable
            />
            <n-button size="small" type="primary" :disabled="subscribed" @click="startSubscribe">
              订阅
            </n-button>
            <n-button size="small" :disabled="!subscribed" @click="stopSubscribe">取消订阅</n-button>
          </div>

          <div class="pubsub-send">
            <n-input v-model:value="publishChannel" size="small" placeholder="发送频道" style="width: 200px" />
            <n-input v-model:value="publishMessage" size="small" placeholder="消息内容" style="flex: 1" />
            <n-button size="small" type="primary" @click="publish">发布</n-button>
          </div>

          <div class="msg-list" ref="msgListRef">
            <div v-for="(m, i) in messages" :key="i" class="msg-item">
              <n-tag size="tiny" :bordered="false" :type="m.kind === 'pmessage' ? 'warning' : m.kind === 'smessage' ? 'info' : 'success'">
                {{ m.kind }}
              </n-tag>
              <span class="msg-channel">{{ m.channel }}</span>
              <span class="msg-text">{{ m.text }}</span>
              <span class="muted msg-server">{{ m.server }}</span>
            </div>
            <n-empty v-if="messages.length === 0" description="尚未订阅" style="margin-top: 40px" />
          </div>
        </div>
      </n-tab-pane>

      <!-- MONITOR -->
      <n-tab-pane name="monitor" tab="MONITOR 实时命令">
        <div class="tab-body">
          <div class="monitor-control">
            <n-button
              size="small"
              :type="monitoring ? 'error' : 'primary'"
              :loading="monitorStarting"
              @click="monitoring ? stopMonitor() : startMonitor()"
            >
              {{ monitoring ? "停止 MONITOR" : "开启 MONITOR" }}
            </n-button>
            <span class="muted">{{ monitorModeTip }}</span>
          </div>
          <div class="monitor-list" ref="monitorListRef">
            <div v-for="(m, i) in monitorEvents" :key="i" class="monitor-item">
              <span class="monitor-time muted">{{ fmtTime(m.timestamp) }}</span>
              <span class="monitor-db">[{{ m.db }}]</span>
              <span class="monitor-client muted">{{ m.client }}</span>
              <span class="monitor-cmd">{{ m.command }}</span>
              <span class="monitor-args muted">{{ m.args.join(" ") }}</span>
            </div>
            <n-empty v-if="monitorEvents.length === 0" description="开启 MONITOR 后实时显示服务器收到的命令" style="margin-top: 40px" />
          </div>
        </div>
      </n-tab-pane>
    </n-tabs>
  </div>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, ref } from "vue";
import { useRoute, useRouter } from "vue-router";
import { useMessage } from "naive-ui";
import { Cube, Grid } from "@vicons/ionicons5";
import type { PubSubEvent } from "@/types";
import * as api from "@/api";
import { useConnectionStore } from "@/store";

const route = useRoute();
const router = useRouter();
const message = useMessage();
const store = useConnectionStore();

const connId = computed(() => route.params.id as string);
const connName = computed(() => store.byId(connId.value)?.name || "未知连接");
const isMemcached = computed(() => store.byId(connId.value)?.mode === "memcached");
const connMode = computed(() => store.byId(connId.value)?.mode);
const connIcon = computed(() => (connMode.value === "memcached" ? Grid : Cube));
const iconColor = computed(() => (connMode.value === "memcached" ? "#00ADD8" : "#D82C20"));
const typeTitle = computed(() => {
  const m = connMode.value;
  return m === "single" ? "单机" : m === "masterSlave" ? "主从" : m === "sentinel" ? "哨兵" : m === "memcached" ? "Memcached" : "集群";
});

const channelsInput = ref("");
const patternsInput = ref("");
const subscribed = ref(false);
const messages = ref<PubSubEvent[]>([]);
const autoScroll = ref(true);
const msgListRef = ref<HTMLElement>();

const publishChannel = ref("");
const publishMessage = ref("");

const monitoring = ref(false);
const monitorStarting = ref(false);
const monitorEvents = ref<any[]>([]);
const monitorListRef = ref<HTMLElement>();

const go = (p: string) => router.push(p);

const monitorModeTip = computed(() =>
  connMode.value === "cluster" || connMode.value === "sentinel"
    ? "MONITOR 仅支持单机 / 主从模式"
    : ""
);

onBeforeUnmount(() => {
  if (subscribed.value) api.pubsubUnsubscribe(connId.value, [], []).catch(() => {});
  if (monitoring.value) api.stopTasks(connId.value).catch(() => {});
});

async function startSubscribe() {
  const channels = channelsInput.value.split(/[,，\s]+/).filter(Boolean);
  if (channels.length === 0) {
    message.warning("请至少输入一个频道");
    return;
  }
  const patterns = patternsInput.value.split(/[,，\s]+/).filter(Boolean);
  try {
    await api.pubsubSubscribe(connId.value, channels, patterns, (e) => {
      messages.value.push(e as PubSubEvent);
      if (messages.value.length > 2000) messages.value.shift();
      if (autoScroll.value) scrollToBottom(msgListRef.value);
    });
    subscribed.value = true;
    message.success(`已订阅 ${channels.length} 个频道`);
  } catch (e) {
    message.error(String(e));
  }
}

async function stopSubscribe() {
  try {
    await api.pubsubUnsubscribe(connId.value, [], []);
    await api.stopTasks(connId.value);
    subscribed.value = false;
    message.success("已取消订阅");
  } catch (e) {
    message.error(String(e));
  }
}

async function publish() {
  if (!publishChannel.value.trim()) return message.warning("请输入频道");
  try {
    const receivers = await api.pubsubPublish(connId.value, publishChannel.value, publishMessage.value);
    message.success(`已发布，${receivers} 个接收者`);
    publishMessage.value = "";
  } catch (e) {
    message.error(String(e));
  }
}

async function startMonitor() {
  if (connMode.value === "cluster" || connMode.value === "sentinel") {
    message.warning("MONITOR 仅支持单机 / 主从模式");
    return;
  }
  monitorStarting.value = true;
  try {
    await api.startMonitor(connId.value, (e) => {
      monitorEvents.value.push(e);
      if (monitorEvents.value.length > 5000) monitorEvents.value.shift();
      if (autoScroll.value) scrollToBottom(monitorListRef.value);
    });
    monitoring.value = true;
    message.success("MONITOR 已开启");
  } catch (e) {
    message.error(String(e));
  } finally {
    monitorStarting.value = false;
  }
}

async function stopMonitor() {
  try {
    await api.stopTasks(connId.value);
    monitoring.value = false;
    message.success("MONITOR 已停止");
  } catch (e) {
    message.error(String(e));
  }
}

function scrollToBottom(el?: HTMLElement) {
  if (el) el.scrollTo({ top: el.scrollHeight });
}

function fmtTime(ts: number) {
  const d = new Date(ts * 1000);
  return d.toLocaleTimeString("zh-CN", { hour12: false }) + "." + String(d.getMilliseconds()).padStart(3, "0");
}

function clearAll() {
  messages.value = [];
  monitorEvents.value = [];
}
</script>

<style scoped>
.monitor {
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
  display: flex;
  flex-direction: column;
  height: 100%;
}
.pubsub-control {
  display: flex;
  gap: 8px;
  margin-bottom: 8px;
}
.pubsub-send {
  display: flex;
  gap: 8px;
  margin-bottom: 10px;
}
.msg-list,
.monitor-list {
  flex: 1;
  overflow: auto;
  border: 1px solid rgba(128, 128, 128, 0.15);
  border-radius: 6px;
  padding: 8px;
  font-family: "JetBrains Mono", Consolas, monospace;
  font-size: 12.5px;
}
.msg-item,
.monitor-item {
  display: flex;
  align-items: baseline;
  gap: 8px;
  padding: 3px 0;
  border-bottom: 1px dashed rgba(128, 128, 128, 0.1);
}
.msg-channel {
  color: #e8590c;
  font-weight: 600;
}
.msg-text {
  flex: 1;
  word-break: break-all;
}
.msg-server {
  font-size: 11px;
}
.monitor-control {
  display: flex;
  gap: 10px;
  align-items: center;
  margin-bottom: 10px;
}
.monitor-time {
  min-width: 110px;
}
.monitor-db {
  min-width: 36px;
}
.monitor-client {
  min-width: 160px;
}
.monitor-cmd {
  color: #e8590c;
  font-weight: 600;
}
.monitor-args {
  flex: 1;
  word-break: break-all;
}
.muted {
  color: #888;
}
</style>
