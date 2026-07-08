<script lang="ts">
  import { t } from "../i18n";

  let { error = null }: { error?: string | null } = $props();
</script>

{#if error}
  <div class="banner error" role="alert">{$t("error.prefix")}: {error}</div>
{/if}
