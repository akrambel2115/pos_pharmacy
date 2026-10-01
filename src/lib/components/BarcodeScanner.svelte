<script lang="ts">
  import { onMount } from "svelte";
  import { t } from "$lib/i18n/index.svelte";

  let { onScan, placeholder = "barcodePlaceholder" } = $props<{
    onScan: (barcode: string) => void;
    placeholder?: string;
  }>();

  let barcode = $state("");
  let inputEl = $state<HTMLInputElement | null>(null);

  function handleSubmit(e: SubmitEvent) {
    e.preventDefault();
    const code = barcode.trim();
    if (code) {
      onScan(code);
      barcode = "";
    }
  }

  function handleBlur(e: FocusEvent) {
    // Focus back to this barcode input unless another input/button/textarea was selected
    const target = e.relatedTarget as HTMLElement;
    if (!target || (target.tagName !== "INPUT" && target.tagName !== "TEXTAREA" && target.tagName !== "BUTTON")) {
      setTimeout(() => {
        inputEl?.focus();
      }, 50);
    }
  }

  onMount(() => {
    inputEl?.focus();
  });
</script>

<form onsubmit={handleSubmit} class="barcode-scanner-form">
  <div class="input-wrapper">
    <svg class="barcode-icon" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke="currentColor">
      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v1m-3 0v2m3-2v3m-3-3v3m6-3v2m-6-2v1m9-1h-1m2 0v2m-2-2v3m2-3v4m-12 0H4a2 2 0 01-2-2V6a2 2 0 012-2h2m12 0h2a2 2 0 012 2v12a2 2 0 01-2 2h-2m-12 0v-4m0 4v1" />
    </svg>
    <input
      bind:this={inputEl}
      type="text"
      class="input-pos barcode-input"
      placeholder={t(placeholder as any)}
      bind:value={barcode}
      onblur={handleBlur}
      autocomplete="off"
    />
  </div>
</form>

<style>
  .barcode-scanner-form {
    width: 100%;
  }

  .input-wrapper {
    position: relative;
    display: flex;
    align-items: center;
    width: 100%;
  }

  .barcode-icon {
    position: absolute;
    left: 1.25rem;
    width: 32px;
    height: 32px;
    color: var(--color-primary);
    pointer-events: none;
    z-index: 10;
  }

  /* Handle RTL position flipping */
  :global([dir="ltr"]) .barcode-input {
    padding-left: 4rem;
  }

  :global([dir="rtl"]) .barcode-input {
    padding-right: 4rem;
    padding-left: 1rem;
  }

  :global([dir="rtl"]) .barcode-icon {
    right: 1.25rem;
    left: auto;
  }

  .barcode-input {
    height: 65px;
    font-size: 1.4rem;
    font-weight: 750;
    border-color: var(--color-primary);
  }

  .barcode-input:focus {
    box-shadow: 0 0 0 4px rgba(0, 135, 90, 0.2);
  }
</style>
