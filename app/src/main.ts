import ui from "@nuxt/ui/vue-plugin";
import { createPinia } from "pinia";
import { createApp } from "vue";

import "./assets/main.css";

import App from "./App.vue";
import { router } from "./router";

createApp(App).use(createPinia()).use(router).use(ui).mount("#app");
