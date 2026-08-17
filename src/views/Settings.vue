<template>
  <div class="settings-page">
    <div class="settings-header">
      <h2>{{ t("settings.title") }}</h2>
      <span class="muted">{{ t("settings.subtitle") }}</span>
    </div>

    <!-- ============ 语言 ============ -->
    <n-card :title="t('settings.language')" size="small" class="settings-card">
      <div class="setting-row">
        <div class="setting-info">
          <div class="setting-name">{{ t("settings.language") }}</div>
          <div class="setting-desc">{{ t("settings.languageDesc") }}</div>
        </div>
        <n-radio-group :value="locale" @update:value="onChangeLocale">
          <n-radio-button v-for="l in LOCALES" :key="l" :value="l">
            {{ LOCALE_LABELS[l] }}
          </n-radio-button>
        </n-radio-group>
      </div>
    </n-card>

    <!-- ============ 主题外观 ============ -->
    <n-card :title="t('settings.theme')" size="small" class="settings-card">
      <div class="theme-row">
        <n-radio-group :value="themeState.mode" @update:value="setTheme">
          <n-radio-button v-for="m in THEME_ORDER" :key="m" :value="m">
            {{ t("theme." + m) }}
          </n-radio-button>
        </n-radio-group>
        <div class="theme-preview" :class="`preview-${themeState.mode}`">
          <span class="preview-dot"></span>
          <span class="preview-dot"></span>
          <span class="preview-bar"></span>
        </div>
      </div>
      <p class="card-desc">
        {{ t("settings.current") }}<b>{{ t("theme." + themeState.mode) }}</b>
        <template v-if="themeState.mode === 'auto'">
          {{ t("settings.followSystem") }}
          <n-tag size="tiny" :bordered="false" :type="isDark ? 'primary' : 'default'">
            {{ isDark ? t("settings.systemDark") : t("settings.systemLight") }}
          </n-tag>
          ）
        </template>
        <template v-else>
          {{ t("settings.unified", { name: t("theme." + themeState.mode) }) }}
        </template>
      </p>
    </n-card>

    <!-- ============ 窗口行为 ============ -->
    <n-card :title="t('settings.window')" size="small" class="settings-card">
      <div class="setting-row">
        <div class="setting-info">
          <div class="setting-name">{{ t("settings.minimizeToTray") }}</div>
          <div class="setting-desc">{{ t("settings.minimizeToTrayDesc") }}</div>
        </div>
        <n-switch :value="settings.minimizeToTray" :loading="settingsLoading" @update:value="setMinimizeToTray" />
      </div>
    </n-card>

    <!-- ============ 连接导出 ============ -->
    <n-card :title="t('settings.export')" size="small" class="settings-card">
      <div class="setting-row column">
        <div class="setting-info">
          <div class="setting-name">{{ t("settings.exportDir") }}</div>
          <div class="setting-desc">{{ t("settings.exportDirDesc") }}</div>
        </div>
        <n-input
          v-model:value="exportDirDraft"
          :placeholder="t('settings.exportDirPlaceholder')"
          clearable
        />
        <div class="setting-actions">
          <n-button size="small" secondary @click="exportDirDraft = settings.exportDir || ''">{{ t("settings.reset") }}</n-button>
          <n-button size="small" type="primary" :loading="savingExportDir" @click="saveExportDir">{{ t("settings.saveExportDir") }}</n-button>
        </div>
      </div>
    </n-card>

    <!-- ============ 项目地址 ============ -->
    <n-card :title="t('settings.project')" size="small" class="settings-card">
      <div class="setting-row column">
        <div class="setting-info">
          <div class="setting-name">GitHub</div>
          <div class="setting-desc">{{ t("settings.projectDesc") }}</div>
        </div>
        <div class="project-links">
          <n-button size="small" secondary @click="openUrl(PROJECT_URL)">
            <template #icon><n-icon><LogoGithub /></n-icon></template>
            {{ t("settings.openProject") }}
          </n-button>
          <n-button size="small" type="primary" @click="openUrl(ISSUE_URL)">
            <template #icon><n-icon><BugOutline /></n-icon></template>
            {{ t("settings.submitIssue") }}
          </n-button>
        </div>
      </div>
    </n-card>

    <!-- ============ 关于 ============ -->
    <n-card :title="t('settings.about')" size="small" class="settings-card">
      <div class="setting-row">
        <div class="about-wrap">
          <img class="app-logo" :src="appLogo" alt="Cache Manager" />
          <div class="setting-info">
            <div class="setting-name">Cache Manager</div>
            <div class="setting-desc">{{ t("settings.aboutDesc") }}</div>
          </div>
        </div>
        <n-tag size="small" type="info" :bordered="false">v{{ version }}</n-tag>
      </div>
    </n-card>
  </div>
</template>

<script setup lang="ts">
import { onMounted, ref } from "vue";
import { useMessage, useOsTheme } from "naive-ui";
import { getVersion } from "@tauri-apps/api/app";
import { BugOutline, LogoGithub } from "@vicons/ionicons5";
import { getAppSettings, setAppSettings, openUrl as openExternal } from "@/api";
import { setTheme, THEME_ORDER, themeState } from "@/theme";
import { LOCALES, LOCALE_LABELS, localeState, setLocale, t } from "@/i18n";
import appLogo from "@/assets/logo.png";

/** 项目地址与 Issue 链接 */
const PROJECT_URL = "https://github.com/freewu/cache-manager";
const ISSUE_URL = "https://github.com/freewu/cache-manager/issues/new";

const osTheme = useOsTheme();
const isDark = ref(osTheme.value === "dark");
const message = useMessage();

const version = ref("0.1.1");
const settings = ref({ minimizeToTray: true, exportDir: null as string | null });
const settingsLoading = ref(false);

const exportDirDraft = ref("");
const savingExportDir = ref(false);

const locale = localeState;

function onChangeLocale(l: (typeof LOCALES)[number]) {
  setLocale(l);
  message.success(t("settings.language") + ": " + LOCALE_LABELS[l]);
}

/** 打开外部链接（Tauri 环境走 invoke，浏览器环境回退 window.open） */
async function openUrl(url: string) {
  try {
    await openExternal(url);
  } catch {
    window.open(url, "_blank");
  }
}

async function setMinimizeToTray(v: boolean) {
  settingsLoading.value = true;
  try {
    settings.value.minimizeToTray = v;
    await setAppSettings({ minimizeToTray: v, exportDir: settings.value.exportDir });
  } finally {
    settingsLoading.value = false;
  }
}

async function saveExportDir() {
  savingExportDir.value = true;
  try {
    const dir = exportDirDraft.value.trim();
    settings.value.exportDir = dir || null;
    await setAppSettings({ minimizeToTray: settings.value.minimizeToTray, exportDir: settings.value.exportDir });
    message.success(dir ? t("settings.exportDirSaved", { dir }) : t("settings.exportDirReset"));
  } catch (e) {
    message.error(String(e));
  } finally {
    savingExportDir.value = false;
  }
}

onMounted(async () => {
  try {
    version.value = await getVersion();
  } catch {
    /* 忽略版本获取失败 */
  }
  try {
    settings.value = await getAppSettings();
    exportDirDraft.value = settings.value.exportDir || "";
  } catch {
    /* 忽略 */
  }
});
</script>

<style scoped>
.settings-page {
  max-width: 720px;
  margin: 0 auto;
  padding: 32px 24px;
}
.settings-header {
  margin-bottom: 20px;
}
.settings-header h2 {
  font-size: 20px;
  margin-bottom: 4px;
}
.settings-card {
  margin-bottom: 16px;
}
.theme-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  flex-wrap: wrap;
}
.theme-preview {
  display: flex;
  align-items: flex-end;
  gap: 4px;
  padding: 10px 14px;
  border-radius: 8px;
  border: 1px solid rgba(128, 128, 128, 0.25);
  transition: background-color 0.3s;
}
.preview-dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  background: currentColor;
  opacity: 0.7;
}
.preview-bar {
  width: 34px;
  height: 4px;
  border-radius: 2px;
  background: currentColor;
  opacity: 0.4;
  margin-bottom: 3px;
}
.preview-light {
  background: #ffffff;
  color: #444;
}
.preview-dark {
  background: #101014;
  color: #c8c8d0;
}
.preview-auto {
  background: linear-gradient(135deg, #ffffff 0%, #ffffff 50%, #101014 50%, #101014 100%);
  color: #888;
}
.card-desc {
  margin-top: 14px;
  font-size: 13px;
  color: #888;
}
.setting-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
}
.setting-row.column {
  flex-direction: column;
  align-items: stretch;
  gap: 12px;
}
.setting-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}
.project-links {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}
.about-wrap {
  display: flex;
  align-items: center;
  gap: 12px;
}
.app-logo {
  width: 40px;
  height: 40px;
  border-radius: 8px;
}
.setting-name {
  font-weight: 600;
  font-size: 14px;
}
.setting-desc {
  margin-top: 2px;
  font-size: 12px;
  color: #888;
}
</style>
