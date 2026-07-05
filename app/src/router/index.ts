import { createRouter, createWebHistory } from "vue-router";

import BuscaView from "@/views/BuscaView.vue";

export const router = createRouter({
  history: createWebHistory(),
  routes: [{ path: "/", name: "busca", component: BuscaView }],
});
