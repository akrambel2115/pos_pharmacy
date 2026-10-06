<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { t, langState } from "$lib/i18n/index.svelte";
  import { onMount } from "svelte";
  import cashierIcon from "../../public/icons/cashier.png";
  import adminIcon from "../../public/icons/admin.png";

  // Bind properties to notify parent
  let { onSessionStarted } = $props<{
    onSessionStarted: (role: "cashier" | "admin") => void;
  }>();

  let isFirstLaunch = $state(true);
  let sessionState = $state<"checking" | "setup" | "selection" | "pin_prompt">("checking");
  
  // State variables for inputs
  let pin = $state("");
  let confirmPin = $state("");
  let errorMsg = $state("");

  onMount(async () => {
    try {
      const configured = await invoke<boolean>("is_pin_configured");
      isFirstLaunch = !configured;
      if (isFirstLaunch) {
        sessionState = "setup";
      } else {
        sessionState = "selection";
      }
    } catch (err) {
      console.error(err);
      sessionState = "selection"; // Fallback
    }
  });

  async function handleSetupPin(e: SubmitEvent) {
    e.preventDefault();
    errorMsg = "";
    if (pin.length < 4) {
      errorMsg = "PIN must be at least 4 digits";
      return;
    }
    if (pin !== confirmPin) {
      errorMsg = "PINs do not match";
      return;
    }
    try {
      await invoke("configure_pin", { pin });
      isFirstLaunch = false;
      sessionState = "selection";
      pin = "";
      confirmPin = "";
    } catch (err: any) {
      errorMsg = err.toString();
    }
  }

  async function handleVerifyPin(e: SubmitEvent) {
    e.preventDefault();
    errorMsg = "";
    try {
      const success = await invoke<boolean>("verify_pin", { pin });
      if (success) {
        onSessionStarted("admin");
      } else {
        errorMsg = t("pinError");
        pin = "";
      }
    } catch (err: any) {
      errorMsg = err.toString();
    }
  }

  function startCashier() {
    onSessionStarted("cashier");
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

<div class="session-overlay">
  {#if sessionState === "checking"}
    <div class="loader">Loading...</div>
  {:else if sessionState === "setup"}
    <!-- First Launch PIN Setup -->
    <div class="card">
      <h2 class="title">{t("setupPinTitle")}</h2>
      <form onsubmit={handleSetupPin} class="form-container">
        <label for="new-pin">{t("setupPinPrompt")}</label>
        <input
          id="new-pin"
          type="password"
          inputmode="numeric"
          pattern="[0-9]*"
          maxlength="8"
          class="input-pos text-center"
          bind:value={pin}
          placeholder="••••"
        />

        <label for="confirm-pin">{t("setupPinConfirm")}</label>
        <input
          id="confirm-pin"
          type="password"
          inputmode="numeric"
          pattern="[0-9]*"
          maxlength="8"
          class="input-pos text-center"
          bind:value={confirmPin}
          placeholder="••••"
        />

        {#if errorMsg}
          <div class="error-banner">{errorMsg}</div>
        {/if}

        <button type="submit" class="btn-action btn-action-primary w-100 mt-1">
          {t("save")}
        </button>
      </form>
    </div>
  {:else if sessionState === "selection"}
    <!-- Select Session Mode -->
    <div class="card select-card">
      <h2 class="title text-center mb-2">{t("selectSession")}</h2>
      
      <!-- Bilingual Toggle inside overlay -->
      <div class="lang-toggle-bar">
        <button 
          class="lang-btn {langState.current === 'fr' ? 'active' : ''}" 
          onclick={() => langState.setLanguage('fr')}
        >
          Français
        </button>
        <button 
          class="lang-btn {langState.current === 'ar' ? 'active' : ''}" 
          onclick={() => langState.setLanguage('ar')}
        >
          العربية
        </button>
      </div>

      <div class="btn-grid">
        <button onclick={startCashier} class="btn-pos-huge btn-cashier">
          <img src={cashierIcon} alt="Cashier" class="session-icon-img" />
          <span class="btn-label">{t("cashierMode")}</span>
        </button>

        <button onclick={() => sessionState = "pin_prompt"} class="btn-pos-huge btn-admin">
          <img src={adminIcon} alt="Admin" class="session-icon-img" />
          <span class="btn-label">{t("adminMode")}</span>
        </button>
      </div>
    </div>
  {:else if sessionState === "pin_prompt"}
    <!-- Admin PIN Entry Form -->
    <div class="card pin-card">
      <div class="back-bar">
        <button class="back-btn" onclick={() => { sessionState = "selection"; pin = ""; errorMsg = ""; }}>
          ← {t("cancel")}
        </button>
      </div>
      <h2 class="title text-center">{t("enterPin")}</h2>

      <form onsubmit={handleVerifyPin} class="pin-form">
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
      </form>
    </div>
  {/if}
</div>

<style>
  .session-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background-color: var(--color-bg-app);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 2000;
    padding: 1rem;
    overflow-y: auto;
    box-sizing: border-box;
  }

  .card {
    background-color: var(--color-bg-card);
    border: 3px solid var(--color-border);
    border-radius: 16px;
    padding: clamp(1.25rem, 3vw, 2.5rem);
    box-shadow: 0 10px 30px var(--color-shadow);
    width: 90%;
    max-width: 500px;
    max-height: 92vh;
    overflow-y: auto;
    box-sizing: border-box;
  }

  .select-card {
    max-width: 600px;
  }

  .pin-card {
    max-width: 420px;
  }

  .title {
    font-size: clamp(1.3rem, 2.2vw, 1.8rem);
    margin-bottom: clamp(0.75rem, 1.5vh, 1.5rem);
    color: var(--color-text-dark);
  }

  .text-center {
    text-align: center;
  }

  .mb-2 {
    margin-bottom: clamp(1rem, 2vh, 2rem);
  }

  .mt-1 {
    margin-top: 0.75rem;
  }

  .w-100 {
    width: 100%;
  }

  .form-container {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .form-container label {
    font-weight: bold;
    font-size: clamp(0.95rem, 1.1vw, 1.1rem);
  }

  .btn-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: clamp(0.75rem, 1.5vw, 1.5rem);
  }

  @media (max-width: 460px) {
    .btn-grid {
      grid-template-columns: 1fr;
    }
  }

  .btn-pos-huge {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: clamp(1rem, 2vw, 2rem);
    border: 3px solid var(--color-border);
    border-radius: 16px;
    background-color: var(--color-bg-card);
    cursor: pointer;
    transition: all 0.2s ease-in-out;
    height: clamp(130px, 18vh, 200px);
    box-shadow: 0 6px 12px var(--color-shadow);
  }

  .session-icon-img {
    width: clamp(48px, 7vh, 80px);
    height: clamp(48px, 7vh, 80px);
    object-fit: contain;
    margin-bottom: clamp(0.5rem, 1vh, 1rem);
  }

  .btn-pos-huge:hover {
    transform: scale(1.04);
  }

  .btn-cashier:hover {
    border-color: var(--color-primary);
  }

  .btn-admin:hover {
    border-color: var(--color-secondary);
  }

  .btn-pos-huge .btn-label {
    font-size: 1.3rem;
    font-weight: bold;
    color: var(--color-text-dark);
  }

  .lang-toggle-bar {
    display: flex;
    justify-content: center;
    gap: 1rem;
    margin-bottom: 1.5rem;
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

  .back-bar {
    margin-bottom: 1rem;
  }

  .back-btn {
    background: none;
    border: none;
    font-size: 1.1rem;
    font-weight: bold;
    color: var(--color-secondary);
    cursor: pointer;
  }

  .pin-display {
    font-size: clamp(1.4rem, 2.5vw, 2rem);
    letter-spacing: 0.5rem;
    height: clamp(44px, 6vh, 60px);
    margin-bottom: 0.75rem;
  }

  .error-banner {
    background-color: #ffebe6;
    border: 2px solid var(--color-danger);
    color: var(--color-danger);
    padding: 0.5rem 0.75rem;
    border-radius: 8px;
    text-align: center;
    font-weight: bold;
    margin-bottom: 0.75rem;
  }

  /* Numpad layout */
  .numpad {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: clamp(0.4rem, 1vh, 0.75rem);
    margin-top: 0.5rem;
  }

  .numpad-btn {
    height: clamp(48px, 6.5vh, 70px);
    font-size: clamp(1.2rem, 1.8vw, 1.6rem);
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

  .loader {
    font-size: 1.5rem;
    font-weight: bold;
  }
</style>
