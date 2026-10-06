<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { t, langState } from "$lib/i18n/index.svelte";

  let { status, machineCode, onActivated } = $props<{
    status: "missing" | "invalid" | "expired" | "clock";
    machineCode: string;
    onActivated: () => void;
  }>();

  let token = $state("");
  let errorMsg = $state("");
  let copied = $state(false);
  let busy = $state(false);

  let notice = $derived(
    status === "expired" ? t("licenseExpired")
    : status === "clock" ? t("licenseClock")
    : status === "invalid" ? t("licenseInvalidStored")
    : ""
  );

  async function copyCode() {
    try {
      await navigator.clipboard.writeText(machineCode);
      copied = true;
      setTimeout(() => (copied = false), 2000);
    } catch (e) {
      console.error(e);
    }
  }

  async function activate(e: SubmitEvent) {
    e.preventDefault();
    errorMsg = "";
    if (!token.trim()) return;
    busy = true;
    try {
      await invoke("activate_license", { token });
      onActivated();
    } catch (err: any) {
      errorMsg = err.toString() === "expired" ? t("licenseExpired") : t("licenseInvalid");
    } finally {
      busy = false;
    }
  }
</script>

<div class="license-overlay">
  <div class="card">
    <div class="lang-toggle-bar">
      <button class="lang-btn {langState.current === 'fr' ? 'active' : ''}" onclick={() => langState.setLanguage('fr')}>Français</button>
      <button class="lang-btn {langState.current === 'ar' ? 'active' : ''}" onclick={() => langState.setLanguage('ar')}>العربية</button>
    </div>

    <h2 class="title">{t("licenseTitle")}</h2>

    {#if notice}
      <div class="error-banner">{notice}</div>
    {/if}

    <p class="step">1. {t("licenseStep1")}</p>
    <div class="code-row">
      <div class="code-box" dir="ltr">{machineCode}</div>
      <button type="button" class="btn-action btn-action-secondary" onclick={copyCode}>
        {copied ? t("licenseCopied") : t("licenseCopy")}
      </button>
    </div>

    <form onsubmit={activate} class="form-container">
      <label for="license-token">2. {t("licenseStep2")}</label>
      <textarea
        id="license-token"
        class="input-pos token-input"
        dir="ltr"
        rows="4"
        bind:value={token}
        placeholder={t("licensePlaceholder")}
      ></textarea>

      {#if errorMsg}
        <div class="error-banner">{errorMsg}</div>
      {/if}

      <button type="submit" class="btn-action btn-action-primary w-100" disabled={busy || !token.trim()}>
        {t("licenseActivate")}
      </button>
    </form>
  </div>
</div>

<style>
  .license-overlay {
    position: fixed;
    inset: 0;
    background-color: var(--color-bg-app);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 3000;
    overflow-y: auto;
  }

  .card {
    background-color: var(--color-bg-card);
    border: 3px solid var(--color-border);
    border-radius: 16px;
    padding: 2.5rem;
    box-shadow: 0 10px 30px var(--color-shadow);
    width: 90%;
    max-width: 620px;
  }

  .title {
    font-size: 1.8rem;
    margin-bottom: 1.25rem;
    text-align: center;
  }

  .step {
    font-weight: bold;
    margin-bottom: 0.5rem;
  }

  .code-row {
    display: flex;
    gap: 0.75rem;
    align-items: stretch;
    margin-bottom: 1.5rem;
  }

  .code-box {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0.85rem 1rem;
    font-family: Consolas, "Courier New", monospace;
    font-size: 1.4rem;
    font-weight: bold;
    letter-spacing: 0.08rem;
    border: 3px dashed var(--color-secondary);
    border-radius: 12px;
    background-color: #f4f8ff;
    user-select: all;
  }

  .form-container {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .form-container label {
    font-weight: bold;
  }

  .token-input {
    font-family: Consolas, "Courier New", monospace;
    font-size: 0.95rem;
    resize: none;
    user-select: text;
    word-break: break-all;
  }

  .w-100 {
    width: 100%;
  }

  .btn-action:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .lang-toggle-bar {
    display: flex;
    justify-content: center;
    gap: 1rem;
    margin-bottom: 1.25rem;
  }

  .lang-btn {
    padding: 0.5rem 1.5rem;
    border-radius: 20px;
    border: 2px solid var(--color-border);
    background-color: var(--color-bg-card);
    font-weight: bold;
    cursor: pointer;
  }

  .lang-btn.active {
    background-color: var(--color-primary);
    color: var(--color-text-light);
    border-color: var(--color-primary);
  }

  .error-banner {
    background-color: #ffebe6;
    border: 2px solid var(--color-danger);
    color: var(--color-danger);
    padding: 0.75rem;
    border-radius: 8px;
    text-align: center;
    font-weight: bold;
    margin-bottom: 1rem;
  }
</style>
