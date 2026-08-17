import { ref } from "vue";
import { getVersion } from "@tauri-apps/api/app";
import { openUrl } from "@/api";

/** 新版本检查状态 */
export const updateState = ref<{
  checking: boolean;
  checked: boolean;
  available: boolean;
  latest?: string;
  current?: string;
}>({
  checking: false,
  checked: false,
  available: false,
});

const REPO = "freewu/cache-manager";

/** 比较版本号，a > b 返回 true */
function isNewer(a: string, b: string): boolean {
  const na = a.replace(/^v/, "").split(".").map((n) => parseInt(n, 10) || 0);
  const nb = b.replace(/^v/, "").split(".").map((n) => parseInt(n, 10) || 0);
  const len = Math.max(na.length, nb.length);
  for (let i = 0; i < len; i++) {
    const x = na[i] ?? 0;
    const y = nb[i] ?? 0;
    if (x > y) return true;
    if (x < y) return false;
  }
  return false;
}

/** 获取当前应用版本（Tauri 环境） */
export async function getAppVersion(): Promise<string> {
  try {
    return await getVersion();
  } catch {
    // 浏览器环境回退到打包时注入的版本
    return (import.meta as any).env?.APP_VERSION ?? "0.0.0";
  }
}

/** 开启一条异步连接 */
export const openReleases = () => openUrl(`https://github.com/${REPO}/releases`);

/** 异步检查是否发布新版本（结果写入 updateState） */
export async function checkForUpdate(): Promise<void> {
  if (updateState.value.checking) return;
  updateState.value.checking = true;
  try {
    const current = await getAppVersion();
    let latest = "";
    // 优先使用 GitHub API
    try {
      const res = await fetch(`https://api.github.com/repos/${REPO}/releases/latest`);
      if (res.ok) {
        const data = (await res.json()) as { tag_name?: string };
        latest = data.tag_name ?? "";
      }
    } catch {
      latest = "";
    }
    updateState.value.current = current;
    updateState.value.latest = latest || undefined;
    updateState.value.available = !!latest && isNewer(latest, current);
    updateState.value.checked = true;
  } finally {
    updateState.value.checking = false;
  }
}

/** 手动点击 badge 时跳转并更新状态 */
export function openUpdatePage() {
  openReleases();
  // 点击后隐藏 badge（避免重复提醒）
  updateState.value.available = false;
}
