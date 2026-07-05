// Revisão do PR #13 — CategoriaChips não tinha teste. Cobre: contagem
// (incluindo "Todos"), emissão de seleção ao clicar, e destaque
// (aria-pressed) do chip selecionado.
import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";

import CategoriaChips from "@/components/busca/CategoriaChips.vue";
import type { CategoriaGrupo } from "@/services/ipc";

const grupos: CategoriaGrupo[] = [
  { categoria: "Portaria", qtd: 2, itens: [] },
  { categoria: "Diária", qtd: 1, itens: [] },
];

describe("CategoriaChips", () => {
  it("renderiza 'Todos' com a soma e cada categoria com sua contagem", () => {
    const wrapper = mount(CategoriaChips, { props: { grupos, selecionada: null } });

    expect(wrapper.text()).toContain("Todos (3)");
    expect(wrapper.text()).toContain("Portaria (2)");
    expect(wrapper.text()).toContain("Diária (1)");
  });

  it("emite 'selecionar' com o nome da categoria ao clicar num chip", async () => {
    const wrapper = mount(CategoriaChips, { props: { grupos, selecionada: null } });

    const botoes = wrapper.findAll(".categoria-chips__item");
    await botoes[1].trigger("click"); // 0 = Todos, 1 = Portaria

    expect(wrapper.emitted("selecionar")?.[0]).toEqual(["Portaria"]);
  });

  it("emite null ao clicar em 'Todos'", async () => {
    const wrapper = mount(CategoriaChips, { props: { grupos, selecionada: "Portaria" } });

    const botoes = wrapper.findAll(".categoria-chips__item");
    await botoes[0].trigger("click");

    expect(wrapper.emitted("selecionar")?.[0]).toEqual([null]);
  });

  it("destaca (aria-pressed=true) somente a categoria selecionada", () => {
    const wrapper = mount(CategoriaChips, { props: { grupos, selecionada: "Diária" } });

    const botoes = wrapper.findAll(".categoria-chips__item");
    expect(botoes[0].attributes("aria-pressed")).toBe("false"); // Todos
    expect(botoes[1].attributes("aria-pressed")).toBe("false"); // Portaria
    expect(botoes[2].attributes("aria-pressed")).toBe("true"); // Diária
  });

  it("sem seleção, 'Todos' fica marcada como pressionada", () => {
    const wrapper = mount(CategoriaChips, { props: { grupos, selecionada: null } });

    const botoes = wrapper.findAll(".categoria-chips__item");
    expect(botoes[0].attributes("aria-pressed")).toBe("true");
  });
});
