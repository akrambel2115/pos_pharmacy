<script lang="ts">
  import { t } from "$lib/i18n/index.svelte";
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import closeIcon from "../../public/icons/close.png";
  import printerIcon from "../../public/icons/printer.png";
  import { formatDateFr } from "$lib/utils";
  import { printThermalInvoice } from "$lib/receipt";

  let { invoice, onClose, title = "TICKET DE CAISSE" } = $props<{
    invoice: any;
    onClose: () => void;
    title?: string;
  }>();

  let pharmacyName = $state("");
  let pharmacyAddress = $state("");
  let isPrinting = $state(false);

  onMount(() => {
    (async () => {
      try {
        const settings = await invoke<any>("get_settings");
        pharmacyName = settings?.pharmacy_name || "";
        pharmacyAddress = settings?.pharmacy_address || "";
      } catch (e) {
        console.error("Failed to load pharmacy settings:", e);
      }
    })();

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        onClose();
      } else if (e.key === "Enter" || e.key === "F9" || (e.ctrlKey && e.key.toLowerCase() === "p")) {
        e.preventDefault();
        handlePrint();
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  });

  async function handlePrint() {
    if (isPrinting) return;
    isPrinting = true;
    try {
      await printThermalInvoice(invoice, pharmacyName, pharmacyAddress);
    } catch (err) {
      console.error("Failed to print thermal invoice:", err);
    } finally {
      setTimeout(() => {
        isPrinting = false;
      }, 500);
    }
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="modal-overlay" onclick={onClose}>
  <!-- Stop propagation so clicking inside the modal content doesn't close it -->
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="modal-content invoice-modal" onclick={(e) => e.stopPropagation()}>
    <button class="btn-close-modal" onclick={onClose} aria-label="Close">
      <img src={closeIcon} alt="Close" class="close-icon-img" />
    </button>

    <div class="receipt-paper">
      <div class="receipt-header-center">
        <h2 class="receipt-pharmacy">{pharmacyName || "PHARMACIE"}</h2>
        {#if pharmacyAddress}
          <div class="receipt-address">{pharmacyAddress}</div>
        {/if}
      </div>

      <hr class="receipt-divider" />

      <h3 class="receipt-title">{invoice.is_loan ? "AVANCE / CRÉDIT" : title}</h3>

      {#if invoice.is_loan}
        <div class="receipt-loan-banner">
          {#if invoice.loan_status === 'settled'}
            AVANCE RÉGLÉE {#if invoice.loan_settled_at}LE {formatDateFr(invoice.loan_settled_at)}{/if}
          {:else}
            AVANCE EN COURS (NON RÉGLÉE)
          {/if}
        </div>
      {/if}

      <div class="receipt-meta">
        <p><strong>Facture N°:</strong> #{invoice.id}</p>
        <p><strong>Date:</strong> {formatDateFr(invoice.created_at)}</p>
        {#if invoice.prescribing_doctor_name}
          <p><strong>Médecin:</strong> {invoice.prescribing_doctor_name}</p>
        {/if}
        {#if invoice.treatment_period_days}
          <p><strong>Durée traitement:</strong> {invoice.treatment_period_days} jours</p>
        {/if}
      </div>

      <hr class="receipt-divider" />
      
      <table class="receipt-table">
        <thead>
          <tr>
            <th>Désignation</th>
            <th class="text-center">Qté</th>
            <th class="text-right">P.U</th>
            <th class="text-right">Total</th>
          </tr>
        </thead>
        <tbody>
          {#each invoice.items as item}
            {@const unitPrice = item.unit_price_da !== undefined && item.unit_price_da !== null ? item.unit_price_da : (item.line_total_da / (item.quantity_items || 1))}
            <tr>
              <td class="col-name">{item.drug_name}</td>
              <td class="text-center">x{item.quantity_items}</td>
              <td class="text-right">{unitPrice.toFixed(2)}</td>
              <td class="text-right bold">{item.line_total_da.toFixed(2)}</td>
            </tr>
          {/each}
        </tbody>
      </table>

      <hr class="receipt-divider" />
      <div class="receipt-total">
        <span>TOTAL :</span>
        <span>{invoice.total_da.toFixed(2)} DA</span>
      </div>

      <div class="receipt-footer-msg">
        <div>Merci de votre visite !</div>
        <div class="receipt-footer-sub">Veuillez conserver ce ticket</div>
      </div>
    </div>

    <!-- Print Action Button -->
    <div class="invoice-modal-actions mt-1">
      <button 
        type="button" 
        class="btn-action btn-action-primary btn-print-receipt" 
        onclick={handlePrint}
        disabled={isPrinting}
      >
        <img src={printerIcon} alt="Print" class="btn-print-icon" />
        <span>{t("printInvoice")}</span>
      </button>
    </div>
  </div>
</div>

<style>
  /* Receipt layout */
  .invoice-modal {
    position: relative;
    max-width: 440px;
    width: 90%;
    max-height: 90vh;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    padding: clamp(1.5rem, 2.5vw, 2.2rem) clamp(1rem, 2vw, 1.5rem) clamp(0.75rem, 1.5vw, 1.25rem);
    box-sizing: border-box;
  }

  .btn-close-modal {
    position: absolute;
    top: 0.75rem;
    background: transparent;
    border: none;
    cursor: pointer;
    padding: 0.2rem;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: transform 0.1s ease;
    z-index: 10;
  }

  .btn-close-modal:hover {
    transform: scale(1.15);
  }

  .close-icon-img {
    width: 24px;
    height: 24px;
    object-fit: contain;
  }

  :global([dir="ltr"]) .btn-close-modal {
    right: 0.75rem;
  }

  :global([dir="rtl"]) .btn-close-modal {
    left: 0.75rem;
  }

  .receipt-paper {
    background-color: #fcfcfc;
    border: 1px dashed #555;
    padding: clamp(0.75rem, 1.5vw, 1.25rem);
    color: #000;
    font-family: 'Courier New', Courier, monospace;
    font-size: clamp(0.85rem, 0.95vw, 0.95rem);
    flex: 1;
    min-height: 0;
    max-height: calc(85vh - 110px);
    overflow-y: auto;
  }

  .receipt-header-center {
    text-align: center;
    margin-bottom: 0.5rem;
  }

  .receipt-pharmacy {
    font-size: 1.25rem;
    font-weight: 900;
    letter-spacing: 0.5px;
    color: #000;
  }

  .receipt-address {
    font-size: 0.85rem;
    color: #333;
    margin-top: 0.2rem;
  }

  .receipt-title {
    text-align: center;
    font-size: 1.05rem;
    font-weight: bold;
    margin: 0.35rem 0;
    color: #000;
  }

  .receipt-loan-banner {
    background-color: #f4f5f7;
    color: #000;
    padding: 6px 10px;
    border: 1px dashed #333;
    font-weight: bold;
    text-align: center;
    margin: 6px 0;
    font-size: 0.85rem;
  }

  .receipt-meta p {
    margin: 0.2rem 0;
    font-size: 0.9rem;
    line-height: 1.35;
  }

  .receipt-divider {
    border: none;
    border-top: 1px dashed #555;
    margin: 0.65rem 0;
  }

  .receipt-table {
    width: 100%;
    border-collapse: collapse;
  }

  .receipt-table th, .receipt-table td {
    border: none;
    padding: 0.25rem 0;
    font-size: 0.88rem;
    line-height: 1.3;
  }

  .receipt-table th {
    font-weight: bold;
    border-bottom: 1px dashed #777;
    padding-bottom: 0.3rem;
  }

  .col-name {
    text-align: left;
    max-width: 180px;
    word-break: break-word;
  }

  .text-center {
    text-align: center;
  }

  .text-right {
    text-align: right;
  }

  .bold {
    font-weight: bold;
  }

  .receipt-total {
    display: flex;
    justify-content: space-between;
    font-size: 1.25rem;
    font-weight: 900;
    padding: 0.2rem 0;
  }

  .receipt-footer-msg {
    text-align: center;
    font-size: 0.85rem;
    margin-top: 0.75rem;
    color: #333;
  }

  .receipt-footer-sub {
    font-size: 0.78rem;
    margin-top: 0.2rem;
  }

  .invoice-modal-actions {
    display: flex;
    justify-content: center;
    align-items: center;
    width: 100%;
    margin-top: 1.25rem;
  }

  .btn-print-receipt {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.65rem;
    font-size: 1.15rem;
    font-weight: bold;
    padding: 0.85rem 2rem;
    box-shadow: 0 4px 10px rgba(0, 135, 90, 0.25);
  }

  .btn-print-icon {
    width: 26px;
    height: 26px;
    object-fit: contain;
    filter: brightness(0) invert(1);
  }
</style>
