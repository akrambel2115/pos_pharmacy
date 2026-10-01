<script lang="ts">
  import '../app.css';
  import { langState } from '$lib/i18n/index.svelte';
  import { onMount } from 'svelte';
  import UpdateModal from '$lib/components/UpdateModal.svelte';
  import { updaterStore } from '$lib/updater.svelte';

  let { children } = $props();

  onMount(() => {
    // Synchronize language directory with document root on launch
    langState.setLanguage(langState.current);

    // Check for updates silently shortly after startup
    const timer = setTimeout(() => {
      updaterStore.checkForUpdates(true);
    }, 4000);

    return () => clearTimeout(timer);
  });
</script>

<div class="container">
  {@render children()}
</div>

<UpdateModal />

