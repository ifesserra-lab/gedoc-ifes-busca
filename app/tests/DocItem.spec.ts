// Revisão do PR #13 — DocItem não tinha teste. Cobre: truncamento do
// título longo (mantendo o texto completo acessível via `title`) e o botão
// de PDF permanecendo não-acionável (aria-disabled), porém focável por
// teclado — não usa o atributo nativo `disabled` (Constituição XII: leitor
// de tela/teclado devem alcançar o motivo via UTooltip).
//
// `UTooltip` (Reka UI) exige um `TooltipProvider` ancestral — em produção,
// `UApp` (App.vue) fornece esse contexto; aqui usamos o `TooltipProvider`
// real como host mínimo, para testar o componente de verdade.
import { mount } from "@vue/test-utils";
import { TooltipProvider } from "reka-ui";
import { describe, expect, it } from "vitest";
import { defineComponent, h } from "vue";

import DocItem from "@/components/busca/DocItem.vue";
import type { DocView } from "@/services/ipc";

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

function montarDocItem(doc: DocView, categoria = "Portaria") {
  const Host = defineComponent({
    setup() {
      return () => h(TooltipProvider, null, { default: () => h(DocItem, { doc, categoria }) });
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

  it("mantém o botão de PDF sempre não-acionável (aria-disabled) — download pendente de IPC", () => {
    const wrapper = montarDocItem(criarDoc());

    const botao = wrapper.find("button");
    expect(botao.attributes("aria-disabled")).toBe("true");
  });

  it("não usa o atributo nativo disabled, para permanecer alcançável por teclado", () => {
    const wrapper = montarDocItem(criarDoc());

    const botao = wrapper.find("button");
    expect(botao.attributes("disabled")).toBeUndefined();
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
