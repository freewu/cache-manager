<template>
  <n-modal
    :show="show"
    preset="card"
    :title="isEdit ? t('connectionForm.editTitle') : t('connectionForm.createTitle')"
    style="width: 640px"
    @update:show="(v: boolean) => $emit('update:show', v)"
  >
    <n-form :model="form" label-placement="left" label-width="110" size="small">
      <n-form-item :label="t('connectionForm.name')" required>
        <n-input v-model:value="form.name" :placeholder="t('connectionForm.placeholder.name')" />
      </n-form-item>

      <n-form-item :label="t('connectionForm.type')" required>
        <n-tabs v-model:value="typeTab" type="segment" size="small" style="width: 100%">
          <n-tab-pane name="redis" tab="Redis">
            <div style="padding-top: 10px">
              <n-radio-group v-model:value="form.mode">
                <n-radio-button value="single">{{ t("mode.single") }}</n-radio-button>
                <n-radio-button value="masterSlave">{{ t("mode.masterSlave") }}</n-radio-button>
                <n-radio-button value="sentinel">{{ t("mode.sentinel") }}</n-radio-button>
                <n-radio-button value="cluster">{{ t("mode.cluster") }}</n-radio-button>
              </n-radio-group>
            </div>
          </n-tab-pane>
          <n-tab-pane name="memcached" tab="Memcached" />
        </n-tabs>
      </n-form-item>

      <n-form-item :label="modeLabel" required>
        <n-input-group>
          <n-input v-model:value="form.host" :placeholder="t('connectionForm.placeholder.host')" style="flex: 1" />
          <n-input-number
            v-model:value="form.port"
            :show-button="false"
            style="width: 110px"
          />
        </n-input-group>
      </n-form-item>

      <n-form-item v-if="form.mode === 'sentinel'" :label="t('connectionForm.masterName')" required>
        <n-input v-model:value="form.serviceName" :placeholder="t('connectionForm.placeholder.serviceName')" />
      </n-form-item>

      <n-form-item v-if="form.mode !== 'cluster' && form.mode !== 'memcached'" :label="t('connectionForm.database')">
        <n-input-number v-model:value="form.database" :min="0" :max="255" />
      </n-form-item>

      <n-form-item v-if="form.mode !== 'single' && form.mode !== 'memcached'" :label="t('connectionForm.extraNodes')">
        <div style="width: 100%">
          <div
            v-for="(n, i) in form.nodes"
            :key="i"
            style="display: flex; gap: 6px; margin-bottom: 6px"
          >
            <n-input v-model:value="n.host" :placeholder="t('connectionForm.placeholder.nodeHost')" style="flex: 1" />
            <n-input-number v-model:value="n.port" :show-button="false" style="width: 100px" />
            <n-button quaternary type="error" @click="form.nodes.splice(i, 1)">{{ t("connectionForm.delete") }}</n-button>
          </div>
          <n-button size="tiny" @click="form.nodes.push({ host: '', port: 6379 })">
            {{ t("connectionForm.addNode", { name: nodeLabel }) }}
          </n-button>
        </div>
      </n-form-item>

      <n-form-item v-if="form.mode !== 'memcached'" :label="t('connectionForm.username')">
        <n-input v-model:value="form.username" :placeholder="t('connectionForm.placeholder.aclUsername')" />
      </n-form-item>
      <n-form-item v-if="form.mode !== 'memcached'" :label="t('connectionForm.password')">
        <n-input v-model:value="form.password" type="password" show-password-on="click" />
      </n-form-item>

      <n-form-item v-if="form.mode !== 'memcached'" :label="t('connectionForm.options')">
        <n-space>
          <n-checkbox v-model:checked="form.tls">TLS/SSL</n-checkbox>
        </n-space>
      </n-form-item>
      <n-form-item :label="t('connectionForm.timeout')">
        <n-input-number v-model:value="form.connectTimeoutMs" :min="1000" :step="1000" />
      </n-form-item>
    </n-form>

    <template #footer>
      <div style="display: flex; justify-content: space-between; align-items: center">
        <div class="muted" v-if="testResult !== null">
          {{ testResult.ok ? t("connectionForm.testOk") : t("connectionForm.testFail", { msg: testResult.msg }) }}
        </div>
        <div style="margin-left: auto; display: flex; gap: 8px">
          <n-button @click="$emit('update:show', false)">{{ t("connectionForm.cancel") }}</n-button>
          <n-button :loading="testing" @click="onTest">{{ t("connectionForm.test") }}</n-button>
          <n-button type="primary" :loading="saving" @click="onSave">{{ t("connectionForm.saveConnect") }}</n-button>
        </div>
      </div>
    </template>
  </n-modal>
</template>

<script setup lang="ts">
import { computed, reactive, ref, watch } from "vue";
import { useMessage } from "naive-ui";
import type { ConnConfig, ConnMode } from "@/types";
import * as api from "@/api";
import { useConnectionStore } from "@/store";
import { t } from "@/i18n";

const props = defineProps<{
  show: boolean;
  config?: ConnConfig | null;
}>();
const emit = defineEmits<{
  (e: "update:show", v: boolean): void;
  (e: "saved"): void;
}>();

const message = useMessage();
const store = useConnectionStore();
const testing = ref(false);
const saving = ref(false);
const testResult = ref<{ ok: boolean; msg: string } | null>(null);

const isEdit = computed(() => !!props.config);

const form = reactive<ConnConfig>(emptyConfig());

function emptyConfig(): ConnConfig {
  return {
    id: crypto.randomUUID(),
    name: "",
    mode: "single",
    host: "127.0.0.1",
    port: 6379,
    username: "",
    password: "",
    database: 0,
    nodes: [],
    serviceName: "",
    tls: false,
    connectTimeoutMs: 10000,
  };
}

watch(
  () => [props.show, props.config] as const,
  ([show, config]) => {
    if (show) {
      testResult.value = null;
      Object.assign(form, config ? JSON.parse(JSON.stringify(config)) : emptyConfig());
    }
  }
);

watch(
  () => form.mode,
  (mode, prev) => {
    if (mode === "memcached" && prev !== "memcached" && form.port === 6379) {
      form.port = 11211;
    }
    if (mode !== "memcached" && prev === "memcached" && form.port === 11211) {
      form.port = 6379;
    }
  }
);

// tab 区分 Redis / Memcached
const typeTab = ref("redis");
let lastRedisMode: ConnMode = "single";
watch(typeTab, (t) => {
  if (t === "memcached") {
    lastRedisMode = form.mode === "memcached" ? lastRedisMode : form.mode;
    form.mode = "memcached";
  } else {
    form.mode = lastRedisMode;
  }
});
watch(
  () => props.show as boolean,
  (show) => {
    if (show) {
      typeTab.value = form.mode === "memcached" ? "memcached" : "redis";
      if (form.mode !== "memcached") lastRedisMode = form.mode;
    }
  }
);

const modeLabel = computed(() => {
  switch (form.mode) {
    case "sentinel":
      return t("connectionForm.modeLabel.sentinel");
    case "cluster":
      return t("connectionForm.modeLabel.cluster");
    case "masterSlave":
      return t("connectionForm.modeLabel.masterSlave");
    default:
      return t("connectionForm.modeLabel.default");
  }
});

const nodeLabel = computed(() => {
  switch (form.mode) {
    case "sentinel":
      return t("connectionForm.nodeLabel.sentinel");
    case "cluster":
      return t("connectionForm.nodeLabel.cluster");
    default:
      return t("connectionForm.nodeLabel.default");
  }
});

async function onTest() {
  const cfg = buildPayload();
  if (!cfg) return;
  testing.value = true;
  try {
    await api.testConnection(cfg);
    testResult.value = { ok: true, msg: "" };
    message.success(t("connectionForm.success"));
  } catch (e) {
    testResult.value = { ok: false, msg: String(e) };
    message.error(String(e));
  } finally {
    testing.value = false;
  }
}

async function onSave() {
  const cfg = buildPayload();
  if (!cfg) return;
  saving.value = true;
  try {
    await store.saveAndConnect(cfg);
    message.success(t("connectionForm.connected"));
    emit("update:show", false);
    emit("saved");
  } catch (e) {
    message.error(String(e));
  } finally {
    saving.value = false;
  }
}

function buildPayload(): ConnConfig | null {
  if (!form.name.trim()) {
    message.warning(t("connectionForm.err.name"));
    return null;
  }
  if (!form.host.trim()) {
    message.warning(t("connectionForm.err.host"));
    return null;
  }
  if (form.mode === "sentinel" && !form.serviceName?.trim()) {
    message.warning(t("connectionForm.err.sentinelMaster"));
    return null;
  }
  const cfg: ConnConfig = {
    ...form,
    username: form.mode === "memcached" ? null : form.username || null,
    password: form.mode === "memcached" ? null : form.password || null,
    serviceName: form.serviceName || null,
    nodes: form.mode === "memcached" ? [] : form.nodes.filter((n) => n.host.trim()),
    database: form.mode === "cluster" || form.mode === "memcached" ? null : form.database,
  };
  return cfg;
}
</script>
