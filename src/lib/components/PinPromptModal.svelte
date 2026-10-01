<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { t } from "$lib/i18n/index.svelte";

  let { onVerified, onCancel } = $props<{
    onVerified: () => void;
    onCancel: () => void;
  }>();

  let pin = $state("");
  let errorMsg = $state("");

  async function handleVerify(e?: SubmitEvent) {
    if (e) e.preventDefault();
    errorMsg = "";
    try {
      const success = await invoke<boolean>("verify_pin", { pin });
      if (success) {
        onVerified();
      } else {
        errorMsg = t("pinError");
        pin = "";
      }
    } catch (err: any) {
      errorMsg = err.toString();
    }
  }

  function appendToPin(val: string) {
    if (pin.length < 8) {
      pin += val;
    }
  }

  function clearPin() {
    pin = "";
  }
</script>

<div class="modal-overlay">
  <div class="modal-content pin-modal">
    <h3 class="text-center">{t("enterPin")}</h3>
    
    <form onsubmit={handleVerify} class="pin-form mt-1">
      <input
        type="password"
        readonly
        class="input-pos text-center pin-display"
        value={pin}
        placeholder="••••••••"
      />

      {#if errorMsg}
        <div class="error-banner">{errorMsg}</div>
      {/if}

      <!-- Big Numeric Keypad for Elderly Accessibility -->
      <div class="numpad">
        {#each ["1", "2", "3", "4", "5", "6", "7", "8", "9"] as num}
          <button type="button" class="numpad-btn" onclick={() => appendToPin(num)}>{num}</button>
        {/each}
        <button type="button" class="numpad-btn clear-btn" onclick={clearPin}>C</button>
        <button type="button" class="numpad-btn" onclick={() => appendToPin("0")}>0</button>
        <button type="submit" class="numpad-btn ok-btn">OK</button>
      </div>

      <button type="button" onclick={onCancel} class="btn-action btn-action-danger w-100 mt-1">
        {t("cancel")}
      </button>
    </form>
  </div>
</div>

<style>
  .pin-modal {
    max-width: 400px;
    padding: 2rem;
  }

  .text-center {
    text-align: center;
  }

  .mt-1 {
    margin-top: 1rem;
  }

  .w-100 {
    width: 100%;
  }

  .pin-display {
    font-size: 2rem;
    letter-spacing: 0.5rem;
    height: 60px;
    margin-bottom: 1rem;
    text-align: center;
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

  .numpad {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 0.75rem;
    margin-top: 1rem;
  }

  .numpad-btn {
    height: 70px;
    font-size: 1.6rem;
    font-weight: bold;
    border-radius: 12px;
    border: 2px solid var(--color-border);
    background-color: #f8f9fa;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    box-shadow: 0 2px 4px var(--color-shadow);
  }

  .numpad-btn:active {
    background-color: #e2e6ea;
  }

  .clear-btn {
    background-color: #f8d7da;
    border-color: #f5c6cb;
    color: #721c24;
  }

  .ok-btn {
    background-color: var(--color-primary);
    border-color: var(--color-primary-hover);
    color: var(--color-text-light);
  }
</style>
