// 与后端 DTO 对应的类型定义（camelCase）

export type ConnMode = "single" | "masterSlave" | "sentinel" | "cluster" | "memcached";
export type ConnStatusKind = "disconnected" | "connecting" | "connected" | "error";

export interface NodeSpec {
  host: string;
  port: number;
}

export interface ConnConfig {
  id: string;
  name: string;
  mode: ConnMode;
  host: string;
  port: number;
  username?: string | null;
  password?: string | null;
  database?: number | null;
  nodes: NodeSpec[];
  serviceName?: string | null;
  tls: boolean;
  connectTimeoutMs: number;
}

export interface NodeStatus {
  host: string;
  port: number;
  role: string;
  status: string;
  extra?: string | null;
}

export interface ConnStatusInfo {
  id: string;
  name: string;
  mode: ConnMode;
  status: ConnStatusKind;
  error?: string | null;
  nodes: NodeStatus[];
  databases?: number[] | null;
  connectedAt?: number | null;
}

export interface ScanPage {
  cursor: string;
  keys: string[];
  truncated: boolean;
  /** key -> 类型 */
  types: Record<string, string>;
}

export interface KeyInfo {
  key: string;
  type: string;
  ttl: number;
  pttl: number;
  length: number;
  memory?: number | null;
  encoding?: string | null;
}

export interface EncodedValue {
  encoding: "utf8" | "base64";
  value: string;
}

export interface HashField {
  field: EncodedValue;
  value: EncodedValue;
}

export interface ZMember {
  member: EncodedValue;
  score: number;
}

export interface StreamEntry {
  id: string;
  fields: HashField[];
}

export type ValuePayload =
  | { kind: "string"; value: EncodedValue }
  | { kind: "list"; values: EncodedValue[]; truncated: boolean }
  | { kind: "hash"; fields: HashField[]; truncated: boolean }
  | { kind: "set"; values: EncodedValue[]; truncated: boolean }
  | { kind: "zset"; members: ZMember[]; truncated: boolean }
  | { kind: "stream"; entries: StreamEntry[]; truncated: boolean };

export interface ValueView {
  key: string;
  type: string;
  ttl: number;
  length: number;
  payload: ValuePayload;
}

export interface CommandResult {
  command: string;
  elapsedMs: number;
  ok: boolean;
  error?: string | null;
  value?: unknown | null;
  text?: string | null;
}

export interface InfoSection {
  name: string;
  fields: [string, string][];
}

export interface ServerInfo {
  sections: InfoSection[];
  raw: string;
}

export interface ClientEntry {
  id: string;
  addr: string;
  name: string;
  age: string;
  idle: string;
  flags: string;
  db: string;
  sub: string;
  cmd: string;
  user: string;
  resp: string;
  other: [string, string][];
}

export interface SlowlogEntry {
  id: number;
  timestamp: number;
  durationUs: number;
  args: string[];
  clientAddr: string;
  clientName: string;
}

export interface PubSubEvent {
  kind: "message" | "pmessage" | "smessage";
  channel: string;
  text?: string | null;
  json?: unknown | null;
  server: string;
}

export interface MonitorEvent {
  timestamp: number;
  db: number;
  client: string;
  command: string;
  args: string[];
  raw: string;
}
