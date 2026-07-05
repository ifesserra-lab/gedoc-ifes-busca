// T009 — os três componentes-base de estado (Constituição XII: cinco
// estados sempre) renderizam corretamente. Sem regra de negócio: só
// verificamos apresentação (texto, roles/aria) a partir das props.
import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";

import EmptyState from "@/components/base/EmptyState.vue";
import ErrorState from "@/components/base/ErrorState.vue";
import LoadingState from "@/components/base/LoadingState.vue";

describe("LoadingState", () => {
  it("anuncia o carregamento e desenha as linhas de skeleton", () => {
    const wrapper = mount(LoadingState, { props: { label: "Buscando...", linhas: 4 } });

    expect(wrapper.find('[role="status"]').attributes("aria-label")).toBe("Buscando...");
    expect(wrapper.findAll(".loading-state__linha").length).toBe(4);
  });

  it("usa valores padrão quando nenhuma prop é informada", () => {
    const wrapper = mount(LoadingState);

    expect(wrapper.text()).toContain("Carregando...");
  });
});

describe("EmptyState", () => {
  it("mostra título e descrição", () => {
    const wrapper = mount(EmptyState, {
      props: {
        titulo: "Nenhum documento para este SIAPE",
        descricao: "Verifique o número informado ou tente outro SIAPE.",
      },
    });

    expect(wrapper.text()).toContain("Nenhum documento para este SIAPE");
    expect(wrapper.text()).toContain("Verifique o número informado");
  });

  it("expõe uma ação de próximo passo via slot, quando fornecida", () => {
    const wrapper = mount(EmptyState, {
      props: { titulo: "Vazio" },
      slots: { default: "<button>Nova busca</button>" },
    });

    expect(wrapper.find("button").exists()).toBe(true);
  });
});

describe("ErrorState", () => {
  it("mostra a mensagem de erro com role=alert e botão de tentar novamente", async () => {
    const wrapper = mount(ErrorState, {
      props: { mensagem: "Falha ao comunicar com o portal GeDoc." },
    });

    expect(wrapper.find('[role="alert"]').exists()).toBe(true);
    expect(wrapper.text()).toContain("Falha ao comunicar com o portal GeDoc.");

    await wrapper.find("button").trigger("click");
    expect(wrapper.emitted("retry")).toHaveLength(1);
  });

  it("esconde o botão de retry quando permiteRetry=false", () => {
    const wrapper = mount(ErrorState, {
      props: { mensagem: "Erro inesperado.", permiteRetry: false },
    });

    expect(wrapper.find("button").exists()).toBe(false);
  });
});
