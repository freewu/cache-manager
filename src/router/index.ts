import { createRouter, createWebHashHistory } from "vue-router";

const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    { path: "/", name: "connections", component: () => import("@/views/Connections.vue") },
    { path: "/explorer/:id", name: "explorer", component: () => import("@/views/Explorer.vue") },
    { path: "/console/:id", name: "console", component: () => import("@/views/Console.vue") },
    { path: "/server/:id", name: "server", component: () => import("@/views/ServerInfo.vue") },
    { path: "/monitor/:id", name: "monitor", component: () => import("@/views/Monitor.vue") },
    { path: "/settings", name: "settings", component: () => import("@/views/Settings.vue") },
    { path: "/:pathMatch(.*)*", redirect: "/" },
  ],
});

export default router;
