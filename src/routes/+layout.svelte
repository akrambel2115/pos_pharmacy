<script lang="ts">
  import '../app.css';
  import { langState } from '$lib/i18n/index.svelte';
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import UpdateModal from '$lib/components/UpdateModal.svelte';
  import LicenseGate from '$lib/components/LicenseGate.svelte';
  import { updaterStore } from '$lib/updater.svelte';

  let { children } = $props();

  type GateStatus = 'checking' | 'valid' | 'missing' | 'invalid' | 'expired' | 'clock';
  let licenseStatus = $state<GateStatus>('checking');
  let machineCode = $state('');

  // License is verified once per app launch (not per action)
  async function checkLicense() {
    try {
      const res = await invoke<{ status: GateStatus; machine_code: string }>('check_license');
      machineCode = res.machine_code;
      licenseStatus = res.status;
    } catch (err) {
      console.error(err);
      try { machineCode = await invoke<string>('get_machine_code'); } catch {}
      licenseStatus = 'invalid';
    }
  }

  onMount(() => {
    // Synchronize language directory with document root on launch
    langState.setLanguage(langState.current);

    checkLicense();

    // Check for updates silently shortly after startup
    const timer = setTimeout(() => {
      updaterStore.checkForUpdates(true);
    }, 4000);

    return () => clearTimeout(timer);
  });
</script>

{#if licenseStatus === 'valid'}
  <div class="container">
    {@render children()}
  </div>

  <UpdateModal />
{:else if licenseStatus !== 'checking'}
  <LicenseGate status={licenseStatus} {machineCode} onActivated={checkLicense} />
{/if}

