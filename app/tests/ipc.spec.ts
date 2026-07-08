// Testa o adaptador dual-mode `ipc.ts` no MODO WEB. Em jsdom, `window` existe
// sem `__TAURI_INTERNALS__`, então `emTauri()` é false e todas as funções caem
// no branch `fetch` da API. Cobre US1/US3/US5/US6/US7 (lado frontend).

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import * as ipc from "@/services/ipc";

function okJson(data: unknown) {
  return { ok: true, status: 200, json: async () => data } as unknown as Response;
}
function erroJson(status: number, payload: unknown) {
  return { ok: false, status, json: async () => payload } as unknown as Response;
}

beforeEach(() => {
  vi.stubGlobal("fetch", vi.fn());
});
afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

describe("buscarPorSiape (web)", () => {
  it("faz POST /api/buscar com credentials e devolve o resultado", async () => {
    const resultado = { termo: "1998547", total: 0, categorias: [], tem_pdf: false };
    (fetch as unknown as ReturnType<typeof vi.fn>).mockResolvedValue(okJson(resultado));

    const r = await ipc.buscarPorSiape({ siape: "1998547" });

    expect(r).toEqual(resultado);
    const [url, init] = (fetch as unknown as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(String(url)).toContain("/api/buscar");
    expect(init.method).toBe("POST");
    expect(init.credentials).toBe("include");
    expect(JSON.parse(init.body)).toEqual({ siape: "1998547" });
  });

  it("rejeita com o payload AppError quando a resposta não é ok", async () => {
    const payload = { tipo: "SiapeInvalido", mensagem: { termo: "abc" } };
    (fetch as unknown as ReturnType<typeof vi.fn>).mockResolvedValue(erroJson(400, payload));

    await expect(ipc.buscarPorSiape({ siape: "abc" })).rejects.toEqual(payload);
    // A mensagem amigável reusa o mesmo mapeamento do desktop.
    expect(ipc.mensagemDeErro(payload)).toContain("SIAPE inválido");
  });
});

describe("documentos (web)", () => {
  it("baixarDocumento devolve o nome do arquivo", async () => {
    (fetch as unknown as ReturnType<typeof vi.fn>).mockResolvedValue(okJson({ arquivo: "x.pdf" }));
    const nome = await ipc.baixarDocumento({
      siape: "1998547",
      link: "http://x",
      titulo: "t",
      data: null,
    });
    expect(nome).toBe("x.pdf");
  });

  it("abrirDocumento abre a URL da API em nova aba", async () => {
    const open = vi.fn();
    vi.stubGlobal("open", open);
    await ipc.abrirDocumento({ siape: "1998547", arquivo: "x.pdf" });
    expect(open).toHaveBeenCalledTimes(1);
    expect(String(open.mock.calls[0][0])).toContain("/api/documento/1998547/x.pdf");
  });
});

describe("categorias (web)", () => {
  it("listarCategorias faz GET /api/categorias", async () => {
    (fetch as unknown as ReturnType<typeof vi.fn>).mockResolvedValue(okJson([{ nome: "A" }]));
    const cats = await ipc.listarCategorias();
    expect(cats).toEqual([{ nome: "A" }]);
    expect(String((fetch as any).mock.calls[0][0])).toContain("/api/categorias");
  });

  it("salvarCategorias faz PUT /api/categorias", async () => {
    (fetch as unknown as ReturnType<typeof vi.fn>).mockResolvedValue(okJson({ ok: true, total: 2 }));
    const r = await ipc.salvarCategorias([{ nome: "A" }, { nome: "B" }]);
    expect(r).toEqual({ ok: true, total: 2 });
    const [, init] = (fetch as any).mock.calls[0];
    expect(init.method).toBe("PUT");
  });
});

describe("baixarZip (web)", () => {
  it("rejeita com AppError amigável quando não há PDFs", async () => {
    const payload = { tipo: "FalhaArquivo", mensagem: { motivo: "Nenhum PDF." } };
    (fetch as unknown as ReturnType<typeof vi.fn>).mockResolvedValue(erroJson(500, payload));
    await expect(ipc.baixarZip("1998547")).rejects.toEqual(payload);
  });
});
