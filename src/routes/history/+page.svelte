<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { t } from "$lib/i18n/index.svelte";
  import { onMount } from "svelte";
  import viewIcon from "../../public/icons/view.png";
  import printerIcon from "../../public/icons/printer.png";
  import InvoiceModal from "$lib/components/InvoiceModal.svelte";
  import { formatDateFr } from "$lib/utils";
  import { printThermalInvoice } from "$lib/receipt";

  let { isAdmin = false } = $props<{ isAdmin?: boolean }>();
  let salesHistory = $state<any[]>([]);
  let searchQuery = $state("");
  let selectedInvoice = $state<any | null>(null);
  let errorMsg = $state("");

  async function handlePrintDirect(sale: any) {
    try {
      await printThermalInvoice(sale);
    } catch (err) {
      console.error("Failed to print sale invoice:", err);
    }
  }

  onMount(async () => {
    try {
      salesHistory = await invoke<any[]>("get_sales_history", { admin: isAdmin });
    } catch (err: any) {
      errorMsg = err.toString();
    }
  });

  // Derived filtered sales history list
  let filteredSales = $derived(
    salesHistory.filter(sale => {
      const q = searchQuery.toLowerCase().trim();
      if (!q) return true;
      
      const docMatch = sale.prescribing_doctor_name?.toLowerCase().includes(q) ?? false;
      const patMatch = sale.patient_name?.toLowerCase().includes(q) ?? false;
      const birthMatch = sale.patient_birth_date?.toLowerCase().includes(q) ?? false;
      const totalMatch = sale.total_da.toString().includes(q);
      const invoiceNoMatch = sale.id.toString().includes(q);
      
      return docMatch || patMatch || birthMatch || totalMatch || invoiceNoMatch;
    })
  );
</script>

<div class="history-workspace">
  <div class="search-filter-row">
    <input
      type="text"
      class="input-pos search-input"
      placeholder="Rechercher par N° Facture, Patient, Date..."
      bind:value={searchQuery}
    />
  </div>

  {#if errorMsg}
    <div class="error-banner mt-1">{errorMsg}</div>
  {/if}

  <div class="table-scroll-container">
    {#if filteredSales.length === 0}
      <p class="empty-text">{t("noSalesFound")}</p>
    {:else}
      <table class="pos-table">
        <thead>
          <tr>
            <th>{t("invoiceNo")}</th>
            <th>{t("date")}</th>
            <th>{t("patientName")}</th>
            <th>{t("birthDate")}</th>
            <th>{t("treatmentPeriodDays")}</th>
            <th>{t("total")}</th>
            <th>{t("actions")}</th>
          </tr>
        </thead>
        <tbody>
          {#each filteredSales as sale}
            <tr>
              <td class="bold">#{sale.id}</td>
              <td>{formatDateFr(sale.created_at)}</td>
              <td>{sale.patient_name ? sale.patient_name : "-"}</td>
              <td>{sale.patient_birth_date ? formatDateFr(sale.patient_birth_date) : "-"}</td>
              <td>{sale.treatment_period_days ? sale.treatment_period_days + " " + t("daysUnit") : "-"}</td>
              <td class="bold text-green">{sale.total_da.toFixed(2)} DA</td>
              <td>
                <div class="patient-actions-wrapper">
                  <button onclick={() => selectedInvoice = sale} class="patient-action-btn" title="Voir détails">
                    <img src={viewIcon} alt="View" class="patient-action-icon" />
                  </button>
                  <button onclick={() => handlePrintDirect(sale)} class="patient-action-btn" title={t("printInvoice")}>
                    <img src={printerIcon} alt="Print" class="patient-action-icon" />
                  </button>
                </div>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  </div>
</div>

<!-- Detailed Invoice Modal View -->
{#if selectedInvoice}
  <InvoiceModal
    invoice={selectedInvoice}
    onClose={() => selectedInvoice = null}
    title="TICKET DE CAISSE"
  />
{/if}

<style>
  .history-workspace {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    width: 100%;
    height: 100%;
    box-sizing: border-box;
  }

  .search-filter-row {
    display: flex;
    gap: 0.75rem;
    width: 100%;
    max-width: 600px;
    flex-wrap: wrap;
  }

  .search-input {
    flex: 1;
    min-width: 220px;
    font-size: clamp(0.95rem, 1.1vw, 1.15rem);
  }

  .table-scroll-container {
    flex: 1;
    overflow: auto;
    width: 100%;
    min-height: 0;
  }

  .mt-1 {
    margin-top: 0.5rem;
  }

  .pos-table {
    width: 100%;
    min-width: 780px;
    border-collapse: collapse;
    font-size: clamp(0.9rem, 1.05vw, 1.1rem);
  }

  .pos-table th, .pos-table td {
    padding: clamp(0.45rem, 0.8vh, 0.75rem) clamp(0.5rem, 0.8vw, 1rem);
    border-bottom: var(--border-width) solid var(--color-border);
    text-align: left;
  }

  :global([dir="rtl"]) .pos-table th, :global([dir="rtl"]) .pos-table td {
    text-align: right;
  }

  .pos-table th {
    position: sticky;
    top: 0;
    background-color: var(--color-bg-app);
    font-weight: bold;
    z-index: 10;
  }

  .bold {
    font-weight: bold;
  }

  .text-green {
    color: var(--color-primary);
  }

  .patient-actions-wrapper {
    display: flex;
    gap: 0.5rem;
    align-items: center;
  }

  .patient-action-btn {
    background: transparent;
    border: none;
    padding: 0.2rem;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: transform 0.1s ease;
  }

  .patient-action-btn:hover {
    transform: scale(1.15);
  }

  .patient-action-icon {
    width: 24px;
    height: 24px;
    object-fit: contain;
  }

  .empty-text {
    color: var(--color-border);
    font-style: italic;
    text-align: center;
    padding: 2rem;
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

</style>
