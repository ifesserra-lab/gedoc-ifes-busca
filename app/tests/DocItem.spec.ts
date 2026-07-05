// US4 — o botão de PDF passou de permanentemente `aria-disabled` (backend
// ainda não existia) para acionável: clicar baixa (`baixarDocumento`) e
// abre (`abrirDocumento`) o documento com o app padrão do SO. Cobre:
// truncamento do título longo (mantendo o texto completo acessível via
// `title`); botão acionável por padrão; clique dispara as duas chamadas de
// IPC com os dados corretos; estado "baixando" desabilita o botão; falha
// mostra mensagem (`role="alert"`) via `mensagemDeErro`.
//
// `UTooltip` (Reka UI) exige um `TooltipProvider` ancestral — em produção,
// `UApp` (App.vue) fornece esse contexto; aqui usamos o `TooltipProvider`
// real como host mínimo, para testar o componente de verdade.
import { flushPromises, mount } from "@vue/test-utils";
import { TooltipProvider } from "reka-ui";
import { describe, expect, it, vi } from "vitest";
import { defineComponent, h } from "vue";

import DocItem from "@/components/busca/DocItem.vue";
import * as ipc from "@/services/ipc";
import type { DocView } from "@/services/ipc";

const SIAPE = "1998547"; // fictício, sem PII (Constituição II/LGPD).

function criarDoc(sobrescreve: Partial<DocView> = {}): DocView {
  return {
    titulo: "Portaria",
    data: "01/01/2024",
    link: "https://gedoc.ifes.edu.br/doc/1",
    arquivo: null,
    resumo: "Resumo do documento.",
    ...sobrescreve,
  };
}

function montarDocItem(doc: DocView, categoria = "Portaria", siape = SIAPE) {
  const Host = defineComponent({
    setup() {
      return () =>
        h(TooltipProvider, null, { default: () => h(DocItem, { doc, categoria, siape }) });
    },
  });
  return mount(Host);
}

describe("DocItem", () => {
  it("trunca visualmente o título longo, mantendo o texto completo em title", () => {
    const tituloLongo =
      "Portaria de designação de comissão de sindicância administrativa disciplinar número 123456789 do ano de 2024";
    const wrapper = montarDocItem(criarDoc({ titulo: tituloLongo }));

    const titulo = wrapper.find(".doc-item__titulo");
    expect(titulo.attributes("title")).toBe(tituloLongo);
    expect(titulo.text()).toBe(tituloLongo);
  });

  it("o botão de PDF começa acionável — não usa mais aria-disabled fixo", () => {
    const wrapper = montarDocItem(criarDoc());

    const botao = wrapper.find("button");
    expect(botao.attributes("aria-disabled")).toBeUndefined();
    expect(botao.attributes("disabled")).toBeUndefined();
  });

  it("clicar baixa e depois abre o documento com os dados do SIAPE e do documento", async () => {
    const baixar = vi.spyOn(ipc, "baixarDocumento").mockResolvedValue("2024_1_Assunto.pdf");
    const abrir = vi.spyOn(ipc, "abrirDocumento").mockResolvedValue(undefined);
    const doc = criarDoc({
      link: "https://gedoc.ifes.edu.br/documento/aaaa?inline",
      titulo: "PORTARIA Nº 1 - 2024 - Designação de função",
      data: "05/03/2024",
    });
    const wrapper = montarDocItem(doc);

    await wrapper.find("button").trigger("click");
    await flushPromises();

    expect(baixar).toHaveBeenCalledWith({
      siape: SIAPE,
      link: doc.link,
      titulo: doc.titulo,
      data: doc.data,
    });
    expect(abrir).toHaveBeenCalledWith({ siape: SIAPE, arquivo: "2024_1_Assunto.pdf" });
  });

  it("desabilita o botão enquanto o download está em andamento", async () => {
    vi.spyOn(ipc, "baixarDocumento").mockImplementation(() => new Promise(() => {}));
    const wrapper = montarDocItem(criarDoc());

    await wrapper.find("button").trigger("click");

    expect(wrapper.find("button").attributes("disabled")).toBeDefined();
  });

  it("mostra mensagem de erro (role=alert) quando o download falha, sem travar o botão", async () => {
    vi.spyOn(ipc, "baixarDocumento").mockRejectedValue({
      tipo: "FalhaPortal",
      mensagem: { motivo: "portal indisponível" },
    });
    const wrapper = montarDocItem(criarDoc());

    await wrapper.find("button").trigger("click");
    await flushPromises();

    const alerta = wrapper.find('[role="alert"]');
    expect(alerta.exists()).toBe(true);
    expect(alerta.text()).toContain("portal indisponível");
    expect(wrapper.find("button").attributes("disabled")).toBeUndefined();
  });

  it("mostra a data e o resumo quando presentes", () => {
    const wrapper = montarDocItem(criarDoc({ data: "10/02/2024", resumo: "Um resumo qualquer." }));

    expect(wrapper.text()).toContain("10/02/2024");
    expect(wrapper.text()).toContain("Um resumo qualquer.");
  });

  it("mostra a categoria do grupo como pill na meta do documento", () => {
    const wrapper = montarDocItem(criarDoc(), "Diária");

    expect(wrapper.find(".doc-item__pill").text()).toBe("Diária");
  });
});
