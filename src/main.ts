import { createApp } from "vue";
import { createPinia } from "pinia";
import naive from "naive-ui";
import App from "./App.vue";
import router from "./router";

const app = createApp(App);
app.use(createPinia());
app.use(router);
// 全局注册 naive-ui 组件（n-layout / n-menu / n-message-provider 等）
app.use(naive);
app.mount("#app");
