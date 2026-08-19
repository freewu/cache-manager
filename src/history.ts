/**
 * 执行历史：每次在命令行执行的指令都会记录，可回溯
 * 持久化到 localStorage（上限 500 条，超出丢弃最旧）
 */

export interface ExecHistoryItem {
  id: string;
  time: number;
  connId: string;
  connName: string;
  mode: string;
  command: string;
  ok: boolean;
  elapsedMs: number;
}

const KEY = "cm.exec.history";
const MAX = 500;

export function addExecHistory(item: Omit<ExecHistoryItem, "id">): void {
  const list = loadExecHistory();
  list.unshift({ ...item, id: `${Date.now()}-${Math.random().toString(36).slice(2, 8)}` });
  try {
    localStorage.setItem(KEY, JSON.stringify(list.slice(0, MAX)));
  } catch {
    // 存储失败（如配额满）静默忽略
  }
}

export function loadExecHistory(): ExecHistoryItem[] {
  try {
    const raw = localStorage.getItem(KEY);
    if (!raw) return [];
    const list = JSON.parse(raw);
    return Array.isArray(list) ? (list as ExecHistoryItem[]) : [];
  } catch {
    return [];
  }
}

export function clearExecHistory(): void {
  localStorage.removeItem(KEY);
}

export function removeExecHistory(id: string): void {
  const list = loadExecHistory().filter((it) => it.id !== id);
  try {
    localStorage.setItem(KEY, JSON.stringify(list));
  } catch {
    // ignore
  }
}
