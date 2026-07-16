// US3 — componente de busca: SIAPE inválido bloqueia a busca com mensagem
// clara, sem round-trip de IPC (R10 aplicado antes de chamar o backend).
// US6 — toggle "Classificar e resumir com IA": off por padrão, liga
// `store.usarIa` e é refletido no `modo` enviado ao backend.
import { mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { beforeEach, describe, expect, it, vi } from "vitest";

import * as ipc from "@/services/ipc";
import { useBuscaStore } from "@/stores/busca";
import BuscaView from "@/views/BuscaView.vue";

describe("BuscaView", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it("bloqueia a busca e mostra mensagem quando o SIAPE é inválido", async () => {
    const espiao = vi.spyOn(ipc, "buscarPorSiape");

    const wrapper = mount(BuscaView);
    await wrapper.find("#siape").setValue("abc");
    await wrapper.find("form").trigger("submit");

    expect(wrapper.find('[role="alert"]').exists()).toBe(true);
    expect(wrapper.text()).toContain("SIAPE válido");
    expect(espiao).not.toHaveBeenCalled();
  });

  it("não mostra erro nem resultado antes de qualquer busca", () => {
    const wrapper = mount(BuscaView);
    expect(wrapper.find('[role="alert"]').exists()).toBe(false);
    expect(wrapper.find(".busca__resultado").exists()).toBe(false);
  });

  it("toggle de IA começa desligado e tem um alvo clicável acessível (>= 40px, Constituição XII)", () => {
    const wrapper = mount(BuscaView);

    const toggle = wrapper.get("#usar-ia");
    expect(toggle.attributes("aria-checked")).toBe("false");
    expect(toggle.attributes("id")).toBe("usar-ia");
    // "alvo-minimo" (min. 40px) fica no agrupamento switch+rótulo — o texto
    // ao lado também ativa o switch (`<label for>`), então a área clicável
    // combinada, não só o track do switch, é o alvo relevante.
    expect(wrapper.get(".busca__toggle-ia").classes()).toContain("alvo-minimo");
    expect(wrapper.find('label[for="usar-ia"]').exists()).toBe(true);
    expect(wrapper.text()).toContain("Classificar e resumir com IA");
  });

  it("clicar no toggle liga usarIa na store", async () => {
    const wrapper = mount(BuscaView);
    const store = useBuscaStore();

    await wrapper.get("#usar-ia").trigger("click");

    expect(store.usarIa).toBe(true);
    expect(wrapper.get("#usar-ia").attributes("aria-checked")).toBe("true");
  });

  it("com o toggle ligado, a busca envia modo 'llm' ao backend", async () => {
    const espiao = vi
      .spyOn(ipc, "buscarPorSiape")
      .mockResolvedValue({ termo: "1998547", total: 0, categorias: [], tem_pdf: false });
    const wrapper = mount(BuscaView);

    await wrapper.get("#usar-ia").trigger("click");
    await wrapper.find("#siape").setValue("1998547");
    await wrapper.find("form").trigger("submit");

    expect(espiao).toHaveBeenCalledWith({ siape: "1998547", modo: "llm", por: "siape" });
  });

  it("modo nome: ligar o toggle 'por nome' aceita termo livre e envia por 'nome' (spec 009)", async () => {
    const espiao = vi
      .spyOn(ipc, "buscarPorSiape")
      .mockResolvedValue({ termo: "joão silva", total: 0, categorias: [], tem_pdf: false });
    const wrapper = mount(BuscaView);
    const store = useBuscaStore();

    await wrapper.get("#por-nome").trigger("click");
    expect(store.porNome).toBe(true);

    await wrapper.find("#siape").setValue("joão silva");
    await wrapper.find("form").trigger("submit");

    expect(espiao).toHaveBeenCalledWith({ siape: "joão silva", modo: "keyword", por: "nome" });
  });
});
