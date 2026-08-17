import { computed, ref } from "vue";
import zhCN from "./locales/zh-CN";
import zhTW from "./locales/zh-TW";
import en from "./locales/en";

export type Locale = "zh-CN" | "zh-TW" | "en";

export const LOCALE_KEY = "cache-manager:locale";

export const LOCALES: Locale[] = ["zh-CN", "zh-TW", "en"];

const messages: Record<Locale, Record<string, string>> = {
  "zh-CN": zhCN as Record<string, string>,
  "zh-TW": zhTW as Record<string, string>,
  en: en as Record<string, string>,
};

/** 界面语言标签（设置页语言选择器用） */
export const LOCALE_LABELS: Record<Locale, string> = {
  "zh-CN": "简体中文",
  "zh-TW": "繁體中文",
  en: "English",
};

function detectDefault(): Locale {
  try {
    const saved = localStorage.getItem(LOCALE_KEY) as Locale | null;
    if (saved && messages[saved]) return saved;
  } catch {
    /* localStorage 不可用时使用默认 */
  }
  return "zh-CN";
}

/** 当前语言（响应式） */
export const localeState = ref<Locale>(detectDefault());

export function setLocale(locale: Locale) {
  localeState.value = locale;
  try {
    localStorage.setItem(LOCALE_KEY, locale);
  } catch {
    /* 忽略 */
  }
  if (typeof document !== "undefined") {
    document.documentElement.lang = locale;
  }
}

/** 翻译：支持 {param} 占位符 */
export function t(key: string, params?: Record<string, string | number>): string {
  const table = messages[localeState.value] ?? messages["zh-CN"];
  let msg = table[key] ?? messages["zh-CN"][key] ?? key;
  if (params) {
    msg = msg.replace(/\{(\w+)\}/g, (_, k: string) =>
      params[k] !== undefined ? String(params[k]) : `{${k}}`,
    );
  }
  return msg;
}

/** 组合式 API：模板中用 t() 即可（导入的 t 已绑定当前 locale） */
export function useI18n() {
  return {
    t,
    locale: computed(() => localeState.value),
    setLocale,
  };
}

// 初始化 document lang
if (typeof document !== "undefined") {
  document.documentElement.lang = localeState.value;
}
