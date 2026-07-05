import { createRouter, createWebHistory } from "vue-router";

import BuscaView from "@/views/BuscaView.vue";
import CategoriasView from "@/views/CategoriasView.vue";

export const router = createRouter({
  history: createWebHistory(),
  routes: [
    { path: "/", name: "busca", component: BuscaView },
    { path: "/categorias", name: "categorias", component: CategoriasView },
  ],
});
