<script setup lang="ts">
// Casca do app — cabeçalho (marca, navegação, alternador de tema claro/
// escuro) + <UApp> (necessário para overlays/toasts do Nuxt UI). Sem regra
// de negócio: só apresentação/navegação (Constituição V/XII).
import { useColorMode } from "@vueuse/core";
import { computed } from "vue";

import { emTauri } from "@/services/ipc";

// VueUse alterna as classes "dark"/"light" em <html> (ver assets/tokens.css);
// `emitAuto: true` preserva "não escolhido ainda" = segue o sistema.
const colorMode = useColorMode({ emitAuto: true });

const ehEscuro = computed({
  get: () => colorMode.value === "dark",
  set: (valor: boolean) => {
    colorMode.value = valor ? "dark" : "light";
  },
});

// Categorias só no desktop (Tauri): na web a gestão de categorias é removida
// (spec 005) — a classificação segue usando as categorias globais do servidor.
const links = computed(() => [
  { to: "/", label: "Busca" },
  ...(emTauri() ? [{ to: "/categorias", label: "Categorias" }] : []),
]);
</script>

<template>
  <UApp>
    <div class="app-shell">
      <header class="app-header">
        <span class="app-header__marca">GeDoc <span class="app-header__marca-acento">IFES</span></span>

        <nav class="app-header__nav" aria-label="Navegação principal">
          <RouterLink
            v-for="link in links"
            :key="link.to"
            :to="link.to"
            class="app-header__link"
            active-class="app-header__link--ativo"
          >
            {{ link.label }}
          </RouterLink>
        </nav>

        <UButton
          class="app-header__toggle alvo-minimo"
          :icon="ehEscuro ? 'i-lucide-sun' : 'i-lucide-moon'"
          color="neutral"
          variant="ghost"
          size="lg"
          :aria-label="ehEscuro ? 'Mudar para tema claro' : 'Mudar para tema escuro'"
          @click="ehEscuro = !ehEscuro"
        />
      </header>

      <main class="app-main">
        <RouterView />
      </main>
    </div>
  </UApp>
</template>

<style scoped>
.app-shell {
  min-height: 100vh;
  display: flex;
  flex-direction: column;
  background-color: var(--paper);
  color: var(--ink);
}

.app-header {
  position: sticky;
  top: 0;
  z-index: 20;
  display: flex;
  align-items: center;
  gap: var(--sp-6);
  padding: var(--sp-3) var(--sp-6);
  border-bottom: 1px solid var(--border);
  background-color: var(--surface);
}

.app-header__marca {
  font-size: var(--text-16);
  font-weight: 700;
  color: var(--ink);
  white-space: nowrap;
}

.app-header__marca-acento {
  color: var(--accent);
}

.app-header__nav {
  display: flex;
  gap: var(--sp-5);
  flex: 1;
}

.app-header__link {
  font-size: var(--text-14);
  font-weight: 500;
  color: var(--muted);
  text-decoration: none;
  padding: var(--sp-2) var(--sp-1);
  border-bottom: 2px solid transparent;
}

.app-header__link:hover {
  color: var(--ink);
}

.app-header__link--ativo {
  color: var(--accent);
  border-bottom-color: var(--accent);
  font-weight: 600;
}

.app-main {
  flex: 1;
  padding: var(--sp-8) var(--sp-6);
}
</style>
