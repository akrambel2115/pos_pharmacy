<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import { t } from "$lib/i18n/index.svelte";
  import { formatDateFr, calculateRemainingTreatmentDays } from "$lib/utils";
  import viewIcon from "../../public/icons/view.png";
  import checkIcon from "../../public/icons/check.png";
  import printerIcon from "../../public/icons/printer.png";
  import InvoiceModal from "./InvoiceModal.svelte";
  import { permissionsStore } from "$lib/permissions.svelte";
  import { printThermalInvoice } from "$lib/receipt";

  let { userRole = "cashier", triggerAdminPIN } = $props<{ 
    userRole?: string | null;
    triggerAdminPIN?: (action: () => void) => void;
  }>();
  let isAdmin = $derived(userRole === "admin");

  let loans = $state<any[]>([]);
  let searchQuery = $state("");
  let statusFilter = $state<"all" | "active" | "settled">("all");
  let selectedInvoice = $state<any | null>(null);
  let errorMsg = $state("");
  let successMsg = $state("");

  async function loadLoans() {
    errorMsg = "";
    try {
      loans = await invoke<any[]>("get_loans", { admin: isAdmin });
    } catch (err: any) {
      errorMsg = err.toString();
    }
  }

  onMount(() => {
    loadLoans();
  });

  // Filtered loans list matching history view logic
  let filteredLoans = $derived.by(() => {
    return loans.filter((loan) => {
      if (statusFilter === "active" && loan.loan_status !== "active") return false;
      if (statusFilter === "settled" && loan.loan_status !== "settled") return false;

      const q = searchQuery.toLowerCase().trim();
      if (!q) return true;

      const patMatch = loan.patient_name?.toLowerCase().includes(q) ?? false;
      const birthMatch = loan.patient_birth_date?.toLowerCase().includes(q) ?? false;
      const docMatch = loan.prescribing_doctor_name?.toLowerCase().includes(q) ?? false;
      const idMatch = loan.id?.toString().includes(q) ?? false;
      const totalMatch = loan.total_da?.toString().includes(q) ?? false;
      const dateMatch = loan.created_at?.toLowerCase().includes(q) ?? false;

      return patMatch || birthMatch || docMatch || idMatch || totalMatch || dateMatch;
    });
  });

  async function handleSettleLoan(loanId: number) {
    const proceed = async () => {
      if (!confirm(t("settleLoanConfirm"))) return;

      try {
        await invoke("settle_loan", { saleId: loanId });
        successMsg = t("loanSettledSuccess");
        setTimeout(() => (successMsg = ""), 3500);
        await loadLoans();
      } catch (err: any) {
        errorMsg = err.toString();
        setTimeout(() => (errorMsg = ""), 4000);
      }
    };

    if (permissionsStore.isAllowed(userRole, "can_settle_loans")) {
      await proceed();
    } else if (triggerAdminPIN) {
      triggerAdminPIN(proceed);
    } else {
      alert("Action réservée à l'administrateur");
    }
  }
</script>

<div class="history-workspace">
  <!-- Search and filter row identical to History and Stock -->
  <div class="search-filter-row">
    <input
      type="text"
      class="input-pos search-input"
      placeholder={t("searchLoansPlaceholder")}
      bind:value={searchQuery}
    />
    <select class="select-pos" bind:value={statusFilter}>
      <option value="all">{t("allLoans")}</option>
      <option value="active">{t("filterActiveLoans")}</option>
      <option value="settled">{t("filterSettledLoans")}</option>
    </select>
  </div>

  {#if errorMsg}
    <div class="error-banner mt-1">{errorMsg}</div>
  {/if}
  {#if successMsg}
    <div class="success-banner mt-1">{successMsg}</div>
  {/if}

  <!-- Table identical to History and Stock -->
  <div class="table-scroll-container">
    {#if filteredLoans.length === 0}
      <p class="empty-text">{t("noLoansFound")}</p>
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
            <th>{t("status")}</th>
            <th>{t("actions")}</th>
          </tr>
        </thead>
        <tbody>
          {#each filteredLoans as loan}
            {@const remainingInfo = calculateRemainingTreatmentDays(loan.created_at, loan.treatment_period_days)}
            <tr>
              <td class="bold">#{loan.id}</td>
              <td>{formatDateFr(loan.created_at)}</td>
              <td>{loan.patient_name ? loan.patient_name : "-"}</td>
              <td>{loan.patient_birth_date ? formatDateFr(loan.patient_birth_date) : "-"}</td>
              <td>
                {#if loan.treatment_period_days}
                  {loan.treatment_period_days} {t("daysUnit")}
                  {#if remainingInfo}
                    &nbsp;
                    {#if remainingInfo.isPassed}
                      <span class="status-period-passed">({t("periodPassed")})</span>
                    {:else}
                      <span class="status-days-remaining">({t("daysRemaining", { days: remainingInfo.daysLeft })})</span>
                    {/if}
                  {/if}
                {:else}
                  -
                {/if}
              </td>
              <td class="bold text-green">{loan.total_da.toFixed(2)} DA</td>
              <td>
                {#if loan.loan_status === "settled"}
                  <span class="status-ok">{t("loanBadgeSettled")}</span>
                {:else}
                  <span class="badge-warning">{t("filterActiveLoans")}</span>
                {/if}
              </td>
              <td>
                <div class="patient-actions-wrapper">
                  <button
                    onclick={() => selectedInvoice = loan}
                    class="patient-action-btn"
                    title={t("viewDetails")}
                  >
                    <img src={viewIcon} alt="View" class="patient-action-icon" />
                  </button>

                  <button
                    onclick={() => printThermalInvoice(loan)}
                    class="patient-action-btn"
                    title={t("printInvoice")}
                  >
                    <img src={printerIcon} alt="Print" class="patient-action-icon" />
                  </button>

                  {#if loan.loan_status === "active"}
                    <button
                      onclick={() => handleSettleLoan(loan.id)}
                      class="patient-action-btn"
                      title={t("settleLoan")}
                    >
                      <img src={checkIcon} alt="Settle" class="patient-action-icon" />
                    </button>
                  {/if}
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
    title="{t('loanBadge')} #{selectedInvoice.id}"
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
    max-width: 650px;
    flex-wrap: wrap;
  }

  .search-input {
    flex: 1;
    min-width: 200px;
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
    min-width: 820px;
    border-collapse: collapse;
    font-size: clamp(0.9rem, 1.05vw, 1.1rem);
  }

  .pos-table th, .pos-table td {
    padding: clamp(0.45rem, 0.8vh, 0.75rem) clamp(0.5rem, 0.8vw, 1rem);
    border-bottom: var(--border-width) solid var(--color-border);
    text-align: left;
    vertical-align: middle;
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

  .status-ok {
    background-color: var(--color-primary);
    color: #ffffff;
    padding: 0.2rem 0.6rem;
    border-radius: 4px;
    font-weight: bold;
    display: inline-block;
    font-size: 0.95rem;
  }

  .badge-warning {
    background-color: var(--color-warning);
    color: var(--color-text-dark);
    padding: 0.2rem 0.6rem;
    border-radius: 4px;
    font-weight: bold;
    display: inline-block;
    font-size: 0.95rem;
  }

  .status-days-remaining {
    color: var(--color-danger);
    font-weight: 700;
    font-size: 0.95rem;
  }

  .status-period-passed {
    color: var(--color-primary);
    font-weight: 700;
    font-size: 0.95rem;
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

  .success-banner {
    background-color: #e3fcef;
    border: 2px solid var(--color-primary);
    color: var(--color-primary);
    padding: 0.75rem;
    border-radius: 8px;
    text-align: center;
    font-weight: bold;
  }
</style>
