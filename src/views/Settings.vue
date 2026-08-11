<template>
  <div class="settings-page">
    <div class="settings-header">
      <h2>设置</h2>
      <span class="muted">应用程序偏好设置</span>
    </div>

    <!-- ============ 主题外观 ============ -->
    <n-card title="主题外观" size="small" class="settings-card">
      <div class="theme-row">
        <n-radio-group :value="themeState.mode" @update:value="setTheme">
          <n-radio-button v-for="m in THEME_ORDER" :key="m" :value="m">
            {{ THEME_LABELS[m] }}
          </n-radio-button>
        </n-radio-group>
        <div class="theme-preview" :class="`preview-${themeState.mode}`">
          <span class="preview-dot"></span>
          <span class="preview-dot"></span>
          <span class="preview-bar"></span>
        </div>
      </div>
      <p class="card-desc">
        当前：<b>{{ THEME_LABELS[themeState.mode] }}</b>
        <template v-if="themeState.mode === 'auto'">
          —— 跟随操作系统外观（
          <n-tag size="tiny" :bordered="false" :type="isDark ? 'primary' : 'default'">
            {{ isDark ? "系统当前为深色" : "系统当前为浅色" }}
          </n-tag>
          ）
        </template>
        <template v-else>
          —— 所有窗口统一使用{{ THEME_LABELS[themeState.mode] }}外观
        </template>
      </p>
    </n-card>

    <!-- ============ 窗口行为 ============ -->
    <n-card title="窗口行为" size="small" class="settings-card">
      <div class="setting-row">
        <div class="setting-info">
          <div class="setting-name">关闭窗口时最小化到系统托盘</div>
          <div class="setting-desc">关闭主窗口后程序驻留托盘，可从托盘菜单恢复或退出</div>
        </div>
        <n-switch :value="settings.minimizeToTray" :loading="settingsLoading" @update:value="setMinimizeToTray" />
      </div>
    </n-card>

    <!-- ============ 连接导出 ============ -->
    <n-card title="连接导出" size="small" class="settings-card">
      <div class="setting-row column">
        <div class="setting-info">
          <div class="setting-name">默认导出目录</div>
          <div class="setting-desc">导出连接列表时保存到的文件夹；留空则使用系统下载目录</div>
        </div>
        <n-input
          v-model:value="exportDirDraft"
          placeholder="例如 D:\backup\connections（留空使用默认下载目录）"
          clearable
        />
        <div class="setting-actions">
          <n-button size="small" secondary @click="exportDirDraft = settings.exportDir || ''">重置</n-button>
          <n-button size="small" type="primary" :loading="savingExportDir" @click="saveExportDir">保存导出目录</n-button>
        </div>
      </div>
    </n-card>

    <!-- ============ 关于 ============ -->
    <n-card title="关于" size="small" class="settings-card">
      <div class="setting-row">
        <div class="about-wrap">
          <img class="app-logo" :src="appLogo" alt="Cache Manager" />
          <div class="setting-info">
            <div class="setting-name">Cache Manager</div>
            <div class="setting-desc">Redis / Memcached 桌面管理工具 · 支持单机 / 主从 / Sentinel / Cluster</div>
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
import { getAppSettings, setAppSettings } from "@/api";
import { setTheme, THEME_LABELS, THEME_ORDER, themeState } from "@/theme";
import appLogo from "@/assets/logo.png";

const osTheme = useOsTheme();
const isDark = ref(osTheme.value === "dark");
const message = useMessage();

const version = ref("0.1.1");
const settings = ref({ minimizeToTray: true, exportDir: null as string | null });
const settingsLoading = ref(false);

const exportDirDraft = ref("");
const savingExportDir = ref(false);

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
    message.success(dir ? `导出目录已设置为 ${dir}` : "已恢复默认下载目录");
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
