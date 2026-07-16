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

  it("usarIa começa desligado e busca envia modo 'keyword' por padrão (US6)", async () => {
    const store = useBuscaStore();
    const espiao = vi
      .spyOn(ipc, "buscarPorSiape")
      .mockResolvedValue(mockResultado({ total: 0, categorias: [] }));
    store.siape = "1998547";

    expect(store.usarIa).toBe(false);
    await store.buscar();

    expect(espiao).toHaveBeenCalledWith({ siape: "1998547", modo: "keyword", por: "siape" });
  });

  it("com usarIa ligado, busca envia modo 'llm' (US6)", async () => {
    const store = useBuscaStore();
    const espiao = vi
      .spyOn(ipc, "buscarPorSiape")
      .mockResolvedValue(mockResultado({ total: 0, categorias: [] }));
    store.siape = "1998547";
    store.usarIa = true;

    await store.buscar();

    expect(espiao).toHaveBeenCalledWith({ siape: "1998547", modo: "llm", por: "siape" });
  });

  it("modo nome: termo não-SIAPE é válido e busca envia por 'nome' (spec 009)", async () => {
    const store = useBuscaStore();
    const espiao = vi
      .spyOn(ipc, "buscarPorSiape")
      .mockResolvedValue(mockResultado({ total: 0, categorias: [] }));
    store.porNome = true;
    store.siape = "joão silva";

    expect(store.consultaValida).toBe(true);
    await store.buscar();

    expect(store.estado).not.toBe("erro");
    expect(espiao).toHaveBeenCalledWith({ siape: "joão silva", modo: "keyword", por: "nome" });
  });

  it("modo nome: termo vazio é inválido e não busca", async () => {
    const store = useBuscaStore();
    const espiao = vi.spyOn(ipc, "buscarPorSiape");
    store.porNome = true;
    store.siape = "   ";

    await store.buscar();

    expect(store.estado).toBe("erro");
    expect(espiao).not.toHaveBeenCalled();
  });

  it("modo SIAPE (padrão): termo não-SIAPE é inválido e não busca", async () => {
    const store = useBuscaStore();
    const espiao = vi.spyOn(ipc, "buscarPorSiape");
    store.siape = "joão";

    await store.buscar();

    expect(store.estado).toBe("erro");
    expect(espiao).not.toHaveBeenCalled();
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

describe("useBuscaStore — baixar todos os PDFs (US #22)", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  function docView(link: string) {
    return { titulo: `Doc ${link}`, data: "10/01/2024", link, arquivo: null, resumo: null };
  }

  it("baixa cada documento uma vez, com o SIAPE informado", async () => {
    const store = useBuscaStore();
    const espiao = vi.spyOn(ipc, "baixarDocumento").mockResolvedValue("arquivo.pdf");

    const resumo = await store.baixarTodos([docView("l1"), docView("l2")], "1998547");

    expect(resumo).toEqual({ ok: 2, falhas: 0 });
    expect(espiao).toHaveBeenCalledTimes(2);
    expect(espiao).toHaveBeenCalledWith({ siape: "1998547", link: "l1", titulo: "Doc l1", data: "10/01/2024" });
    expect(store.downloadProgresso).toBeNull(); // zerado ao fim
  });

  it("falha em um documento não aborta o lote (R11)", async () => {
    const store = useBuscaStore();
    vi.spyOn(ipc, "baixarDocumento")
      .mockResolvedValueOnce("a.pdf")
      .mockRejectedValueOnce({ tipo: "FalhaPortal", mensagem: { motivo: "timeout" } })
      .mockResolvedValueOnce("c.pdf");

    const resumo = await store.baixarTodos([docView("l1"), docView("l2"), docView("l3")], "1998547");

    expect(resumo).toEqual({ ok: 2, falhas: 1 });
  });

  it("no-op quando não há documentos ou SIAPE", async () => {
    const store = useBuscaStore();
    const espiao = vi.spyOn(ipc, "baixarDocumento");

    expect(await store.baixarTodos([], "1998547")).toEqual({ ok: 0, falhas: 0 });
    expect(await store.baixarTodos([docView("l1")], "")).toEqual({ ok: 0, falhas: 0 });
    expect(espiao).not.toHaveBeenCalled();
  });

  it("expõe progresso durante o download (atual/total)", async () => {
    const store = useBuscaStore();
    const progressos: Array<{ atual: number; total: number } | null> = [];
    vi.spyOn(ipc, "baixarDocumento").mockImplementation(async () => {
      progressos.push(store.downloadProgresso ? { ...store.downloadProgresso } : null);
      return "x.pdf";
    });

    await store.baixarTodos([docView("l1"), docView("l2")], "1998547");

    // Durante o 1º download, o total já é conhecido (2) e atual ainda 0.
    expect(progressos[0]).toEqual({ atual: 0, total: 2 });
    expect(store.baixandoTodos).toBe(false); // encerrado
  });
});
