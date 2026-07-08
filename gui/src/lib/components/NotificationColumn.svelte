<script lang="ts">
  import { t } from "../i18n";
  import type { Notification } from "../types";

  let { notifications = [] }: { notifications?: Notification[] } = $props();
</script>

<section class="card" aria-label={$t("col.notifications")}>
  <header><h2>{$t("col.notifications")}</h2></header>
  {#if notifications.length === 0}
    <p class="empty">{$t("empty.notifications")}</p>
  {:else}
    <ul class="list">
      {#each notifications as n (n.id)}
        <li class="notif">
          <span class="reason">{n.reason}</span>
          <span class="subject">{n.subject}</span>
        </li>
      {/each}
    </ul>
  {/if}
</section>
