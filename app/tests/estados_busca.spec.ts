// US2/T018 — estados de UI da busca: forçar cada estado da store e verificar
// que a BuscaView renderiza o componente base correto (loading/vazio/erro) e
// o resultado no estado de sucesso. Feedback claro por estado (Constituição XII).
import { mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { beforeEach, describe, expect, it } from "vitest";

import EmptyState from "@/components/base/EmptyState.vue";
import ErrorState from "@/components/base/ErrorState.vue";
import LoadingState from "@/components/base/LoadingState.vue";
import { useBuscaStore } from "@/stores/busca";
import BuscaView from "@/views/BuscaView.vue";

describe("BuscaView — estados de UI (US2)", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it("estado 'loading' mostra o LoadingState e nada de erro/resultado", async () => {
    const wrapper = mount(BuscaView);
    const store = useBuscaStore();
    store.estado = "loading";
    await wrapper.vm.$nextTick();

    expect(wrapper.findComponent(LoadingState).exists()).toBe(true);
    expect(wrapper.findComponent(ErrorState).exists()).toBe(false);
    expect(wrapper.find(".busca__resultado").exists()).toBe(false);
  });

  it("estado 'erro' mostra o ErrorState com a mensagem", async () => {
    const wrapper = mount(BuscaView);
    const store = useBuscaStore();
    store.estado = "erro";
    store.erro = "Falha ao comunicar com o portal GeDoc: tempo esgotado";
    await wrapper.vm.$nextTick();

    expect(wrapper.findComponent(ErrorState).exists()).toBe(true);
    expect(wrapper.text()).toContain("Falha ao comunicar com o portal GeDoc");
    expect(wrapper.findComponent(LoadingState).exists()).toBe(false);
  });

  it("resultado com total 0 mostra o EmptyState (vazio)", async () => {
    const wrapper = mount(BuscaView);
    const store = useBuscaStore();
    store.estado = "resultado";
    store.resultado = { termo: "1998547", total: 0, categorias: [], tem_pdf: false };
    await wrapper.vm.$nextTick();

    expect(wrapper.findComponent(EmptyState).exists()).toBe(true);
    expect(wrapper.find(".busca__resultado").exists()).toBe(false);
  });

  // Nota: o estado de sucesso (lista com DocItem) é coberto por DocItem.spec.ts
  // e BuscaView.spec.ts — aqui o foco de T018 são os estados loading/vazio/erro.
});
