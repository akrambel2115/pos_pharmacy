<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { t } from "$lib/i18n/index.svelte";
  import { onMount } from "svelte";
  import { formatDateFr } from "$lib/utils";
  import InvoiceModal from "./InvoiceModal.svelte";

  // Icons
  import plusIcon from "../../public/icons/plus.png";
  import viewIcon from "../../public/icons/view.png";
  import editIcon from "../../public/icons/edit.png";
  import closeIcon from "../../public/icons/close.png";
  import trashIcon from "../../public/icons/trash.png";
  import { permissionsStore } from "$lib/permissions.svelte";

  interface PatientItem {
    id: number;
    name: string;
    birth_date: string;
    last_purchase_date: string | null;
  }

  interface LastInvoice {
    id: number;
    prescribing_doctor_name: string | null;
    patient_name: string | null;
    patient_birth_date?: string | null;
    treatment_period_days?: number | null;
    total_da: number;
    created_at: string;
    items: any[];
  }

  // Props
  let { userRole = "cashier", triggerAdminPIN } = $props<{ 
    userRole?: string;
    triggerAdminPIN?: (action: () => void) => void;
  }>();
  let isAdmin = $derived(userRole === "admin");

  // State
  let patientsList = $state<PatientItem[]>([]);
  let searchQuery = $state("");
  let currentPage = $state(1);
  const itemsPerPage = 10;

  // Add/Edit Modal state
  let showAddEditModal = $state(false);
  let isEditing = $state(false);
  let selectedPatient = $state<PatientItem | null>(null);
  let patientName = $state("");
  let patientBirthDate = $state("");
  let modalError = $state("");

  // Invoices list modal state
  let showInvoicesModal = $state(false);
  let patientInvoices = $state<LastInvoice[]>([]);
  let invoicesLoading = $state(false);
  let selectedInvoiceForView = $state<LastInvoice | null>(null);
  let showInvoiceDetails = $state(false);

  // Load patients list
  async function fetchPatients() {
    try {
      patientsList = await invoke<PatientItem[]>("get_patients_list");
    } catch (err: any) {
      console.error("Failed to fetch patients:", err);
    }
  }

  onMount(() => {
    fetchPatients();
  });

  // Filter patients
  let filteredPatients = $derived.by(() => {
    const query = searchQuery.trim().toLowerCase();
    if (!query) return patientsList;
    return patientsList.filter(
      (p) =>
        p.name.toLowerCase().includes(query) ||
        p.birth_date.includes(query)
    );
  });

  // Pagination
  let totalPages = $derived(Math.ceil(filteredPatients.length / itemsPerPage));
  let paginatedPatients = $derived.by(() => {
    const start = (currentPage - 1) * itemsPerPage;
    return filteredPatients.slice(start, start + itemsPerPage);
  });

  // Handlers for Add/Edit
  function openAddModal() {
    isEditing = false;
    selectedPatient = null;
    patientName = "";
    patientBirthDate = "";
    modalError = "";
    showAddEditModal = true;
  }

  function openEditModal(patient: PatientItem) {
    isEditing = true;
    selectedPatient = patient;
    patientName = patient.name;
    patientBirthDate = patient.birth_date;
    modalError = "";
    showAddEditModal = true;
  }

  async function handleSavePatient(e: SubmitEvent) {
    e.preventDefault();
    modalError = "";

    const nameTrimmed = patientName.trim();
    const birthTrimmed = patientBirthDate.trim();

    if (!nameTrimmed || !birthTrimmed) {
      modalError = "Tous les champs obligatoires doivent être renseignés.";
      return;
    }

    try {
      if (isEditing && selectedPatient) {
        await invoke("update_customer", {
          id: selectedPatient.id,
          name: nameTrimmed,
          birthDate: birthTrimmed,
        });
      } else {
        await invoke("add_customer", {
          name: nameTrimmed,
          birthDate: birthTrimmed,
        });
      }
      showAddEditModal = false;
      fetchPatients();
    } catch (err: any) {
      const errStr = err?.toString() || "";
      if (errStr.includes("PATIENT_ALREADY_EXISTS")) {
        modalError = t("patientDuplicateError");
      } else {
        modalError = errStr;
      }
    }
  }

  // Related invoices modal
  async function openPatientInvoices(patient: PatientItem) {
    selectedPatient = patient;
    patientInvoices = [];
    invoicesLoading = true;
    showInvoicesModal = true;

    try {
      patientInvoices = await invoke<LastInvoice[]>("get_customer_sales", {
        customerId: patient.id,
        admin: isAdmin
      });
    } catch (err: any) {
      console.error("Failed to load patient invoices:", err);
    } finally {
      invoicesLoading = false;
    }
  }

  function viewInvoiceDetails(invoice: LastInvoice) {
    selectedInvoiceForView = invoice;
    showInvoiceDetails = true;
  }

  async function handleDeletePatient(patient: PatientItem) {
    const proceed = async () => {
      if (confirm(`Êtes-vous sûr de vouloir supprimer le patient ${patient.name} ?`)) {
        try {
          await invoke("delete_customer", { id: patient.id });
          await fetchPatients();
        } catch (err: any) {
          alert("Erreur lors de la suppression : " + err.toString());
        }
      }
    };

    if (permissionsStore.isAllowed(userRole, "can_delete_patient")) {
      await proceed();
    } else if (triggerAdminPIN) {
      triggerAdminPIN(proceed);
    } else {
      alert("Action réservée à l'administrateur");
    }
  }
</script>

<div class="patients-workspace">
  <!-- Search and Actions Header -->
  <div class="stock-header-actions mt-1">
    <div class="search-filter-row">
      <input
        type="text"
        class="input-pos search-input"
        placeholder={t("searchPatient")}
        bind:value={searchQuery}
        oninput={() => currentPage = 1}
      />
      <button onclick={openAddModal} class="btn-add-manual-stock" title={t("addCustomer")}>
        <img src={plusIcon} alt="Add" class="plus-icon-img" />
      </button>
    </div>
  </div>

  <!-- Patients List Table -->
  <div class="cart-section">
    <div class="table-scroll-container">
      <table class="pos-table">
        <thead>
          <tr>
            <th>{t("name")}</th>
            <th>{t("birthDate")}</th>
            <th>{t("lastPurchase")}</th>
            <th>{t("actions")}</th>
          </tr>
        </thead>
        <tbody>
          {#each paginatedPatients as patient}
            <tr>
              <td class="bold">{patient.name}</td>
              <td>{formatDateFr(patient.birth_date)}</td>
              <td>
                {#if patient.last_purchase_date}
                  {formatDateFr(patient.last_purchase_date)}
                {:else}
                  <span class="empty-text">{t("noPurchase")}</span>
                {/if}
              </td>
              <td>
                <div class="patient-actions-wrapper">
                  <!-- View invoices -->
                  <button onclick={() => openPatientInvoices(patient)} class="patient-action-btn" title="Voir les factures">
                    <img src={viewIcon} alt="View" class="patient-action-icon" />
                  </button>
                  <!-- Edit patient -->
                  <button onclick={() => openEditModal(patient)} class="patient-action-btn" title="Modifier le patient">
                    <img src={editIcon} alt="Edit" class="patient-action-icon" />
                  </button>
                  <!-- Delete patient -->
                  <button onclick={() => handleDeletePatient(patient)} class="patient-action-btn" title="Supprimer le patient">
                    <img src={trashIcon} alt="Delete" class="patient-action-icon" />
                  </button>
                </div>
              </td>
            </tr>
          {/each}
          {#each Array(Math.max(0, 10 - paginatedPatients.length)) as _}
            <tr class="placeholder-row">
              <td>&nbsp;</td>
              <td>&nbsp;</td>
              <td>&nbsp;</td>
              <td>&nbsp;</td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>

    <!-- Pagination controls -->
    {#if totalPages > 1}
      <div class="pagination-container">
        <button
          class="btn-pos-action"
          disabled={currentPage === 1}
          onclick={() => currentPage = Math.max(1, currentPage - 1)}
        >
          &lt;
        </button>
        <span class="page-indicator">{currentPage} / {totalPages}</span>
        <button
          class="btn-pos-action"
          disabled={currentPage === totalPages}
          onclick={() => currentPage = Math.min(totalPages, currentPage + 1)}
        >
          &gt;
        </button>
      </div>
    {/if}
  </div>
</div>

<!-- Add/Edit Patient Modal -->
{#if showAddEditModal}
  <div class="modal-overlay">
    <div class="modal-content client-modal">
      <h3>{isEditing ? t("editPatient") : t("addCustomer")}</h3>
      <form onsubmit={handleSavePatient} class="client-form mt-1">
        {#if modalError}
          <div class="error-banner">{modalError}</div>
        {/if}

        <div class="form-group">
          <label for="p-name">{t("name")} *</label>
          <input
            id="p-name"
            type="text"
            class="input-pos"
            bind:value={patientName}
            required
            placeholder="Nom et Prénom"
          />
        </div>

        <div class="form-group mt-1">
          <label for="p-birth">{t("birthDate")} *</label>
          <input
            id="p-birth"
            type="date"
            class="input-pos"
            bind:value={patientBirthDate}
            required
          />
        </div>

        <div class="form-actions mt-2">
          <button type="submit" class="btn-action btn-action-primary">
            {t("save")}
          </button>
          <button type="button" onclick={() => showAddEditModal = false} class="btn-action btn-action-danger">
            Annuler
          </button>
        </div>
      </form>
    </div>
  </div>
{/if}

<!-- Patient Related Invoices Modal -->
{#if showInvoicesModal && selectedPatient}
  <div class="modal-overlay">
    <div class="modal-content card max-w-lg">
      <div class="modal-header">
        <h3>{t("patientInvoices")} - {selectedPatient.name}</h3>
        <button class="btn-close-modal" onclick={() => showInvoicesModal = false}>&times;</button>
      </div>
      <div class="modal-body mt-1">
        {#if invoicesLoading}
          <p class="empty-text">Chargement des factures...</p>
        {:else if patientInvoices.length === 0}
          <p class="empty-text">{t("noSalesFound")}</p>
        {:else}
          <div class="table-scroll-container" style="max-height: 350px; overflow-y: auto;">
            <table class="pos-table">
              <thead>
                <tr>
                  <th>N°</th>
                  <th>{t("date")}</th>
                  <th>{t("treatmentPeriodDays")}</th>
                  <th>{t("total")}</th>
                  <th>Action</th>
                </tr>
              </thead>
              <tbody>
                {#each patientInvoices as invoice}
                  <tr>
                    <td class="bold">#{invoice.id}</td>
                    <td>{formatDateFr(invoice.created_at)}</td>
                    <td>{invoice.treatment_period_days ? invoice.treatment_period_days + " " + t("daysUnit") : "-"}</td>
                    <td>{invoice.total_da.toFixed(2)} {t("da")}</td>
                    <td>
                      <button onclick={() => viewInvoiceDetails(invoice)} class="patient-action-btn" title="Voir reçu">
                        <img src={viewIcon} alt="View" class="patient-action-icon" />
                      </button>
                    </td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        {/if}
      </div>
    </div>
  </div>
{/if}

<!-- Receipt details nested view -->
{#if showInvoiceDetails && selectedInvoiceForView}
  <InvoiceModal
    invoice={selectedInvoiceForView}
    onClose={() => showInvoiceDetails = false}
  />
{/if}

<style>
  .patients-workspace {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    width: 100%;
    height: 100%;
    box-sizing: border-box;
  }

  .stock-header-actions {
    display: flex;
    justify-content: space-between;
    align-items: center;
    width: 100%;
    box-sizing: border-box;
  }

  .search-filter-row {
    display: flex;
    gap: clamp(0.5rem, 1vw, 1rem);
    flex: 1;
    max-width: 600px;
    align-items: center;
  }

  .search-input {
    flex: 1;
  }

  .btn-add-manual-stock {
    background-color: transparent;
    border: none;
    padding: 0;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: transform 0.15s ease;
  }

  .btn-add-manual-stock:hover {
    transform: scale(1.15);
  }

  .plus-icon-img {
    width: clamp(26px, 3vw, 32px);
    height: clamp(26px, 3vw, 32px);
    object-fit: contain;
  }

  .cart-section {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    min-height: 0;
  }

  .table-scroll-container {
    flex: 1;
    overflow: auto;
    width: 100%;
    min-height: 0;
  }

  /* Table styling */
  .pos-table {
    width: 100%;
    min-width: 600px;
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

  .pos-table td.bold {
    font-weight: bold;
  }

  .placeholder-row td {
    height: clamp(32px, 3.8vh, 50px);
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
    width: clamp(20px, 1.8vw, 24px);
    height: clamp(20px, 1.8vw, 24px);
    object-fit: contain;
  }

  /* Pagination */
  .pagination-container {
    display: flex;
    justify-content: center;
    align-items: center;
    gap: 1.5rem;
    margin-top: 0.5rem;
  }

  .btn-pos-action {
    background-color: var(--color-bg-app);
    border: var(--border-width) solid var(--color-border);
    border-radius: var(--border-radius);
    padding: 0.4rem 0.8rem;
    font-size: clamp(0.95rem, 1vw, 1.1rem);
    font-weight: bold;
    cursor: pointer;
  }

  .btn-pos-action:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .page-indicator {
    font-size: clamp(0.95rem, 1.1vw, 1.15rem);
    font-weight: bold;
  }

  /* Modal styling */
  .client-modal {
    max-width: 450px;
    padding: 1.5rem;
  }

  .client-form {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .form-group {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .form-actions {
    display: flex;
    gap: 1rem;
    justify-content: flex-end;
  }

  .error-banner {
    background-color: #ffebee;
    color: var(--color-danger);
    padding: 0.75rem;
    border-radius: var(--border-radius);
    border: 2px solid var(--color-danger);
    font-weight: bold;
  }

  .modal-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-bottom: 2px solid var(--color-bg-app);
    padding-bottom: 0.75rem;
  }

  .btn-close-modal {
    background: transparent;
    border: none;
    font-size: 1.8rem;
    cursor: pointer;
    font-weight: bold;
    line-height: 1;
    padding: 0;
  }

  .max-w-lg {
    max-width: 600px;
  }

  .empty-text {
    color: var(--color-border);
    font-style: italic;
  }
</style>

