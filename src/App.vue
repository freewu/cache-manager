<template>
  <n-config-provider
    :theme="isDark ? darkTheme : null"
    :theme-overrides="themeOverrides"
    :locale="naiveLocale"
  >
    <n-message-provider>
      <n-dialog-provider>
        <n-layout has-sider style="height: 100vh">
          <n-layout-sider
            bordered
            collapse-mode="width"
            :collapsed-width="64"
            :width="240"
            :collapsed="siderCollapsed"
            show-trigger="arrow-circle"
            @collapse="siderCollapsed = true"
            @expand="siderCollapsed = false"
          >
            <div class="brand">
              <n-icon size="22"><database-icon /></n-icon>
              <span v-if="!siderCollapsed" class="brand-title">Cache Manager</span>
            </div>
            <n-menu
              :value="activeMenu"
              :collapsed="siderCollapsed"
              :collapsed-width="64"
              :collapsed-icon-size="20"
              :options="menuOptions"
              @update:value="onMenuSelect"
            />
          </n-layout-sider>

          <n-layout>
            <n-layout-content content-style="height: 100vh; overflow: hidden">
              <router-view />
            </n-layout-content>
          </n-layout>
        </n-layout>
      </n-dialog-provider>
    </n-message-provider>
  </n-config-provider>
</template>

<script setup lang="ts">
import { computed, h, onBeforeUnmount, onMounted, ref } from "vue";
import { useRoute, useRouter } from "vue-router";
import { NIcon, darkTheme, useOsTheme, enUS, zhCN, zhTW } from "naive-ui";
import type { GlobalThemeOverrides } from "naive-ui";
import { AlbumsOutline, SettingsOutline } from "@vicons/ionicons5";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useConnectionStore } from "@/store";
import { themeState } from "@/theme";
import { localeState, t } from "@/i18n";

const DatabaseIcon = AlbumsOutline;
const osTheme = useOsTheme();
const siderCollapsed = ref(false);
const route = useRoute();
const router = useRouter();
const store = useConnectionStore();

// ===== 主题：light / dark / auto（跟随系统）=====
// 状态与切换逻辑在 src/theme.ts（设置页与侧边栏共用）
const isDark = computed(() =>
  themeState.mode === "auto" ? osTheme.value === "dark" : themeState.mode === "dark",
);

const themeOverrides: GlobalThemeOverrides = {
  common: {
    primaryColor: "#e8590c",
    primaryColorHover: "#f76707",
    primaryColorPressed: "#d9480f",
    borderRadius: "6px",
  },
};

// naive-ui 组件内置文案随界面语言切换
const naiveLocale = computed(() =>
  localeState.value === "zh-TW" ? zhTW : localeState.value === "en" ? enUS : zhCN,
);

const activeMenu = computed(() => {
  const id = route.params.id as string | undefined;
  if (route.name === "explorer") return `explorer:${id}`;
  if (route.name === "server") return `server:${id}`;
  if (route.name === "monitor") return `monitor:${id}`;
  if (route.name === "settings") return "settings";
  return "connections";
});

const menuOptions = computed<any[]>(() => {
  const base: any[] = [
    {
      label: t("app.menu.connections"),
      key: "connections",
      icon: () => h(NIcon, null, { default: () => h(AlbumsOutline) }),
    },
  ];
  const conns = store.connections;
  if (conns.length > 0) {
    base.push({
      type: "group" as const,
      label: t("app.menu.connected"),
      key: "group-connected",
      children: conns.map((c) => ({
        label: c.name,
        key: `explorer:${c.id}`,
      })),
    });
  }
  base.push({
    label: t("app.menu.settings"),
    key: "settings",
    icon: () => h(NIcon, null, { default: () => h(SettingsOutline) }),
  });
  return base;
});

function onMenuSelect(key: string) {
  if (key === "connections") {
    router.push("/");
  } else if (key.startsWith("explorer:")) {
    router.push(`/explorer/${key.split(":")[1]}`);
  } else if (key === "settings") {
    router.push("/settings");
  }
}

onMounted(async () => {
  await store.init();
  // 禁用页面右键菜单（WebView2 默认菜单）
  onContextMenu = (e: MouseEvent) => e.preventDefault();
  window.addEventListener("contextmenu", onContextMenu, { capture: true });
  // 监听托盘快速连接：连接成功后默认展示服务器信息页
  unlistenTrayConnect = await listen<string>("tray:connect", (e) => {
    router.push(`/server/${e.payload}`);
  });
  // 托盘快速连接失败时回到连接管理页
  unlistenTrayError = await listen<string>("tray:connect-error", (e) => {
    router.push("/");
    console.error("托盘连接失败:", e.payload);
  });
  // 托盘点击“设置”：打开设置页
  unlistenTraySettings = await listen("tray:settings", () => {
    router.push("/settings");
  });
});

onBeforeUnmount(() => {
  if (onContextMenu) window.removeEventListener("contextmenu", onContextMenu, { capture: true } as any);
  unlistenTrayConnect?.();
  unlistenTrayError?.();
  unlistenTraySettings?.();
});

let onContextMenu: ((e: MouseEvent) => void) | null = null;
let unlistenTrayConnect: UnlistenFn | null = null;
let unlistenTrayError: UnlistenFn | null = null;
let unlistenTraySettings: UnlistenFn | null = null;
</script>

<style>
* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}
html,
body,
#app {
  height: 100%;
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "PingFang SC",
    "Microsoft YaHei", sans-serif;
}
.brand {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 16px;
  font-weight: 700;
  font-size: 15px;
}
.brand-title {
  white-space: nowrap;
  overflow: hidden;
}
:deep(.n-layout-sider-scroll-container) {
  display: flex;
  flex-direction: column;
  height: 100%;
}
.muted {
  color: #888;
  font-size: 12px;
}
</style>
