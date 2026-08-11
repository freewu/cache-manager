import { defineStore } from "pinia";
import type { ConnConfig, ConnStatusInfo } from "@/types";
import * as api from "@/api";

export const useConnectionStore = defineStore("connections", {
  state: () => ({
    connections: [] as ConnStatusInfo[],
    saved: [] as ConnConfig[],
    loading: false,
  }),

  getters: {
    connectedIds: (state) =>
      state.connections.filter((c) => c.status === "connected").map((c) => c.id),
    byId: (state) => (id: string) => state.connections.find((c) => c.id === id),
  },

  actions: {
    async init() {
      await this.loadSaved();
      await this.refresh();
    },

    async loadSaved() {
      try {
        this.saved = await api.loadSavedConnections();
      } catch (e) {
        console.error("加载连接配置失败", e);
      }
    },

    async refresh() {
      try {
        this.connections = await api.listConnections();
      } catch (e) {
        console.error("获取连接状态失败", e);
      }
    },

    async persist() {
      try {
        await api.saveConnections(this.saved);
        // 同步托盘快速连接菜单
        await api.updateTrayMenu();
      } catch (e) {
        console.error("保存连接配置失败", e);
      }
    },

    /** 新建或更新连接配置（先测试，成功后连接） */
    async saveAndConnect(config: ConnConfig): Promise<void> {
      const idx = this.saved.findIndex((c) => c.id === config.id);
      if (idx >= 0) this.saved[idx] = config;
      else this.saved.push(config);
      await this.persist();
      await this.connect(config);
    },

    async connect(config: ConnConfig) {
      await api.connectConnection(config);
      await this.refresh();
    },

    /** 仅测试连通性，不建立连接 */
    async test(config: ConnConfig) {
      await api.testConnection(config);
    },

    async connectSaved(id: string) {
      const cfg = this.saved.find((c) => c.id === id);
      if (!cfg) throw new Error("连接配置不存在");
      await this.connect(cfg);
    },

    async disconnect(id: string) {
      await api.disconnectConnection(id);
      await this.refresh();
    },

    async disconnectAll() {
      await api.disconnectAll();
      await this.refresh();
    },

    async removeSaved(id: string) {
      await api.disconnectConnection(id);
      this.saved = this.saved.filter((c) => c.id !== id);
      await this.persist();
      await this.refresh();
    },
  },
});
