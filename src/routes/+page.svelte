<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import { t } from "$lib/i18n/index.svelte";
  import SessionOverlay from "$lib/components/SessionOverlay.svelte";
  import Header from "$lib/components/Header.svelte";
  import PinPromptModal from "$lib/components/PinPromptModal.svelte";
  import StockWorkspace from "$lib/components/StockWorkspace.svelte";
  import SellWorkspace from "$lib/components/SellWorkspace.svelte";
  import HistoryView from "./history/+page.svelte";
  import SettingsView from "./settings/+page.svelte";
  import PatientsWorkspace from "$lib/components/PatientsWorkspace.svelte";
  import InvoicesWorkspace from "$lib/components/InvoicesWorkspace.svelte";
  import LoansWorkspace from "$lib/components/LoansWorkspace.svelte";
  import { permissionsStore, type CashierPermissions } from "$lib/permissions.svelte";
  import cartIcon from "../public/icons/cart.png";
  import historyIcon from "../public/icons/history.png";
  import settingsIcon from "../public/icons/settings.png";
  import stockIcon from "../public/icons/stock.png";
  import peopleIcon from "../public/icons/people.png";
  import pdfIcon from "../public/icons/pdf.png";
  import loanIcon from "../public/icons/loan.png";

  // Session State
  let userRole = $state<"cashier" | "admin" | null>(null);
  let showPinPrompt = $state(false);
  let pendingAdminAction = $state<(() => void) | null>(null);

  // Active navigation tab
  let activeTab = $state<"sell" | "stock" | "invoices" | "loans" | "history" | "patients" | "settings">("sell");

  // Inactivity tracking
  let inactivityTimeout: any;

  function resetInactivityTimer() {
    if (userRole === "admin") {
      clearTimeout(inactivityTimeout);
      inactivityTimeout = setTimeout(() => {
        userRole = "cashier";
        alert(t("inactivityLock"));
      }, 5 * 60 * 1000); // 5 minutes
    }
  }

  // React to userRole changes to setup inactivity listeners
  $effect(() => {
    if (userRole === "admin") {
      resetInactivityTimer();
      const events = ["mousedown", "mousemove", "keypress", "scroll", "touchstart"];
      const handler = () => resetInactivityTimer();
      
      events.forEach(e => window.addEventListener(e, handler));
      return () => {
        events.forEach(e => window.removeEventListener(e, handler));
        clearTimeout(inactivityTimeout);
      };
    } else {
      clearTimeout(inactivityTimeout);
    }
  });

  function handleUnlockSuccess() {
    userRole = "admin";
    showPinPrompt = false;
    if (pendingAdminAction) {
      pendingAdminAction();
      pendingAdminAction = null;
    }
  }

  function handleUnlockCancel() {
    showPinPrompt = false;
    pendingAdminAction = null;
  }

  // Request admin actions
  function triggerAdminAction(action: () => void) {
    if (userRole === "admin") {
      action();
    } else {
      pendingAdminAction = action;
      showPinPrompt = true;
    }
  }

  function navigateTab(tab: "sell" | "stock" | "invoices" | "loans" | "history" | "patients" | "settings", permKey?: keyof CashierPermissions) {
    if (!permKey || permissionsStore.isAllowed(userRole, permKey)) {
      activeTab = tab;
    } else {
      triggerAdminAction(() => {
        activeTab = tab;
      });
    }
  }

  function openSettings() {
    triggerAdminAction(() => {
      activeTab = "settings";
    });
  }

  function openHistory() {
    navigateTab("history", "access_history");
  }

  let showBackupAlert = $state(false);

  async function checkBackupStatus() {
    try {
      const settings = await invoke<any>("get_settings");
      showBackupAlert = shouldShowBackupAlert(settings.last_backup_at);
    } catch (err) {
      console.error("Failed to load settings in backup check:", err);
    }
  }

  function shouldShowBackupAlert(lastBackupStr: string | null): boolean {
    if (!lastBackupStr) return true;
    const lastBackup = new Date(lastBackupStr.replace(/-/g, "/"));
    const now = new Date();
    const diffTime = now.getTime() - lastBackup.getTime();
    const diffDays = diffTime / (1000 * 60 * 60 * 24);
    return diffDays >= 7;
  }

  async function triggerBackup() {
    try {
      const msg = await invoke<string>("backup_database");
      alert(msg);
      showBackupAlert = false;
    } catch (err: any) {
      alert(err.toString());
    }
  }

  onMount(() => {
    permissionsStore.load();
    checkBackupStatus();

    const handleKeyDown = (e: KeyboardEvent) => {
      // Ctrl+H -> History
      if (e.ctrlKey && e.key.toLowerCase() === "h") {
        e.preventDefault();
        openHistory();
      }
      // Ctrl+, -> Settings
      if (e.ctrlKey && e.key === ",") {
        e.preventDefault();
        openSettings();
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  });
</script>

{#if userRole === null}
    <!-- Session Start Selection -->
    <SessionOverlay onSessionStarted={(role) => userRole = role} />
  {:else}
  <!-- Header status bar -->
  <Header 
    {userRole} 
    onUnlockAdmin={() => showPinPrompt = true} 
    onLockAdmin={() => userRole = "cashier"} 
  />

  {#if showBackupAlert}
    <div class="backup-alert-banner">
      <span>{t("backupReminder")}</span>
      <div class="backup-actions">
        <button onclick={triggerBackup} class="btn-action btn-action-primary btn-sm">
          {t("backupNow")}
        </button>
        <button onclick={() => showBackupAlert = false} class="btn-close-banner" aria-label="Close">×</button>
      </div>
    </div>
  {/if}

  <div class="workspace-layout">
    <!-- Big Traditional POS Navigation Buttons (Elderly friendly) -->
    <aside class="sidebar-nav">
      <button 
        class="btn-pos {activeTab === 'sell' ? 'active' : ''}" 
        onclick={() => navigateTab('sell')}
      >
        <img src={cartIcon} alt="Cart" class="btn-icon" />
        <span class="btn-label">{t("sell")}</span>
      </button>

      <button 
        class="btn-pos {activeTab === 'stock' ? 'active' : ''}" 
        onclick={() => navigateTab('stock', 'access_stock')}
      >
        <img src={stockIcon} alt="Stock" class="btn-icon" />
        <span class="btn-label">{t("stock")}</span>
      </button>

      <!-- Invoices -->
      <button 
        class="btn-pos {activeTab === 'invoices' ? 'active' : ''}" 
        onclick={() => navigateTab('invoices', 'access_invoices')}
      >
        <img src={pdfIcon} alt="Factures" class="btn-icon" />
        <span class="btn-label">{t("invoices")}</span>
      </button>

      <!-- History -->
      <button 
        onclick={openHistory} 
        class="btn-pos {activeTab === 'history' ? 'active' : ''}"
      >
        <img src={historyIcon} alt="History" class="btn-icon" />
        <span class="btn-label">{t("history")}</span>
      </button>

      <!-- Loans -->
      <button 
        class="btn-pos {activeTab === 'loans' ? 'active' : ''}" 
        onclick={() => navigateTab('loans', 'access_loans')}
      >
        <img src={loanIcon} alt="Loans" class="btn-icon" />
        <span class="btn-label">{t("loans")}</span>
      </button>

      <!-- Patients -->
      <button 
        onclick={() => navigateTab('patients', 'access_patients')} 
        class="btn-pos {activeTab === 'patients' ? 'active' : ''}"
      >
        <img src={peopleIcon} alt="Patients" class="btn-icon" />
        <span class="btn-label">{t("patients")}</span>
      </button>
 
      <!-- Settings -->
      <button 
        onclick={openSettings} 
        class="btn-pos {activeTab === 'settings' ? 'active' : ''}"
      >
        <img src={settingsIcon} alt="Settings" class="btn-icon" />
        <span class="btn-label">{t("settings")}</span>
      </button>
    </aside>

    <!-- Main Workspace Pane -->
    <main class="workspace-pane" class:no-scroll={activeTab === "sell" || activeTab === "patients" || activeTab === "invoices"}>
      {#if activeTab === "sell"}
        <SellWorkspace {userRole} triggerAdminPIN={triggerAdminAction} />
      {:else if activeTab === "stock"}
        <StockWorkspace {userRole} triggerAdminPIN={triggerAdminAction} />
      {:else if activeTab === "invoices"}
        <InvoicesWorkspace {userRole} />
      {:else if activeTab === "loans"}
        <LoansWorkspace {userRole} triggerAdminPIN={triggerAdminAction} />
      {:else if activeTab === "history"}
        <HistoryView isAdmin={userRole === "admin"} />
      {:else if activeTab === "patients"}
        <PatientsWorkspace {userRole} triggerAdminPIN={triggerAdminAction} />
      {:else if activeTab === "settings"}
        <SettingsView />
      {/if}
    </main>
  </div>
  {/if}

{#if showPinPrompt}
  <PinPromptModal 
    onVerified={handleUnlockSuccess} 
    onCancel={handleUnlockCancel} 
  />
{/if}

<style>
  .workspace-layout {
    display: flex;
    flex: 1;
    min-height: 0;
    height: 100%;
    overflow: hidden;
  }

  /* Traditional Side Toolbar common in retail POS */
  .sidebar-nav {
    background-color: var(--color-bg-card);
    border-inline-end: var(--border-width) solid var(--color-border);
    width: clamp(76px, 8.5vw, 125px);
    padding: clamp(0.4rem, 1vh, 1rem) clamp(0.2rem, 0.4vw, 0.5rem);
    display: flex;
    flex-direction: column;
    gap: clamp(0.35rem, 0.8vh, 0.85rem);
    align-items: center;
    overflow-y: auto;
    flex-shrink: 0;
    height: 100%;
    position: sticky;
    top: 0;
    z-index: 10;
  }

  .sidebar-nav :global(.btn-pos) {
    width: 100%;
    min-height: clamp(62px, 9vh, 105px);
    height: clamp(62px, 9vh, 105px);
    padding: clamp(0.25rem, 0.5vh, 0.6rem) 0.25rem;
    flex-shrink: 0;
    gap: 0.35rem;
  }

  .sidebar-nav :global(.btn-pos.active) {
    border-color: var(--color-primary);
    background-color: #e3fcef;
  }

  .sidebar-nav :global(.btn-icon) {
    width: clamp(30px, 4.8vh, 64px);
    height: clamp(30px, 4.8vh, 64px);
    object-fit: contain;
  }

  .sidebar-nav :global(.btn-pos .btn-label) {
    font-size: clamp(0.72rem, 0.85vw, 0.95rem);
    line-height: 1.1;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 100%;
  }

  @media (max-height: 750px) {
    .sidebar-nav :global(.btn-pos) {
      min-height: 58px;
      height: 58px;
    }
    .sidebar-nav :global(.btn-icon) {
      width: 28px;
      height: 28px;
    }
  }

  @media (max-width: 900px) {
    .sidebar-nav {
      width: 76px;
    }
  }

  .workspace-pane {
    flex: 1;
    min-height: 0;
    height: 100%;
    padding: clamp(0.6rem, 1.4vw, 1.75rem);
    overflow-y: auto;
  }

  .workspace-pane.no-scroll {
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  .backup-actions {
    display: flex;
    align-items: center;
    gap: 1.5rem;
  }

  .btn-close-banner {
    background: none;
    border: none;
    font-size: 1.8rem;
    font-weight: bold;
    color: var(--color-text-dark);
    cursor: pointer;
    line-height: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0;
    transition: transform 0.15s ease;
  }

  .btn-close-banner:hover {
    transform: scale(1.2);
  }
</style>
