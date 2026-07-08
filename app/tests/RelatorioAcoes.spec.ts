// US7 — ações do cabeçalho-resumo da Busca: "Baixar relatório" (HTML
// consolidado) e "Baixar ZIP" (PDFs já baixados, US4), ambas via IPC
// mockado (Constituição VII: nenhum teste toca rede/disco reais). Cobre:
// botões desabilitados sem `resultado`/sem itens (R2 pode zerar `categorias`
// mesmo com `total > 0`); clique chama o IPC certo com os dados certos;
// estado "processando" desabilita o botão da própria ação sem travar a
// outra; erro aparece via `role="alert"` (`mensagemDeErro`), sem travar o
// botão.
//
// `UTooltip` (Reka UI) exige um `TooltipProvider` ancestral — mesmo padrão
// de `DocItem.spec.ts`.
import { flushPromises, mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { TooltipProvider } from "reka-ui";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { defineComponent, h } from "vue";

import RelatorioAcoes from "@/components/busca/RelatorioAcoes.vue";
import * as ipc from "@/services/ipc";
import type { ResultadoView } from "@/services/ipc";
import { useBuscaStore } from "@/stores/busca";

const SIAPE = "1998547"; // fictício, sem PII (Constituição II/LGPD).

function resultadoCom(overrides: Partial<ResultadoView> = {}): ResultadoView {
  return {
    termo: SIAPE,
    total: 1,
    tem_pdf: false,
    categorias: [
      {
        categoria: "Progressão",
        qtd: 1,
        itens: [
          {
            titulo: "PORTARIA Nº 1 - 2024 - Progressão",
            data: "10/01/2024",
            link: "https://gedoc.ifes.edu.br/documento/aaaa?inline",
            arquivo: "2024_1_Progressao.pdf",
            resumo: "Determina a progressão do servidor.",
          },
        ],
      },
    ],
    ...overrides,
  };
}

// `comIa` reflete se a busca atual foi no modo IA — o relatório (US7) só é
// habilitado nesse caso (consolida os resumos da IA). Default `true` para os
// testes que exercitam o relatório habilitado.
function montar(resultado: ResultadoView | null, comIa = true) {
  useBuscaStore().resultadoComIa = comIa;
  const Host = defineComponent({
    setup() {
      return () =>
        h(TooltipProvider, null, { default: () => h(RelatorioAcoes, { resultado }) });
    },
  });
  return mount(Host);
}

function botoes(wrapper: ReturnType<typeof montar>) {
  // Ordem no template: relatório · baixar todos os PDFs · ZIP.
  const [relatorio, todos, zip] = wrapper.findAll("button");
  return { relatorio, todos, zip };
}

describe("RelatorioAcoes", () => {
  beforeEach(() => {
    setActivePinia(createPinia()); // RelatorioAcoes usa a store de busca (US #22)
  });

  it("sem resultado, os dois botões ficam desabilitados", () => {
    const wrapper = montar(null);

    const { relatorio, zip } = botoes(wrapper);
    expect(relatorio.attributes("disabled")).toBeDefined();
    expect(zip.attributes("disabled")).toBeDefined();
  });

  it("resultado sem nenhum item (categorias vazias) mantém os botões desabilitados", () => {
    const wrapper = montar(resultadoCom({ categorias: [] }));

    const { relatorio, zip } = botoes(wrapper);
    expect(relatorio.attributes("disabled")).toBeDefined();
    expect(zip.attributes("disabled")).toBeDefined();
  });

  it("com itens, os dois botões ficam habilitados e são alvos acessíveis (>= 40px, Constituição XII)", () => {
    const wrapper = montar(resultadoCom());

    const { relatorio, zip } = botoes(wrapper);
    expect(relatorio.attributes("disabled")).toBeUndefined();
    expect(zip.attributes("disabled")).toBeUndefined();
    expect(relatorio.classes()).toContain("alvo-minimo");
    expect(zip.classes()).toContain("alvo-minimo");
  });

  it("busca sem IA (keyword) mantém o relatório desabilitado; ZIP segue habilitado (US7)", () => {
    const wrapper = montar(resultadoCom(), false);

    const { relatorio, zip } = botoes(wrapper);
    expect(relatorio.attributes("disabled")).toBeDefined();
    expect(zip.attributes("disabled")).toBeUndefined();
  });

  it("clicar em 'Baixar relatório' chama gerarRelatorio com o resultado atual", async () => {
    const espiao = vi.spyOn(ipc, "gerarRelatorio").mockResolvedValue("1998547_relatorio.html");
    const resultado = resultadoCom();
    const wrapper = montar(resultado);

    await botoes(wrapper).relatorio.trigger("click");
    await flushPromises();

    expect(espiao).toHaveBeenCalledWith(resultado);
  });

  it("clicar em 'Baixar ZIP' chama baixarZip com o SIAPE do resultado", async () => {
    const espiao = vi.spyOn(ipc, "baixarZip").mockResolvedValue("1998547_documentos.zip");
    const wrapper = montar(resultadoCom());

    await botoes(wrapper).zip.trigger("click");
    await flushPromises();

    expect(espiao).toHaveBeenCalledWith(SIAPE);
  });

  it("clicar em 'Baixar todos os PDFs' baixa cada documento do resultado (US #22)", async () => {
    const espiao = vi.spyOn(ipc, "baixarDocumento").mockResolvedValue("a.pdf");
    const wrapper = montar(resultadoCom());

    await botoes(wrapper).todos.trigger("click");
    await flushPromises();

    expect(espiao).toHaveBeenCalledWith({
      siape: SIAPE,
      link: "https://gedoc.ifes.edu.br/documento/aaaa?inline",
      titulo: "PORTARIA Nº 1 - 2024 - Progressão",
      data: "10/01/2024",
    });
  });

  it("mostra 'Gerando...' e desabilita só o botão do relatório enquanto ele está em andamento", async () => {
    vi.spyOn(ipc, "gerarRelatorio").mockImplementation(() => new Promise(() => {}));
    const wrapper = montar(resultadoCom());

    await botoes(wrapper).relatorio.trigger("click");

    const { relatorio, zip } = botoes(wrapper);
    expect(relatorio.text()).toContain("Gerando...");
    expect(relatorio.attributes("disabled")).toBeDefined();
    expect(zip.attributes("disabled")).toBeUndefined();
  });

  it("mostra 'Baixando...' e desabilita só o botão do ZIP enquanto ele está em andamento", async () => {
    vi.spyOn(ipc, "baixarZip").mockImplementation(() => new Promise(() => {}));
    const wrapper = montar(resultadoCom());

    await botoes(wrapper).zip.trigger("click");

    const { relatorio, zip } = botoes(wrapper);
    expect(zip.text()).toContain("Baixando...");
    expect(zip.attributes("disabled")).toBeDefined();
    expect(relatorio.attributes("disabled")).toBeUndefined();
  });

  it("erro ao gerar o relatório mostra mensagem (role=alert) sem travar o botão", async () => {
    vi.spyOn(ipc, "gerarRelatorio").mockRejectedValue({
      tipo: "FalhaArquivo",
      mensagem: { motivo: "disco cheio" },
    });
    const wrapper = montar(resultadoCom());

    await botoes(wrapper).relatorio.trigger("click");
    await flushPromises();

    const alerta = wrapper.find('[role="alert"]');
    expect(alerta.exists()).toBe(true);
    expect(alerta.text()).toContain("disco cheio");
    expect(botoes(wrapper).relatorio.attributes("disabled")).toBeUndefined();
  });

  it("erro ao baixar o zip (ex.: nenhum PDF baixado) mostra mensagem amigável", async () => {
    vi.spyOn(ipc, "baixarZip").mockRejectedValue({
      tipo: "FalhaArquivo",
      mensagem: { motivo: "Nenhum PDF baixado para este SIAPE ainda." },
    });
    const wrapper = montar(resultadoCom());

    await botoes(wrapper).zip.trigger("click");
    await flushPromises();

    const alerta = wrapper.find('[role="alert"]');
    expect(alerta.exists()).toBe(true);
    expect(alerta.text()).toContain("Nenhum PDF baixado para este SIAPE ainda.");
    expect(botoes(wrapper).zip.attributes("disabled")).toBeUndefined();
  });
});
