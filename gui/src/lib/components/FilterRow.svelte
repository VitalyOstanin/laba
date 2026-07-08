<script lang="ts">
  import { filterText } from "../store";
  import { t } from "../i18n";

  let { count }: { count: number } = $props();
</script>

<div class="filterbar">
  <input
    type="search"
    aria-label={$t("filter.placeholder")}
    placeholder={$t("filter.placeholder")}
    bind:value={$filterText}
  />
  <span class="filtercount">{count}</span>
</div>
