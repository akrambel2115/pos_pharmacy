<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { t, langState } from "$lib/i18n/index.svelte";
  import { onMount } from "svelte";
  import { formatDateFr } from "$lib/utils";
  import viewIcon from "../../public/icons/view.png";
  import { permissionsStore, defaultCashierPermissions, type CashierPermissions } from "$lib/permissions.svelte";
  import { updaterStore } from "$lib/updater.svelte";

  // App settings state
  let expiryWarningDays = $state(30);
  let defaultLanguage = $state("fr");
  let lastBackupAt = $state<string | null>(null);
  let pharmacyName = $state("");
  let pharmacyAddress = $state("");

  // Cashier permissions state
  let cashierPerms = $state<CashierPermissions>({ ...defaultCashierPermissions });
  let permsSuccess = $state("");
  let permsError = $state("");

  // Change PIN state
  let oldPin = $state("");
  let newPin = $state("");
  let confirmNewPin = $state("");
  
  let settingsSuccess = $state("");
  let settingsError = $state("");
  let pinSuccess = $state("");
  let pinError = $state("");

  // Gemini AI Key state
  let geminiKey = $state("");
  let showGeminiKey = $state(false);
  let savedGeminiKey = $state("");
  let directAiInvoice = $state(false);
  let aiKeySuccess = $state("");
  let aiKeyError = $state("");
  let isTestingKey = $state(false);

  onMount(async () => {
    try {
      const settings = await invoke<any>("get_settings");
      expiryWarningDays = settings.expiry_warning_days;
      defaultLanguage = settings.default_language;
      lastBackupAt = settings.last_backup_at;
      pharmacyName = settings.pharmacy_name || "";
      pharmacyAddress = settings.pharmacy_address || "";
      directAiInvoice = !!settings.direct_ai_invoice;
      
      // Ensure local translation state is aligned
      langState.setLanguage(defaultLanguage as any);

      // Load permissions
      await permissionsStore.load();
      cashierPerms = { ...permissionsStore.permissions };
    } catch (err: any) {
      settingsError = err.toString();
    }

    try {
      const key = await invoke<string>("get_gemini_api_key");
      savedGeminiKey = key || "";
      geminiKey = key || "";
    } catch (kErr) {
      console.warn("Could not fetch Gemini API key:", kErr);
    }
  });

  async function handleSavePermissions(e: SubmitEvent) {
    e.preventDefault();
    permsSuccess = "";
    permsError = "";
    try {
      const permsJson = JSON.stringify(cashierPerms);
      await invoke("update_settings", {
        expiryWarningDays,
        defaultLanguage,
        cashierPermissions: permsJson,
      });
      permissionsStore.setPermissions(cashierPerms);
      permsSuccess = t("permissionsSavedSuccess");
    } catch (err: any) {
      permsError = err.toString();
    }
  }

  async function handleSaveAiKey(e: SubmitEvent) {
    e.preventDefault();
    aiKeySuccess = "";
    aiKeyError = "";
    try {
      if (geminiKey.trim()) {
        await invoke("save_gemini_api_key", { apiKey: geminiKey.trim() });
        savedGeminiKey = geminiKey.trim();
      }
      await invoke("update_settings", {
        expiryWarningDays,
        defaultLanguage,
        directAiInvoice,
      });
      aiKeySuccess = t("aiKeySavedSuccess");
    } catch (err: any) {
      aiKeyError = err.toString();
    }
  }

  async function handleToggleDirectAi(e: Event) {
    const checked = (e.target as HTMLInputElement).checked;
    directAiInvoice = checked;
    try {
      await invoke("update_settings", {
        expiryWarningDays,
        defaultLanguage,
        directAiInvoice: checked,
      });
      aiKeySuccess = t("aiKeySavedSuccess");
    } catch (err: any) {
      aiKeyError = err.toString();
    }
  }

  async function handleDeleteAiKey() {
    if (!confirm("Voulez-vous vraiment supprimer la clé API IA ? / هل أنت متأكد من حذف مفتاح API؟")) {
      return;
    }
    aiKeySuccess = "";
    aiKeyError = "";
    try {
      await invoke("delete_gemini_api_key");
      geminiKey = "";
      savedGeminiKey = "";
      aiKeySuccess = t("aiKeyDeletedSuccess");
    } catch (err: any) {
      aiKeyError = err.toString();
    }
  }

  async function handleTestAiKey() {
    if (!geminiKey.trim()) {
      aiKeyError = "Veuillez saisir une clé API à tester.";
      return;
    }
    isTestingKey = true;
    aiKeySuccess = "";
    aiKeyError = "";
    try {
      const msg = await invoke<string>("test_gemini_api_key", { apiKey: geminiKey.trim() });
      aiKeySuccess = msg;
    } catch (err: any) {
      aiKeyError = err.toString();
    } finally {
      isTestingKey = false;
    }
  }

  async function handleSaveSettings(e: SubmitEvent) {
    e.preventDefault();
    settingsSuccess = "";
    settingsError = "";

    try {
      await invoke("update_settings", {
        expiryWarningDays,
        defaultLanguage,
        pharmacyName: pharmacyName.trim(),
        pharmacyAddress: pharmacyAddress.trim(),
        directAiInvoice,
      });
      langState.setLanguage(defaultLanguage as any);
      settingsSuccess = "Paramètres enregistrés / تم حفظ الإعدادات";
    } catch (err: any) {
      settingsError = err.toString();
    }
  }

  async function handleChangePin(e: SubmitEvent) {
    e.preventDefault();
    pinSuccess = "";
    pinError = "";

    if (newPin.length < 4) {
      pinError = "New PIN must be at least 4 digits";
      return;
    }
    if (newPin !== confirmNewPin) {
      pinError = "New PINs do not match";
      return;
    }

    try {
      // 1. Verify old PIN
      const pinOk = await invoke<boolean>("verify_pin", { pin: oldPin });
      if (!pinOk) {
        pinError = t("pinError");
        return;
      }

      // 2. Save new PIN
      await invoke("configure_pin", { pin: newPin });
      pinSuccess = "PIN modifié avec succès / تم تغيير الرمز بنجاح";
      
      // Reset inputs
      oldPin = "";
      newPin = "";
      confirmNewPin = "";
    } catch (err: any) {
      pinError = err.toString();
    }
  }

  let usbSuccess = $state("");
  let usbError = $state("");

  async function handleExport() {
    usbSuccess = "";
    usbError = "";
    try {
      const msg = await invoke<string>("backup_database");
      usbSuccess = msg;
      // Refresh last backup time
      const settings = await invoke<any>("get_settings");
      lastBackupAt = settings.last_backup_at;
    } catch (err: any) {
      usbError = err.toString();
    }
  }

  async function handleRestore() {
    usbSuccess = "";
    usbError = "";
    
    if (confirm("Voulez-vous vraiment restaurer la base de données ? Cela écrasera toutes vos données actuelles. / هل تريد فعلاً استعادة قاعدة البيانات؟ سيؤدي هذا إلى مسح كل البيانات الحالية.")) {
      try {
        const msg = await invoke<string>("restore_database");
        usbSuccess = msg;
        setTimeout(() => {
          window.location.reload();
        }, 2000);
      } catch (err: any) {
        usbError = err.toString();
      }
    }
  }
</script>

<div class="settings-page">
  <main class="settings-content">
    <!-- Expiry warning & default lang form -->
    <section class="settings-section card">
      <h3>Configuration Globale</h3>
      
      <form onsubmit={handleSaveSettings} class="form-grid mt-1">
        {#if settingsSuccess}
          <div class="success-banner">{settingsSuccess}</div>
        {/if}
        {#if settingsError}
          <div class="error-banner">{settingsError}</div>
        {/if}
 
        <div class="form-group">
          <label for="warning-days">{t("expiryThreshold")}</label>
          <input
            id="warning-days"
            type="number"
            min="1"
            class="input-pos"
            bind:value={expiryWarningDays}
            required
          />
        </div>
 
        <div class="form-group">
          <label for="pharmacy-name">{t("pharmacyNameLabel")}</label>
          <input
            id="pharmacy-name"
            type="text"
            class="input-pos"
            bind:value={pharmacyName}
            placeholder={t("pharmacyNamePlaceholder")}
          />
        </div>

        <div class="form-group">
          <label for="pharmacy-address">{t("pharmacyAddressLabel")}</label>
          <input
            id="pharmacy-address"
            type="text"
            class="input-pos"
            bind:value={pharmacyAddress}
            placeholder={t("pharmacyAddressPlaceholder")}
          />
        </div>

        <div class="form-group">
          <label for="def-lang">{t("language")} (Par défaut)</label>
          <select id="def-lang" class="input-pos" bind:value={defaultLanguage}>
            <option value="fr">Français</option>
            <option value="ar">العربية</option>
          </select>
        </div>

        <button type="submit" class="btn-action btn-action-primary w-100 mt-1">
          {t("save")}
        </button>
      </form>
    </section>

    <!-- Cashier Permissions Section -->
    <section class="settings-section card">
      <div class="card-header-flex">
        <div>
          <h3>{t("cashierPermissionsTitle")}</h3>
        </div>
      </div>

      <form onsubmit={handleSavePermissions} class="permissions-form mt-1">
        {#if permsSuccess}
          <div class="success-banner">{permsSuccess}</div>
        {/if}
        {#if permsError}
          <div class="error-banner">{permsError}</div>
        {/if}

        <div class="perms-group-title mt-1">{t("permPagesTitle")}</div>
        <div class="perms-grid">
          <label class="perm-switch-row">
            <div class="perm-info">
              <span class="perm-label">{t("permAccessStock")}</span>
              <span class="perm-desc">{t("permAccessStockDesc")}</span>
            </div>
            <input type="checkbox" bind:checked={cashierPerms.access_stock} class="pos-toggle" />
          </label>

          <label class="perm-switch-row">
            <div class="perm-info">
              <span class="perm-label">{t("permAccessInvoices")}</span>
              <span class="perm-desc">{t("permAccessInvoicesDesc")}</span>
            </div>
            <input type="checkbox" bind:checked={cashierPerms.access_invoices} class="pos-toggle" />
          </label>

          <label class="perm-switch-row">
            <div class="perm-info">
              <span class="perm-label">{t("permAccessLoans")}</span>
              <span class="perm-desc">{t("permAccessLoansDesc")}</span>
            </div>
            <input type="checkbox" bind:checked={cashierPerms.access_loans} class="pos-toggle" />
          </label>

          <label class="perm-switch-row">
            <div class="perm-info">
              <span class="perm-label">{t("permAccessHistory")}</span>
              <span class="perm-desc">{t("permAccessHistoryDesc")}</span>
            </div>
            <input type="checkbox" bind:checked={cashierPerms.access_history} class="pos-toggle" />
          </label>

          <label class="perm-switch-row">
            <div class="perm-info">
              <span class="perm-label">{t("permAccessPatients")}</span>
              <span class="perm-desc">{t("permAccessPatientsDesc")}</span>
            </div>
            <input type="checkbox" bind:checked={cashierPerms.access_patients} class="pos-toggle" />
          </label>
        </div>

        <div class="perms-group-title mt-2">{t("permActionsTitle")}</div>
        <div class="perms-grid">
          <label class="perm-switch-row">
            <div class="perm-info">
              <span class="perm-label">{t("permCanGiveLoans")}</span>
              <span class="perm-desc">{t("permCanGiveLoansDesc")}</span>
            </div>
            <input type="checkbox" bind:checked={cashierPerms.can_give_loans} class="pos-toggle" />
          </label>

          <label class="perm-switch-row">
            <div class="perm-info">
              <span class="perm-label">{t("permCanSettleLoans")}</span>
              <span class="perm-desc">{t("permCanSettleLoansDesc")}</span>
            </div>
            <input type="checkbox" bind:checked={cashierPerms.can_settle_loans} class="pos-toggle" />
          </label>

          <label class="perm-switch-row">
            <div class="perm-info">
              <span class="perm-label">{t("permCanAddDrug")}</span>
              <span class="perm-desc">{t("permCanAddDrugDesc")}</span>
            </div>
            <input type="checkbox" bind:checked={cashierPerms.can_add_drug} class="pos-toggle" />
          </label>

          <label class="perm-switch-row">
            <div class="perm-info">
              <span class="perm-label">{t("permCanEditDrug")}</span>
              <span class="perm-desc">{t("permCanEditDrugDesc")}</span>
            </div>
            <input type="checkbox" bind:checked={cashierPerms.can_edit_drug} class="pos-toggle" />
          </label>

          <label class="perm-switch-row">
            <div class="perm-info">
              <span class="perm-label">{t("permCanDeleteDrug")}</span>
              <span class="perm-desc">{t("permCanDeleteDrugDesc")}</span>
            </div>
            <input type="checkbox" bind:checked={cashierPerms.can_delete_drug} class="pos-toggle" />
          </label>

          <label class="perm-switch-row">
            <div class="perm-info">
              <span class="perm-label">{t("permCanDeletePatient")}</span>
              <span class="perm-desc">{t("permCanDeletePatientDesc")}</span>
            </div>
            <input type="checkbox" bind:checked={cashierPerms.can_delete_patient} class="pos-toggle" />
          </label>
        </div>

        <div class="permissions-actions">
          <button type="submit" class="btn-action btn-action-primary">
            {t("savePermissions")}
          </button>
        </div>
      </form>
    </section>

    <!-- Gemini AI API Key CRUD Section -->
    <section class="settings-section card">
      <div class="card-header-flex">
        <div>
          <h3>{t("aiSettingsTitle")}</h3>
        </div>
        <div class="status-badge-container">
          {#if savedGeminiKey}
            <span class="badge-status-active">{t("aiKeyConfigured")}</span>
          {:else}
            <span class="badge-status-inactive">{t("aiKeyNotConfigured")}</span>
          {/if}
        </div>
      </div>

      <form onsubmit={handleSaveAiKey} class="form-grid mt-1">
        {#if aiKeySuccess}
          <div class="success-banner">{aiKeySuccess}</div>
        {/if}
        {#if aiKeyError}
          <div class="error-banner">{aiKeyError}</div>
        {/if}

        <div class="form-group">
          <label for="ai-key-input">{t("aiKeyLabel")}</label>
          <div class="key-input-wrapper">
            <input
              id="ai-key-input"
              type={showGeminiKey ? "text" : "password"}
              class="input-pos font-mono"
              bind:value={geminiKey}
              placeholder={t("aiApiKeyPlaceholder")}
              autocomplete="off"
              spellcheck="false"
            />
            <button
              type="button"
              class="btn-toggle-key"
              onclick={() => showGeminiKey = !showGeminiKey}
              title={showGeminiKey ? t("hideKey") : t("showKey")}
              aria-label={showGeminiKey ? t("hideKey") : t("showKey")}
            >
              <img src={viewIcon} alt="Afficher" class="icon-eye" />
            </button>
          </div>
        </div>

        <!-- Direct AI invoice import option -->
        <label class="perm-switch-row">
          <div class="perm-info">
            <span class="perm-label">{t("directAiInvoiceLabel")}</span>
            <span class="perm-desc">{t("directAiInvoiceDesc")}</span>
          </div>
          <input 
            type="checkbox" 
            checked={directAiInvoice} 
            onchange={handleToggleDirectAi} 
            class="pos-toggle" 
          />
        </label>

        <div class="ai-actions-row">
          <button type="submit" class="btn-action btn-action-primary">
            {t("saveAiKey")}
          </button>
          <button
            type="button"
            class="btn-action btn-action-secondary"
            disabled={isTestingKey || !geminiKey.trim()}
            onclick={handleTestAiKey}
          >
            {#if isTestingKey}
              {t("testingAiKey")}
            {:else}
              {t("testAiKey")}
            {/if}
          </button>
          {#if savedGeminiKey}
            <button
              type="button"
              class="btn-action btn-action-danger"
              onclick={handleDeleteAiKey}
            >
              {t("deleteAiKey")}
            </button>
          {/if}
        </div>
      </form>
    </section>

    <!-- Change PIN Form -->
    <section class="settings-section card">
      <h3>{t("changePin")}</h3>
 
      <form onsubmit={handleChangePin} class="form-grid mt-1">
        {#if pinSuccess}
          <div class="success-banner">{pinSuccess}</div>
        {/if}
        {#if pinError}
          <div class="error-banner">{pinError}</div>
        {/if}
 
        <div class="form-group">
          <label for="old-pin">Code PIN Actuel</label>
          <input
            id="old-pin"
            type="password"
            maxlength="8"
            class="input-pos"
            bind:value={oldPin}
            placeholder="••••"
            required
          />
        </div>
 
        <div class="form-group">
          <label for="new-pin-set">Nouveau Code PIN</label>
          <input
            id="new-pin-set"
            type="password"
            maxlength="8"
            class="input-pos"
            bind:value={newPin}
            placeholder="••••"
            required
          />
        </div>
 
        <div class="form-group">
          <label for="new-pin-confirm">Confirmer le PIN</label>
          <input
            id="new-pin-confirm"
            type="password"
            maxlength="8"
            class="input-pos"
            bind:value={confirmNewPin}
            placeholder="••••"
            required
          />
        </div>
 
        <button type="submit" class="btn-action btn-action-secondary w-100 mt-1">
          {t("changePin")}
        </button>
      </form>
    </section>
 
    <!-- USB Export / Import Section -->
    <section class="settings-section card">
      <h3>{t("usbOperations")}</h3>
      
      <div class="form-grid mt-1">
        {#if usbSuccess}
          <div class="success-banner">{usbSuccess}</div>
        {/if}
        {#if usbError}
          <div class="error-banner">{usbError}</div>
        {/if}

        <div class="form-group">
          <p class="backup-meta" style="margin-bottom: 0.75rem;">
            <strong>Dernière sauvegarde :</strong> 
            {lastBackupAt ? formatDateFr(lastBackupAt) : "Aucune sauvegarde détectée"}
          </p>
          <button type="button" onclick={handleExport} class="btn-action btn-action-primary w-100">
            {t("exportTitle")}
          </button>
        </div>

        <hr style="border: none; border-top: 1px solid var(--color-border); margin: 0.5rem 0;" />

        <div class="form-group">
          <p style="color: var(--color-danger); font-weight: bold; font-size: 0.95rem; margin-bottom: 0.75rem;">
            {t("restoreWarning")}
          </p>
          <button type="button" onclick={handleRestore} class="btn-action btn-action-danger w-100">
            {t("importTitle")}
          </button>
        </div>
      </div>
    </section>

    <!-- Application Updates Section -->
    <section class="settings-section card">
      <h3>{t("checkForUpdates")}</h3>
      <div class="form-grid mt-1">
        <p style="color: var(--color-text-secondary); font-size: 0.95rem; margin-bottom: 0.25rem;">
          {t("appUpToDateDesc")}
        </p>
        <button
          type="button"
          class="btn-action btn-action-primary w-100"
          disabled={updaterStore.status === 'checking' || updaterStore.status === 'downloading'}
          onclick={() => updaterStore.checkForUpdates(false)}
        >
          {#if updaterStore.status === 'checking'}
            {t("checkingUpdates")}
          {:else}
            {t("checkForUpdates")}
          {/if}
        </button>
      </div>
    </section>
  </main>
</div>

<style>
  .settings-page {
    padding: 0;
    background-color: transparent;
    height: 100%;
    overflow-y: auto;
    box-sizing: border-box;
  }

  .settings-content {
    display: grid;
    grid-template-columns: 1fr;
    gap: clamp(1rem, 1.5vw, 1.5rem);
    margin-top: 0.5rem;
    padding-bottom: 2rem;
    max-width: 1200px;
  }

  .card {
    background-color: var(--color-bg-card);
    border: var(--border-width) solid var(--color-border);
    border-radius: var(--border-radius);
    padding: clamp(1rem, 1.8vw, 2rem);
    box-shadow: 0 4px 6px var(--color-shadow);
  }

  .form-grid {
    display: flex;
    flex-direction: column;
    gap: 1.25rem;
  }

  .form-group {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }

  .form-group label {
    font-weight: 700;
  }

  .backup-meta {
    font-size: 1.05rem;
    color: #5a6b82;
  }

  .w-100 {
    width: 100%;
  }

  .mt-1 {
    margin-top: 1rem;
  }

  .success-banner {
    grid-column: span 2;
    background-color: #e3fcef;
    border: 2px solid var(--color-primary);
    color: var(--color-primary-hover);
    padding: 0.75rem;
    border-radius: 8px;
    text-align: center;
    font-weight: bold;
  }

  .error-banner {
    grid-column: span 2;
    background-color: #ffebe6;
    border: 2px solid var(--color-danger);
    color: var(--color-danger);
    padding: 0.75rem;
    border-radius: 8px;
    text-align: center;
    font-weight: bold;
  }

  select.input-pos {
    appearance: none;
    background-image: url("data:image/svg+xml;charset=UTF-8,%3csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='%23091e42' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3e%3cpolyline points='6 9 12 15 18 9'%3e%3c/polyline%3e%3c/svg%3e");
    background-repeat: no-repeat;
    background-position: right 1rem center;
    background-size: 20px;
    padding-right: 2.5rem;
  }

  :global([dir="rtl"]) select.input-pos {
    background-position: left 1rem center;
    padding-left: 2.5rem;
    padding-right: 1rem;
  }

  /* Gemini AI Settings Styles */
  .card-header-flex {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 1rem;
  }

  .badge-status-active {
    display: inline-block;
    background: #dcfce7;
    color: #15803d;
    font-weight: 600;
    font-size: 0.8rem;
    padding: 0.3rem 0.65rem;
    border-radius: 9999px;
    border: 1px solid #86efac;
    white-space: nowrap;
  }

  .badge-status-inactive {
    display: inline-block;
    background: #f1f5f9;
    color: #64748b;
    font-weight: 600;
    font-size: 0.8rem;
    padding: 0.3rem 0.65rem;
    border-radius: 9999px;
    border: 1px solid #cbd5e1;
    white-space: nowrap;
  }

  .key-input-wrapper {
    position: relative;
    display: flex;
    align-items: center;
  }

  .font-mono {
    font-family: monospace;
    font-size: 0.95rem;
    letter-spacing: 0.05em;
  }

  .btn-toggle-key {
    position: absolute;
    right: 0.75rem;
    background: transparent;
    border: none;
    cursor: pointer;
    padding: 0.3rem;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 6px;
    transition: all 0.15s ease;
  }

  .btn-toggle-key:hover {
    background: #e2e8f0;
  }

  .icon-eye {
    width: 22px;
    height: 22px;
    display: block;
    object-fit: contain;
    opacity: 0.65;
    transition: opacity 0.15s ease;
  }

  .btn-toggle-key:hover .icon-eye {
    opacity: 1;
  }

  :global([dir="rtl"]) .btn-toggle-key {
    right: auto;
    left: 0.75rem;
  }

  .ai-actions-row {
    display: flex;
    gap: 0.75rem;
    flex-wrap: wrap;
    align-items: center;
  }

  .perms-group-title {
    font-size: 1.15rem;
    font-weight: 750;
    color: var(--color-primary);
    border-bottom: 2px solid var(--color-border);
    padding-bottom: 0.4rem;
    margin-bottom: 0.75rem;
  }

  .perms-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
    gap: 0.85rem;
  }

  .perm-switch-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    background-color: var(--color-bg-app);
    border: 1px solid var(--color-border);
    border-radius: var(--border-radius);
    padding: 0.85rem 1rem;
    cursor: pointer;
    gap: 1rem;
    transition: border-color 0.15s;
  }

  .perm-switch-row:hover {
    border-color: var(--color-primary);
  }

  .perm-info {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
  }

  .perm-label {
    font-weight: 700;
    font-size: 1.05rem;
    color: var(--color-text-dark);
  }

  .perm-desc {
    font-size: 0.85rem;
    color: var(--color-text-secondary);
  }

  .pos-toggle {
    appearance: none;
    -webkit-appearance: none;
    width: 46px;
    height: 24px;
    background: #cbd5e1;
    border-radius: 12px;
    position: relative;
    cursor: pointer;
    outline: none;
    transition: background-color 0.2s;
    flex-shrink: 0;
  }

  .pos-toggle::after {
    content: '';
    position: absolute;
    top: 2px;
    left: 2px;
    width: 20px;
    height: 20px;
    background: #ffffff;
    border-radius: 50%;
    transition: transform 0.2s;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.25);
  }

  .pos-toggle:checked {
    background-color: var(--color-primary);
  }

  .pos-toggle:checked::after {
    transform: translateX(22px);
  }

  :global([dir="rtl"]) .pos-toggle::after {
    left: auto;
    right: 2px;
  }

  :global([dir="rtl"]) .pos-toggle:checked::after {
    transform: translateX(-22px);
  }

  .permissions-actions {
    margin-top: 2.5rem;
    padding-top: 1.25rem;
    border-top: 1px solid var(--color-border);
    display: flex;
    justify-content: flex-end;
  }
</style>
