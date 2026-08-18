import { Channel, invoke } from "@tauri-apps/api/core";
import type {
  CommandResult,
  ConnConfig,
  ConnStatusInfo,
  KeyInfo,
  ScanPage,
  ServerInfo,
  ValueView,
} from "@/types";

function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  return invoke<T>(cmd, args);
}

// ============ 连接管理 ============
export const testConnection = (config: ConnConfig) =>
  call<string>("test_connection", { config });
export const connectConnection = (config: ConnConfig) =>
  call<string>("connect_connection", { config });
export const disconnectConnection = (connId: string) =>
  call<void>("disconnect_connection", { connId });
export const disconnectAll = () => call<void>("disconnect_all");
export const connectionStatus = (connId: string) =>
  call<ConnStatusInfo>("connection_status", { connId });
export const listConnections = () => call<ConnStatusInfo[]>("list_connections");
export const switchDatabase = (connId: string, database: number) =>
  call<void>("switch_database", { connId, database });
export const loadSavedConnections = () => call<ConnConfig[]>("load_saved_connections");
export const saveConnections = (connections: ConnConfig[]) =>
  call<void>("save_connections", { connections });
export const exportConnectionsPick = (filename: string) =>
  call<string | null>("export_connections_pick", { filename });
export const importConnectionsPick = () =>
  call<{ imported: number; duplicated: number } | null>("import_connections_pick");
export const updateTrayMenu = () => call<void>("update_tray_menu");

// ============ 应用设置 ============
export interface AppSettings {
  minimizeToTray: boolean;
}
export const getAppSettings = () => call<AppSettings>("get_app_settings");
export const setAppSettings = (settings: AppSettings) =>
  call<void>("set_app_settings", { settings });

/** 同步语言到后端（供托盘重建多语言菜单），失败静默 */
export const setLocale = (locale: string) => call<void>("set_locale", { locale }).catch(() => {});

// ============ 外部链接 ============
/** 使用系统默认浏览器打开外部链接（项目主页 / Issue 等） */
export const openUrl = (url: string) => call<void>("open_external", { url });

// ============ 键操作 ============
export const scanKeys = (
  connId: string,
  cursor: string,
  pattern: string,
  count?: number,
  typeFilter?: string,
  replica?: number
) => call<ScanPage>("scan_keys", { connId, cursor, pattern, count, typeFilter, replica });
export const keyInfo = (connId: string, key: string, replica?: number) =>
  call<KeyInfo>("key_info", { connId, key, replica });
export const getValue = (connId: string, key: string, replica?: number) =>
  call<ValueView>("get_value", { connId, key, replica });
export const deleteKeys = (connId: string, keys: string[]) =>
  call<number>("delete_keys", { connId, keys });
export const renameKey = (connId: string, source: string, dest: string) =>
  call<void>("rename_key", { connId, source, dest });
/** 创建键（kind: string|hash|list|set|zset|stream；field 用于 hash/stream，score 用于 zset） */
export const createKey = (
  connId: string,
  key: string,
  kind: string,
  field: string,
  value: string,
  score: number,
) =>
  call<void>("create_key", { connId, key, kind, field, value, score });
export const setTtl = (connId: string, key: string, ttl: number) =>
  call<void>("set_ttl", { connId, key, ttl });
export const flushDb = (connId: string) => call<void>("flush_db", { connId });

// ============ 值编辑 ============
export const setStringValue = (
  connId: string,
  key: string,
  value: string,
  ttl: number | null,
  encoding: string
) => call<void>("set_string_value", { connId, key, value, ttl, encoding });
export const setHashField = (connId: string, key: string, field: string, value: string) =>
  call<void>("set_hash_field", { connId, key, field, value });
export const deleteHashFields = (connId: string, key: string, fields: string[]) =>
  call<number>("delete_hash_fields", { connId, key, fields });
export const pushList = (connId: string, key: string, values: string[], tail: boolean) =>
  call<number>("push_list", { connId, key, values, tail });
export const removeList = (connId: string, key: string, value: string, count: number) =>
  call<number>("remove_list", { connId, key, value, count });
export const addSetMembers = (connId: string, key: string, members: string[]) =>
  call<number>("add_set_members", { connId, key, members });
export const removeSetMembers = (connId: string, key: string, members: string[]) =>
  call<number>("remove_set_members", { connId, key, members });
export const addZsetMembers = (connId: string, key: string, members: [number, string][]) =>
  call<number>("add_zset_members", { connId, key, members });
export const removeZsetMembers = (connId: string, key: string, members: string[]) =>
  call<number>("remove_zset_members", { connId, key, members });
export const xaddStream = (
  connId: string,
  key: string,
  id: string,
  fields: [string, string][]
) => call<string>("xadd_stream", { connId, key, id, fields });
export const xdelStream = (connId: string, key: string, ids: string[]) =>
  call<number>("xdel_stream", { connId, key, ids });

// ============ 命令行 ============
export const executeCommand = (connId: string, command: string, replica?: number) =>
  call<CommandResult>("execute_command", { connId, command, replica });

// ============ 服务器信息 ============
export const getServerInfo = (connId: string, section?: string | null) =>
  call<ServerInfo>("get_server_info", { connId, section });
export const getServerConfig = (connId: string, pattern: string) =>
  call<[string, string][]>("get_server_config", { connId, pattern });
export const setServerConfig = (connId: string, key: string, value: string) =>
  call<void>("set_server_config", { connId, key, value });
export const getClients = (connId: string) =>
  call<Record<string, string>[]>("get_clients", { connId });
export const getSlowlog = (connId: string, count?: number | null) =>
  call<Record<string, unknown>[]>("get_slowlog", { connId, count });
export const getTopology = (connId: string) =>
  call<import("@/types").NodeStatus[]>("get_topology", { connId });
export const listDatabases = (connId: string) => call<number[]>("list_databases", { connId });

// ============ 实时监控 ============
export function makeChannel<T>(onEvent: (e: T) => void): Channel<T> {
  const ch = new Channel<T>();
  ch.onmessage = onEvent;
  return ch;
}

export const pubsubSubscribe = (
  connId: string,
  channels: string[],
  patterns: string[],
  onEvent: (e: unknown) => void
) => {
  const ch = makeChannel<unknown>(onEvent);
  return call<void>("pubsub_subscribe", { connId, channels, patterns, onEvent: ch });
};
export const pubsubUnsubscribe = (connId: string, channels: string[], patterns: string[]) =>
  call<void>("pubsub_unsubscribe", { connId, channels, patterns });
export const pubsubPublish = (connId: string, channel: string, message: string) =>
  call<number>("pubsub_publish", { connId, channel, message });
export const startMonitor = (connId: string, onEvent: (e: unknown) => void) => {
  const ch = makeChannel<unknown>(onEvent);
  return call<void>("start_monitor", { connId, onEvent: ch });
};
export const stopTasks = (connId: string) => call<void>("stop_tasks", { connId });
