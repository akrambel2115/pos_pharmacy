<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { t } from "$lib/i18n/index.svelte";
  import { onMount } from "svelte";

  let { barcode, userRole, onSubmit, onCancel } = $props<{
    barcode: string;
    userRole: "cashier" | "admin";
    onSubmit: () => void;
    onCancel: () => void;
  }>();

  // The 8 medicine fields
  let name = $state(""); // Désignation
  let packagesReceived = $state<number>(1); // Quantité
  let batchNumber = $state("LOT-01"); // N° Lot
  let pricePerItem = $state<number | null>(null); // PPA
  let costPrice = $state<number | null>(null); // PUHT
  let expiryDate = $state(""); // Exp
  let tva = $state<number>(9); // TVA (%)
  let mg = $state<number | null>(null); // MG (%)
  let customBarcode = $state("");

  let errorMsg = $state("");

  onMount(() => {
    if (barcode) {
      customBarcode = barcode;
    }
    if (userRole !== "admin") {
      costPrice = 0;
    }
  });

  function updateMg() {
    if (pricePerItem !== null && costPrice !== null && costPrice > 0) {
      const costTtc = costPrice * (1 + tva / 100);
      if (costTtc > 0) {
        const margin = ((pricePerItem - costTtc) / costTtc) * 100;
        mg = Math.round(margin * 10) / 10;
      }
    }
  }

  function handlePpaInput(e: Event) {
    const val = parseFloat((e.target as HTMLInputElement).value);
    pricePerItem = isNaN(val) ? null : val;
    updateMg();
  }

  function handlePuhtInput(e: Event) {
    const val = parseFloat((e.target as HTMLInputElement).value);
    costPrice = isNaN(val) ? null : val;
    updateMg();
  }

  function handleTvaChange(e: Event) {
    const val = parseFloat((e.target as HTMLSelectElement).value);
    tva = isNaN(val) ? 9 : val;
    updateMg();
  }

  async function handleFormSubmit(e: SubmitEvent) {
    e.preventDefault();
    errorMsg = "";

    if (!name.trim()) {
      errorMsg = "La désignation du médicament est requise";
      return;
    }
    if (packagesReceived <= 0) {
      errorMsg = "La quantité doit être supérieure à 0";
      return;
    }
    if (!batchNumber.trim()) {
      errorMsg = "Le numéro de lot est requis";
      return;
    }
    if (pricePerItem === null || pricePerItem <= 0) {
      errorMsg = "Un PPA valide est requis";
      return;
    }
    if (costPrice === null || costPrice < 0) {
      errorMsg = "Un PUHT valide est requis";
      return;
    }
    if (!expiryDate) {
      errorMsg = "La date d'expiration (Exp) est requise";
      return;
    }

    const today = new Date();
    today.setHours(0, 0, 0, 0);
    const tomorrow = new Date(today);
    tomorrow.setDate(tomorrow.getDate() + 1);

    const [year, month, day] = expiryDate.split("-").map(Number);
    const expiry = new Date(year, month - 1, day);
    expiry.setHours(0, 0, 0, 0);

    if (expiry < tomorrow) {
      errorMsg = t("expiryDateTomorrowError");
      return;
    }

    try {
      await invoke("add_drug_with_batch", {
        barcode: customBarcode.trim() || undefined,
        name: name.trim(),
        requiresPrescription: false,
        pricePerItemDa: pricePerItem,
        costPriceDa: costPrice ?? 0,
        itemsPerPackage: 1,
        expiryDate,
        packagesReceived,
        batchNumber: batchNumber.trim(),
        tva,
        mg: mg ?? 0,
      });
      onSubmit();
    } catch (err: any) {
      errorMsg = err.toString();
    }
  }
</script>

<div class="modal-overlay">
  <div class="modal-content drug-modal">
    <h3>{t("addNewDrug")}</h3>

    <form onsubmit={handleFormSubmit} class="modal-form">
      {#if errorMsg}
        <div class="error-banner">{errorMsg}</div>
      {/if}

      <div class="form-grid">
        <!-- 1. Désignation -->
        <div class="form-group col-span-2">
          <label for="drug-name">{t("designation")} *</label>
          <input
            id="drug-name"
            type="text"
            class="input-pos"
            bind:value={name}
            placeholder="e.g. PARACETAMOL 500MG B/20"
            required
          />
        </div>

        <!-- 2. Quantité -->
        <div class="form-group">
          <label for="qty-received">{t("quantity")} (bxs) *</label>
          <input
            id="qty-received"
            type="number"
            min="1"
            class="input-pos"
            bind:value={packagesReceived}
            required
          />
        </div>

        <!-- 3. N° Lot -->
        <div class="form-group">
          <label for="batch-num">{t("noLot")} *</label>
          <input
            id="batch-num"
            type="text"
            class="input-pos"
            bind:value={batchNumber}
            placeholder="e.g. LOT-24A1"
            required
          />
        </div>

        <!-- 4. PPA -->
        <div class="form-group">
          <label for="item-price">{t("ppa")} ({t("da")}) *</label>
          <input
            id="item-price"
            type="number"
            step="0.01"
            min="0"
            class="input-pos"
            value={pricePerItem ?? ""}
            oninput={handlePpaInput}
            placeholder="0.00"
            required
          />
        </div>

        <!-- 5. PUHT -->
        <div class="form-group">
          <label for="cost-price">{t("puht")} ({t("da")}) *</label>
          <input
            id="cost-price"
            type="number"
            step="0.01"
            min="0"
            class="input-pos"
            value={costPrice ?? ""}
            oninput={handlePuhtInput}
            placeholder="0.00"
            required
          />
        </div>

        <!-- 6. Exp -->
        <div class="form-group">
          <label for="expiry">{t("exp")} *</label>
          <input
            id="expiry"
            type="date"
            class="input-pos"
            bind:value={expiryDate}
            required
          />
        </div>

        <!-- 7. TVA -->
        <div class="form-group">
          <label for="tva-select">{t("tva")} (%)</label>
          <select id="tva-select" class="input-pos" value={tva} onchange={handleTvaChange}>
            <option value={0}>0%</option>
            <option value={9}>9%</option>
            <option value={19}>19%</option>
          </select>
        </div>

        <!-- 8. MG -->
        <div class="form-group">
          <label for="mg-val">{t("mg")} (%)</label>
          <input
            id="mg-val"
            type="number"
            step="0.1"
            class="input-pos"
            bind:value={mg}
            placeholder="Auto"
          />
        </div>

        <!-- Code-barres (Optionnel) -->
        <div class="form-group">
          <label for="barcode-val">{t("barcode")} (Optionnel)</label>
          <input
            id="barcode-val"
            type="text"
            class="input-pos"
            bind:value={customBarcode}
            placeholder="Optionnel"
          />
        </div>
      </div>

      <div class="modal-actions mt-2">
        <button type="button" onclick={onCancel} class="btn-action btn-action-danger">
          {t("cancel")}
        </button>
        <button type="submit" class="btn-action btn-action-primary">
          {t("save")}
        </button>
      </div>
    </form>
  </div>
</div>

<style>
  .drug-modal {
    max-width: 600px;
    width: 90%;
  }

  .modal-form {
    display: flex;
    flex-direction: column;
    gap: 1.25rem;
  }

  .form-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1.25rem;
  }

  .form-group {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }

  .form-group label {
    font-weight: 700;
    font-size: 1rem;
  }

  .col-span-2 {
    grid-column: span 2;
  }

  .error-banner {
    background-color: #ffebe6;
    border: 2px solid var(--color-danger);
    color: var(--color-danger);
    padding: 0.75rem;
    border-radius: 8px;
    text-align: center;
    font-weight: bold;
  }

  .modal-actions {
    display: flex;
    justify-content: flex-end;
    gap: 1rem;
  }

  .mt-2 {
    margin-top: 1.5rem;
  }

  /* Adjust action layout for RTL */
  :global([dir="rtl"]) .modal-actions {
    justify-content: flex-start;
  }
</style>
