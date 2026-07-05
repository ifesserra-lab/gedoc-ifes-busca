<script setup lang="ts">
// View (US3) — campo SIAPE + estados busca/loading/erro/resultado.
// Nenhuma regra de negócio aqui: tudo delega à store (ViewModel).
import { useBuscaStore } from "@/stores/busca";

const store = useBuscaStore();
</script>

<template>
  <section class="busca">
    <h1>GeDoc IFES — Consulta por SIAPE</h1>

    <form class="busca__form" @submit.prevent="store.buscar()">
      <label for="siape">Matrícula SIAPE</label>
      <input
        id="siape"
        v-model="store.siape"
        type="text"
        inputmode="numeric"
        autocomplete="off"
        placeholder="Ex.: 1998547"
        :aria-invalid="store.estado === 'erro'"
      />
      <button type="submit" :disabled="store.estado === 'loading'">
        {{ store.estado === "loading" ? "Buscando..." : "Buscar" }}
      </button>
    </form>

    <p v-if="store.estado === 'erro'" class="busca__erro" role="alert">
      {{ store.erro }}
    </p>

    <p v-else-if="store.estado === 'loading'" role="status">Buscando documentos...</p>

    <div v-else-if="store.estado === 'resultado' && store.resultado" class="busca__resultado">
      <p>
        Total: <strong>{{ store.resultado.total }}</strong> documento(s) para o SIAPE
        <strong>{{ store.resultado.termo }}</strong>
      </p>

      <div v-for="grupo in store.resultado.categorias" :key="grupo.categoria" class="busca__grupo">
        <h2>{{ grupo.categoria }} ({{ grupo.qtd }})</h2>
        <ul>
          <li v-for="item in grupo.itens" :key="item.link">
            <strong>{{ item.titulo }}</strong>
            <span v-if="item.data"> — {{ item.data }}</span>
            <p v-if="item.resumo">{{ item.resumo }}</p>
          </li>
        </ul>
      </div>
    </div>
  </section>
</template>
