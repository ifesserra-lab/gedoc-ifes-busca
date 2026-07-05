// Setup global do Vitest — registra o plugin Vue do Nuxt UI (o mesmo de
// `main.ts`) em todos os testes de componente, para que `<UButton>`,
// `<UInput>`, etc. montem sem avisos de "componente não registrado".
import ui from "@nuxt/ui/vue-plugin";
import { config } from "@vue/test-utils";

config.global.plugins.push(ui);
