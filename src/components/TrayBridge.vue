<template>
  <!-- 无 UI，仅在消息 provider 内接收托盘事件并弹提示 -->
  <span style="display: none"></span>
</template>

<script setup lang="ts">
import { onBeforeUnmount } from "vue";
import { useMessage } from "naive-ui";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { t } from "@/i18n";
import { updateState, checkForUpdate } from "@/version";

const message = useMessage();

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
