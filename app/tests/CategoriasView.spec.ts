// T012/US8 — CRUD de categorias (R5): o modal de criação abre a partir da
// View; a validação de nome vazio/duplicado (regra de negócio) vive na store
// e é testada diretamente (sem depender do timing assíncrono do UForm
// interno do Nuxt UI). Persistência real via IPC (`listar_categorias`/
// `salvar_categorias`, US8) — mockada aqui via `services/ipc.ts`.
import { flushPromises, mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { nextTick } from "vue";

import * as ipc from "@/services/ipc";
import { useCategoriasStore } from "@/stores/categorias";
import CategoriasView from "@/views/CategoriasView.vue";

describe("categoriasStore — validação R5 e persistência via IPC", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.restoreAllMocks();
  });

  it("rejeita nome vazio ao salvar sem chamar o IPC", async () => {
    const store = useCategoriasStore();
    const espiao = vi.spyOn(ipc, "salvarCategorias");

    const erro = await store.salvar({ nome: "   ", descricao: "" }, null);

    expect(erro).toBe("Informe um nome para a categoria.");
    expect(store.itens).toHaveLength(0);
    expect(espiao).not.toHaveBeenCalled();
  });

  it("rejeita nome duplicado (case-insensitive) ao salvar", async () => {
    vi.spyOn(ipc, "salvarCategorias").mockResolvedValue({ ok: true, total: 1 });
    const store = useCategoriasStore();
    await store.salvar({ nome: "Portaria", descricao: "" }, null);

    const erro = await store.salvar({ nome: "portaria", descricao: "outra" }, null);

    expect(erro).toContain("já existe");
    expect(store.itens).toHaveLength(1);
  });

  it("permite editar mantendo o próprio nome (não conta como duplicata)", async () => {
    vi.spyOn(ipc, "salvarCategorias").mockResolvedValue({ ok: true, total: 1 });
    const store = useCategoriasStore();
    await store.salvar({ nome: "Portaria", descricao: "" }, null);

    const erro = await store.salvar({ nome: "Portaria", descricao: "atualizada" }, 0);

    expect(erro).toBeNull();
    expect(store.itens[0].descricao).toBe("atualizada");
  });

  it("salva com sucesso um nome válido e único, persistindo a lista completa via IPC", async () => {
    const espiao = vi.spyOn(ipc, "salvarCategorias").mockResolvedValue({ ok: true, total: 1 });
    const store = useCategoriasStore();

    const erro = await store.salvar({ nome: "Diária", descricao: "Diárias de viagem" }, null);

    expect(erro).toBeNull();
    expect(store.itens).toHaveLength(1);
    expect(store.mensagemSucesso).toBe("Categoria criada.");
    expect(espiao).toHaveBeenCalledWith([{ nome: "Diária", descricao: "Diárias de viagem" }]);
  });

  it("mensagem amigável quando o backend rejeita algo que passou na validação client-side", async () => {
    vi.spyOn(ipc, "salvarCategorias").mockRejectedValue({
      tipo: "NomeDuplicado",
      mensagem: { nome: "Diária" },
    });
    const store = useCategoriasStore();

    const erro = await store.salvar({ nome: "Diária", descricao: "" }, null);

    expect(erro).toContain("já existe");
    expect(store.itens).toHaveLength(0);
    expect(store.erro).toBe(erro);
  });

  it("remove uma categoria e persiste a lista resultante via IPC", async () => {
    vi.spyOn(ipc, "salvarCategorias").mockResolvedValue({ ok: true, total: 1 });
    const store = useCategoriasStore();
    await store.salvar({ nome: "Portaria", descricao: "" }, null);
    const espiao = vi.spyOn(ipc, "salvarCategorias");

    await store.remover(0);

    expect(store.itens).toHaveLength(0);
    expect(espiao).toHaveBeenCalledWith([]);
    expect(store.mensagemSucesso).toBe("Categoria removida.");
  });

  it("carrega as categorias existentes do backend", async () => {
    vi.spyOn(ipc, "listarCategorias").mockResolvedValue([
      { nome: "Progressão", descricao: "Progressão funcional." },
      { nome: "Outros", descricao: null },
    ]);
    const store = useCategoriasStore();

    await store.carregar();

    expect(store.estado).toBe("pronto");
    expect(store.itens).toEqual([
      { nome: "Progressão", descricao: "Progressão funcional." },
      { nome: "Outros", descricao: "" },
    ]);
  });

  it("erro ao carregar do backend cai no estado 'erro' com mensagem amigável", async () => {
    vi.spyOn(ipc, "listarCategorias").mockRejectedValue({
      tipo: "FalhaArquivo",
      mensagem: { motivo: "disco cheio" },
    });
    const store = useCategoriasStore();

    await store.carregar();

    expect(store.estado).toBe("erro");
    expect(store.erro).toContain("disco cheio");
  });
});

describe("CategoriasView", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.restoreAllMocks();
  });

  afterEach(() => {
    document.body.innerHTML = "";
  });

  it("mostra o estado vazio com ação de criar quando o backend não tem categorias", async () => {
    vi.spyOn(ipc, "listarCategorias").mockResolvedValue([]);
    const wrapper = mount(CategoriasView, { attachTo: document.body });

    await flushPromises();

    expect(wrapper.text()).toContain("Nenhuma categoria cadastrada");

    wrapper.unmount();
  });

  it("abre o modal de nova categoria ao clicar no botão", async () => {
    vi.spyOn(ipc, "listarCategorias").mockResolvedValue([]);
    const wrapper = mount(CategoriasView, { attachTo: document.body });
    await flushPromises();

    await wrapper.find('[data-testid="nova-categoria"]').trigger("click");
    await nextTick();

    expect(document.body.textContent).toContain("Nova categoria");
    expect(document.querySelector('[role="dialog"]')).not.toBeNull();

    wrapper.unmount();
  });

  it("reflete na tabela uma categoria persistida pela store (View + Store + IPC)", async () => {
    vi.spyOn(ipc, "listarCategorias").mockResolvedValue([]);
    const espiaoSalvar = vi.spyOn(ipc, "salvarCategorias").mockResolvedValue({ ok: true, total: 1 });
    const wrapper = mount(CategoriasView, { attachTo: document.body });
    await flushPromises();
    expect(wrapper.text()).toContain("Nenhuma categoria cadastrada");

    const store = useCategoriasStore();
    await store.salvar({ nome: "Diária", descricao: "Diárias de viagem" }, null);
    await nextTick();

    expect(espiaoSalvar).toHaveBeenCalledWith([{ nome: "Diária", descricao: "Diárias de viagem" }]);
    expect(wrapper.text()).toContain("Diária");
    expect(wrapper.text()).not.toContain("Nenhuma categoria cadastrada");

    wrapper.unmount();
  });
});
