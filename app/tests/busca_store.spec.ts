// Revisão do PR #13 — cobre a lógica de UI que vive na store `busca`
// (Constituição VII/TDD): filtro por chip (`gruposFiltrados`/
// `categoriaSelecionada`) e o estado "vazio" (US2/T021), que ainda não
// tinham teste.
import { createPinia, setActivePinia } from "pinia";
import { beforeEach, describe, expect, it, vi } from "vitest";

import * as ipc from "@/services/ipc";
import { useBuscaStore } from "@/stores/busca";

function mockResultado(overrides: Partial<Awaited<ReturnType<typeof ipc.buscarPorSiape>>> = {}) {
  return {
    termo: "1998547",
    total: 0,
    tem_pdf: false,
    categorias: [],
    ...overrides,
  };
}

describe("useBuscaStore — filtro por categoria e estado vazio", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it("sem seleção, gruposFiltrados mostra todos os grupos ('Todas')", async () => {
    const store = useBuscaStore();
    vi.spyOn(ipc, "buscarPorSiape").mockResolvedValue(
      mockResultado({
        total: 3,
        categorias: [
          { categoria: "Portaria", qtd: 2, itens: [] },
          { categoria: "Diária", qtd: 1, itens: [] },
        ],
      }),
    );
    store.siape = "1998547";

    await store.buscar();

    expect(store.categoriaSelecionada).toBeNull();
    expect(store.gruposFiltrados).toHaveLength(2);
  });

  it("selecionar uma categoria filtra gruposFiltrados para só ela", async () => {
    const store = useBuscaStore();
    vi.spyOn(ipc, "buscarPorSiape").mockResolvedValue(
      mockResultado({
        total: 3,
        categorias: [
          { categoria: "Portaria", qtd: 2, itens: [] },
          { categoria: "Diária", qtd: 1, itens: [] },
        ],
      }),
    );
    store.siape = "1998547";
    await store.buscar();

    store.selecionarCategoria("Diária");

    expect(store.categoriaSelecionada).toBe("Diária");
    expect(store.gruposFiltrados).toHaveLength(1);
    expect(store.gruposFiltrados[0].categoria).toBe("Diária");
  });

  it("limpar a seleção (null) volta a mostrar todos os grupos", async () => {
    const store = useBuscaStore();
    vi.spyOn(ipc, "buscarPorSiape").mockResolvedValue(
      mockResultado({
        total: 2,
        categorias: [
          { categoria: "Portaria", qtd: 1, itens: [] },
          { categoria: "Diária", qtd: 1, itens: [] },
        ],
      }),
    );
    store.siape = "1998547";
    await store.buscar();
    store.selecionarCategoria("Portaria");
    expect(store.gruposFiltrados).toHaveLength(1);

    store.selecionarCategoria(null);

    expect(store.categoriaSelecionada).toBeNull();
    expect(store.gruposFiltrados).toHaveLength(2);
  });

  it("uma nova busca reinicia a categoria selecionada", async () => {
    const store = useBuscaStore();
    vi.spyOn(ipc, "buscarPorSiape").mockResolvedValue(
      mockResultado({ total: 1, categorias: [{ categoria: "Portaria", qtd: 1, itens: [] }] }),
    );
    store.siape = "1998547";
    await store.buscar();
    store.selecionarCategoria("Portaria");

    await store.buscar();

    expect(store.categoriaSelecionada).toBeNull();
  });

  it("resultado com total 0 fica no estado 'vazio' (distinto de erro)", async () => {
    const store = useBuscaStore();
    vi.spyOn(ipc, "buscarPorSiape").mockResolvedValue(mockResultado({ total: 0, categorias: [] }));
    store.siape = "1998547";

    await store.buscar();

    expect(store.estado).toBe("resultado");
    expect(store.vazio).toBe(true);
    expect(store.gruposFiltrados).toHaveLength(0);
  });

  it("resultado com documentos não fica 'vazio'", async () => {
    const store = useBuscaStore();
    vi.spyOn(ipc, "buscarPorSiape").mockResolvedValue(
      mockResultado({ total: 1, categorias: [{ categoria: "Portaria", qtd: 1, itens: [] }] }),
    );
    store.siape = "1998547";

    await store.buscar();

    expect(store.vazio).toBe(false);
  });

  it("reiniciar limpa a categoria selecionada junto com o restante do estado", async () => {
    const store = useBuscaStore();
    vi.spyOn(ipc, "buscarPorSiape").mockResolvedValue(
      mockResultado({ total: 1, categorias: [{ categoria: "Portaria", qtd: 1, itens: [] }] }),
    );
    store.siape = "1998547";
    await store.buscar();
    store.selecionarCategoria("Portaria");

    store.reiniciar();

    expect(store.categoriaSelecionada).toBeNull();
    expect(store.estado).toBe("idle");
    expect(store.resultado).toBeNull();
  });
});
