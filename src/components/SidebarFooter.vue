<template>
  <div class="sidebar-footer">
    <input
      ref="importInput"
      type="file"
      accept=".json,application/json"
      style="display: none"
      @change="onImportFile"
    />
    <template v-if="!collapsed">
      <n-button
        size="small"
        secondary
        block
        :title="t('connections.exportTitle')"
        @click="doExport"
      >
        <template #icon><n-icon><CloudDownloadOutline /></n-icon></template>
        {{ t("common.export") }}
      </n-button>
      <n-button
        size="small"
        secondary
        block
        :title="t('connections.importTitle')"
        @click="triggerImport"
      >
        <template #icon><n-icon><CloudUploadOutline /></n-icon></template>
        {{ t("common.import") }}
      </n-button>
    </template>
    <template v-else>
      <n-button size="small" quaternary :title="t('connections.exportTitle')" @click="doExport">
        <template #icon><n-icon><CloudDownloadOutline /></n-icon></template>
      </n-button>
      <n-button size="small" quaternary :title="t('connections.importTitle')" @click="triggerImport">
        <template #icon><n-icon><CloudUploadOutline /></n-icon></template>
      </n-button>
    </template>
  </div>
</template>

<script setup lang="ts">
import { onBeforeUnmount, ref } from "vue";
import { NIcon, useMessage } from "naive-ui";
import { CloudDownloadOutline, CloudUploadOutline } from "@vicons/ionicons5";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import * as api from "@/api";
import { useConnectionStore } from "@/store";
import { t } from "@/i18n";
import { updateState, checkForUpdate } from "@/version";

defineProps<{ collapsed: boolean }>();

const message = useMessage();
const store = useConnectionStore();
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

// 托盘“检查更新”：异步检查并提示结果
let unlistenTrayUpdate: UnlistenFn | null = null;
listen("tray:check-update", () => {
  checkForUpdate().then(() => {
    if (updateState.value.available) {
      message.success(t("app.updateAvailable", { version: updateState.value.latest ?? "" }));
    } else {
      message.success(t("app.upToDate"));
    }
  });
}).then((u) => {
  unlistenTrayUpdate = u;
});

onBeforeUnmount(() => {
  unlistenTrayUpdate?.();
});
</script>

<style scoped>
.sidebar-footer {
  margin-top: auto;
  padding: 10px 12px;
  border-top: 1px solid rgba(128, 128, 128, 0.12);
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.sidebar-footer .n-button {
  width: 100%;
  margin: 0;
}
</style>
