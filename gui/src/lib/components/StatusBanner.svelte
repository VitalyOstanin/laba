<script lang="ts">
  import { t } from "../i18n";
  import { friendlyError } from "../friendly-error";

  let { error = null }: { error?: string | null } = $props();

  const friendly = $derived(error ? friendlyError(error, $t) : null);
</script>

{#if friendly}
  <div class="banner error" role="alert">
    {$t("error.prefix")}: {friendly.text}
    {#if friendly.notSignedIn}
      <a class="banner-link" href="/settings">{$t("nav.settings")}</a>
    {/if}
  </div>
{/if}
