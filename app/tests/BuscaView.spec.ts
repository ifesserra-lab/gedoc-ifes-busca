// US3 — componente de busca: SIAPE inválido bloqueia a busca com mensagem
// clara, sem round-trip de IPC (R10 aplicado antes de chamar o backend).
import { mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { beforeEach, describe, expect, it, vi } from "vitest";

import * as ipc from "@/services/ipc";
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
});
