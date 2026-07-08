import { createRouter, createWebHistory } from "vue-router";

import { emTauri } from "@/services/ipc";
import BuscaView from "@/views/BuscaView.vue";
import CategoriasView from "@/views/CategoriasView.vue";

export const router = createRouter({
  history: createWebHistory(),
  routes: [
    { path: "/", name: "busca", component: BuscaView },
    {
      path: "/categorias",
      name: "categorias",
      component: CategoriasView,
      // Gestão de categorias só no desktop (spec 005). Na web, o acesso direto
      // à rota redireciona para a busca.
      beforeEnter: () => (emTauri() ? true : { name: "busca" }),
    },
  ],
});
