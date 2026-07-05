// T012 — CRUD de categorias (R5): o modal de criação abre a partir da View;
// a validação de nome vazio/duplicado (regra de negócio) vive na store e é
// testada diretamente (sem depender do timing assíncrono do UForm interno
// do Nuxt UI).
import { mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { nextTick } from "vue";

import { useCategoriasStore } from "@/stores/categorias";
import CategoriasView from "@/views/CategoriasView.vue";

describe("categoriasStore — validação R5", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it("rejeita nome vazio ao salvar", () => {
    const store = useCategoriasStore();

    const erro = store.salvar({ nome: "   ", descricao: "" }, null);

    expect(erro).toBe("Informe um nome para a categoria.");
    expect(store.itens).toHaveLength(0);
  });

  it("rejeita nome duplicado (case-insensitive) ao salvar", () => {
    const store = useCategoriasStore();
    store.salvar({ nome: "Portaria", descricao: "" }, null);

    const erro = store.salvar({ nome: "portaria", descricao: "outra" }, null);

    expect(erro).toContain("já existe");
    expect(store.itens).toHaveLength(1);
  });

  it("permite editar mantendo o próprio nome (não conta como duplicata)", () => {
    const store = useCategoriasStore();
    store.salvar({ nome: "Portaria", descricao: "" }, null);

    const erro = store.salvar({ nome: "Portaria", descricao: "atualizada" }, 0);

    expect(erro).toBeNull();
    expect(store.itens[0].descricao).toBe("atualizada");
  });

  it("salva com sucesso um nome válido e único", () => {
    const store = useCategoriasStore();

    const erro = store.salvar({ nome: "Diária", descricao: "Diárias de viagem" }, null);

    expect(erro).toBeNull();
    expect(store.itens).toHaveLength(1);
    expect(store.mensagemSucesso).toBe("Categoria criada.");
  });
});

describe("CategoriasView", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  afterEach(() => {
    document.body.innerHTML = "";
  });

  it("mostra o estado vazio com ação de criar quando não há categorias", () => {
    const wrapper = mount(CategoriasView, { attachTo: document.body });

    expect(wrapper.text()).toContain("Nenhuma categoria cadastrada");

    wrapper.unmount();
  });

  it("abre o modal de nova categoria ao clicar no botão", async () => {
    const wrapper = mount(CategoriasView, { attachTo: document.body });

    await wrapper.find('[data-testid="nova-categoria"]').trigger("click");
    await nextTick();

    expect(document.body.textContent).toContain("Nova categoria");
    expect(document.querySelector('[role="dialog"]')).not.toBeNull();

    wrapper.unmount();
  });
});
