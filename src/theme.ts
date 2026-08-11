import { reactive } from "vue";

export type ThemeMode = "light" | "dark" | "auto";

const THEME_KEY = "cache-manager:theme-mode";

/** 共享主题状态（App.vue 与 Settings.vue 共用，自动持久化到 localStorage） */
export const themeState = reactive<{ mode: ThemeMode }>({
  mode: (localStorage.getItem(THEME_KEY) as ThemeMode) || "auto",
});

export function setTheme(mode: ThemeMode) {
  themeState.mode = mode;
  localStorage.setItem(THEME_KEY, mode);
}

export function cycleTheme() {
  const order: ThemeMode[] = ["light", "dark", "auto"];
  setTheme(order[(order.indexOf(themeState.mode) + 1) % order.length]);
}

export const THEME_LABELS: Record<ThemeMode, string> = {
  light: "浅色",
  dark: "深色",
  auto: "跟随系统",
};

export const THEME_ORDER: ThemeMode[] = ["light", "dark", "auto"];
