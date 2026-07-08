<script lang="ts">
  import { servers, activeServer, byServer } from "../store";
</script>

<div class="server-switch" role="group" aria-label="Active server">
  {#each $servers as s (s.name)}
    <button
      type="button"
      aria-current={$activeServer === s.name}
      title={`${s.name} · ${s.base_url}`}
      onclick={() => activeServer.set(s.name)}
    >
      <span class="dot" class:err={$byServer[s.name]?.error}></span>
      {s.name}
      <span class="bk {s.backend === 'github' ? 'gh' : 'op'}">
        {s.backend === "github" ? "GH" : "OP"}
      </span>
    </button>
  {/each}
</div>
