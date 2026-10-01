<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { t } from "$lib/i18n/index.svelte";
  import BarcodeScanner from "./BarcodeScanner.svelte";
  import { formatDateFr, calculateRemainingTreatmentDays } from "../utils";
  import trashIcon from "../../public/icons/trash.png";
  import plusIcon from "../../public/icons/plus.png";
  import checkIcon from "../../public/icons/check.png";
  import closeIcon from "../../public/icons/close.png";
  import viewIcon from "../../public/icons/view.png";
  import closepIcon from "../../public/icons/closep.png";
  import InvoiceModal from "./InvoiceModal.svelte";
  import { cartStore } from "$lib/cart.svelte";
  import { permissionsStore } from "$lib/permissions.svelte";

  let { userRole, triggerAdminPIN } = $props<{
    userRole: "cashier" | "admin";
    triggerAdminPIN?: (action: () => void) => void;
  }>();

  // Cart State (stored in cartStore to persist across workspace navigation)
  let cart = $derived(cartStore.cart);
  let selectedCustomer = $derived(cartStore.selectedCustomer);
  let lastInvoice = $derived(cartStore.lastInvoice);
  let cartTotal = $derived(cartStore.cartTotal);
  let requiresPrescription = $derived(cartStore.requiresPrescription);
  let lastInvoiceTreatmentStatus = $derived(cartStore.lastInvoiceTreatmentStatus);

  let customerSuggestions = $state<any[]>([]);
  let showSuggestions = $state(false);

  // Scanner & Modals state
  let scannedBarcode = $state("");
  let scannerError = $state("");
  let completedInvoice = $state<any | null>(null); // holds invoice for printed mockup modal

  // Customer Add Modal state
  let showAddCustomerModal = $state(false);
  let newCustomerName = $state("");
  let newCustomerBirthDate = $state("");
  let addCustomerError = $state("");

  // Treatment Period Modal state
  let showTreatmentPeriodModal = $state(false);
  let treatmentDaysError = $state("");

  // Loan Confirmation Modal state
  let showLoanConfirmModal = $state(false);
  let isPendingLoan = $state(false);

  // Active loans & Deduction Modal state
  let activeLoansForCustomer = $state<any[]>([]);
  let showLoanDeductionModal = $state(false);
  let detectedLoanMatches = $state<any[]>([]);
  let pendingLoanDeductions = $state<any[]>([]);
  let loanDeductionDecisionMade = $state(false);

  // Patient Details Modal state
  let selectedPatientForView = $state<any | null>(null);
  let showViewPatientModal = $state(false);

  async function handleScan(barcode: string) {
    scannerError = "";
    if (!selectedCustomer) {
      scannerError = t("selectCustomerFirst");
      setTimeout(() => {
        if (scannerError === t("selectCustomerFirst")) {
          scannerError = "";
        }
      }, 4000);
      return;
    }
    try {
      const drug = await invoke<any>("get_drug_by_barcode", { barcode });
      if (drug) {
        addToCart(drug);
      } else {
        scannerError = t("itemNotInStock");
        setTimeout(() => {
          if (scannerError === t("itemNotInStock")) {
            scannerError = "";
          }
        }, 4000);
      }
    } catch (err) {
      console.error(err);
    }
  }

  // Drug search state
  let drugSuggestions = $state<any[]>([]);
  let showDrugSuggestions = $state(false);

  async function handleDrugSearchInput(e: Event) {
    const val = (e.target as HTMLInputElement).value;
    cartStore.drugSearchQuery = val;
    if (val.trim().length >= 2) {
      try {
        drugSuggestions = await invoke<any[]>("search_drugs", { query: val.trim() });
        showDrugSuggestions = true;
      } catch (err) {
        console.error(err);
      }
    } else {
      drugSuggestions = [];
      showDrugSuggestions = false;
    }
  }

  function selectDrugFromSearch(drug: any) {
    if (!selectedCustomer) {
      scannerError = t("selectCustomerFirst");
      setTimeout(() => {
        if (scannerError === t("selectCustomerFirst")) {
          scannerError = "";
        }
      }, 4000);
      return;
    }
    cartStore.addToCart(drug);
    cartStore.drugSearchQuery = "";
    drugSuggestions = [];
    showDrugSuggestions = false;
  }

  function resetLoanDeductionState() {
    loanDeductionDecisionMade = false;
    pendingLoanDeductions = [];
    detectedLoanMatches = [];
  }

  function addToCart(drug: any) {
    cartStore.addToCart(drug);
    resetLoanDeductionState();
  }

  function removeFromCart(drugId: number) {
    cartStore.removeFromCart(drugId);
    resetLoanDeductionState();
  }

  function updateQuantity(drugId: number, delta: number) {
    cartStore.updateQuantity(drugId, delta);
    resetLoanDeductionState();
  }

  async function refreshSuggestions() {
    if (cartStore.customerInput.trim().length >= 2) {
      try {
        customerSuggestions = await invoke<any[]>("search_customers", { query: cartStore.customerInput });
        showSuggestions = true;
      } catch (err) {
        console.error(err);
      }
    } else {
      customerSuggestions = [];
      showSuggestions = false;
    }
  }

  async function handleCustomerInput(e: Event) {
    const val = (e.target as HTMLInputElement).value;
    cartStore.customerInput = val;
    await refreshSuggestions();
  }

  async function selectCustomer(customer: any) {
    let invoice = null;
    try {
      invoice = await invoke<any>("get_last_invoice", { customerId: customer.id });
    } catch (err) {
      console.error(err);
    }
    try {
      activeLoansForCustomer = await invoke<any[]>("get_patient_active_loans", { customerId: customer.id });
    } catch (err) {
      console.error(err);
      activeLoansForCustomer = [];
    }
    cartStore.selectCustomer(customer, invoice);
    showSuggestions = false;
    resetLoanDeductionState();
  }

  function clearCustomer() {
    cartStore.clearCustomer();
    activeLoansForCustomer = [];
    resetLoanDeductionState();
  }

  async function handleManualAdd() {
    scannerError = "";
    if (!selectedCustomer) {
      scannerError = t("selectCustomerFirst");
      setTimeout(() => {
        if (scannerError === t("selectCustomerFirst")) {
          scannerError = "";
        }
      }, 4000);
      return;
    }
    let barcode = prompt("Entrez le code-barres du produit :");
    if (barcode === null) return; // user cancelled
    barcode = barcode.trim();
    if (!barcode) return;
    
    try {
      const drug = await invoke<any>("get_drug_by_barcode", { barcode });
      if (drug) {
        addToCart(drug);
      } else {
        scannerError = t("itemNotInStock");
        setTimeout(() => {
          if (scannerError === t("itemNotInStock")) {
            scannerError = "";
          }
        }, 4000);
      }
    } catch (err) {
      console.error(err);
    }
  }

  async function handleAddCustomerSubmit(e: SubmitEvent) {
    e.preventDefault();
    addCustomerError = "";
    if (!newCustomerName.trim()) {
      addCustomerError = "Le nom est obligatoire";
      return;
    }
    if (!newCustomerBirthDate.trim()) {
      addCustomerError = t("birthDateRequired");
      return;
    }

    try {
      const customer = await invoke<any>("add_customer", {
        name: newCustomerName.trim(),
        birthDate: newCustomerBirthDate.trim(),
      });
      selectCustomer(customer);
      // Clear fields
      newCustomerName = "";
      newCustomerBirthDate = "";
      showAddCustomerModal = false;
    } catch (err: any) {
      const errMsg = err?.toString() || "";
      if (errMsg.includes("PATIENT_ALREADY_EXISTS")) {
        addCustomerError = t("patientDuplicateError");
      } else {
        addCustomerError = errMsg;
      }
    }
  }

  function viewPatientDetails(patient: any) {
    selectedPatientForView = patient;
    showViewPatientModal = true;
  }

  async function deletePatient(patientId: number) {
    if (confirm("Voulez-vous vraiment supprimer ce patient ? / هل تريد حقًا حذف هذا المريض؟")) {
      try {
        await invoke("delete_customer", { id: patientId });
        if (selectedCustomer && selectedCustomer.id === patientId) {
          clearCustomer();
        }
        await refreshSuggestions();
      } catch (err: any) {
        scannerError = err.toString();
      }
    }
  }

  function handleValidateClick() {
    scannerError = "";
    if (cart.length === 0) {
      scannerError = t("cartEmptyError");
      return;
    }
    if (requiresPrescription && (!cartStore.patientName.trim() || !cartStore.doctorName.trim())) {
      scannerError = t("prescriptionInputError");
      return;
    }

    // Step 1: Check if patient has any active loans on the medicines currently in the cart
    if (selectedCustomer && activeLoansForCustomer.length > 0 && !loanDeductionDecisionMade) {
      const matching: any[] = [];
      for (const item of cart) {
        const matches = activeLoansForCustomer.filter(l => l.drug_id === item.drug.id);
        for (const m of matches) {
          const deductQty = Math.min(item.quantity, m.quantity);
          const deliverQty = Math.max(0, item.quantity - deductQty);
          matching.push({
            cartItem: item,
            loan: m,
            deductQty,
            deliverQty,
          });
        }
      }
      if (matching.length > 0) {
        detectedLoanMatches = matching;
        showLoanDeductionModal = true;
        return;
      }
    }

    continueValidationFlow();
  }

  function confirmLoanDeductionAction() {
    pendingLoanDeductions = detectedLoanMatches.map(m => ({
      loan_sale_id: m.loan.sale_id,
      drug_id: m.cartItem.drug.id,
      quantity: m.deductQty,
    }));
    loanDeductionDecisionMade = true;
    showLoanDeductionModal = false;
    continueValidationFlow();
  }

  function declineLoanDeductionAction() {
    pendingLoanDeductions = [];
    loanDeductionDecisionMade = true;
    showLoanDeductionModal = false;
    continueValidationFlow();
  }

  function cancelLoanDeductionAction() {
    showLoanDeductionModal = false;
    loanDeductionDecisionMade = false;
    pendingLoanDeductions = [];
  }

  function continueValidationFlow() {
    // If customer has an active treatment period from their previous sale, prompt for loan
    if (selectedCustomer && lastInvoiceTreatmentStatus && !lastInvoiceTreatmentStatus.isPassed) {
      showLoanConfirmModal = true;
      return;
    }

    isPendingLoan = false;
    cartStore.treatmentDaysInput = 30;
    treatmentDaysError = "";
    showTreatmentPeriodModal = true;
  }

  function confirmAsLoanAction() {
    const proceed = () => {
      isPendingLoan = true;
      showLoanConfirmModal = false;
      cartStore.treatmentDaysInput = 30;
      treatmentDaysError = "";
      showTreatmentPeriodModal = true;
    };

    if (permissionsStore.isAllowed(userRole, "can_give_loans")) {
      proceed();
    } else if (triggerAdminPIN) {
      triggerAdminPIN(proceed);
    } else {
      alert("Action réservée à l'administrateur");
    }
  }

  function proceedNormalSaleAction() {
    isPendingLoan = false;
    showLoanConfirmModal = false;
    cartStore.treatmentDaysInput = 30;
    treatmentDaysError = "";
    showTreatmentPeriodModal = true;
  }

  async function handleConfirmTreatmentPeriod(e?: SubmitEvent) {
    if (e) e.preventDefault();
    treatmentDaysError = "";

    const days = typeof cartStore.treatmentDaysInput === "string" 
      ? parseInt(cartStore.treatmentDaysInput, 10) 
      : cartStore.treatmentDaysInput;
    if (isNaN(days) || days <= 0) {
      treatmentDaysError = "Veuillez entrer un nombre de jours supérieur à 0.";
      return;
    }

    showTreatmentPeriodModal = false;
    await performCheckout(days);
  }

  async function performCheckout(treatmentPeriodDays: number) {
    scannerError = "";
    if (cart.length === 0) {
      scannerError = t("cartEmptyError");
      return;
    }
    if (requiresPrescription && (!cartStore.patientName.trim() || !cartStore.doctorName.trim())) {
      scannerError = t("prescriptionInputError");
      return;
    }

    const itemsInput = cart.map(item => ({
      drug_id: item.drug.id,
      quantity_items: item.quantity,
      unit_price_da: item.drug.price_per_item_da,
    }));

    try {
      const saleId = await invoke<number>("checkout_sale", {
        customerId: selectedCustomer ? selectedCustomer.id : null,
        customerName: cartStore.customerInput.trim() ? cartStore.customerInput : null,
        prescribingDoctorName: cartStore.doctorName.trim() ? cartStore.doctorName : null,
        patientName: cartStore.patientName.trim() ? cartStore.patientName : null,
        treatmentPeriodDays: treatmentPeriodDays,
        cartItems: itemsInput,
        isLoan: isPendingLoan,
        loanDeductions: pendingLoanDeductions.length > 0 ? pendingLoanDeductions : null,
      });

      // Load the completed sale to display the digital invoice
      completedInvoice = await invoke<any>("get_sale_by_id", { 
        saleId, 
        admin: userRole === "admin" 
      });
      
      // Clear Cart & inputs
      cartStore.clearCart();
      isPendingLoan = false;
      resetLoanDeductionState();
      activeLoansForCustomer = [];
    } catch (err: any) {
      scannerError = err.toString();
      setTimeout(() => {
        if (scannerError === err.toString()) {
          scannerError = "";
        }
      }, 5000);
    }
  }
</script>

{#if scannerError}
  <div class="scanner-error-toast">
    {scannerError}
  </div>
{/if}

<div class="sell-workspace">
  <div class="main-sale-area">
    <!-- Hidden scanner wedge to capture global scans -->
    <div style="position: absolute; opacity: 0; pointer-events: none; width: 1px; height: 1px; overflow: hidden;">
      <BarcodeScanner onScan={handleScan} />
    </div>

    <!-- Active Cart Panel -->
    <div class="cart-section card">
      <div class="cart-total-header">
        {#if selectedCustomer}
          <span class="patient-name-total">
            {selectedCustomer.name}
          </span>
        {/if}
        <div class="cart-total-top">
          {cartTotal.toFixed(2)} {t("da")}
        </div>
        {#if cart.length > 0}
          <button onclick={handleValidateClick} class="btn-validate-cart" title={t("checkout")}>
            <img src={checkIcon} alt="Validate" class="check-icon-img" />
          </button>
        {/if}
      </div>

      <!-- Drug Search / Scanner Bar -->
      <div class="customer-search-box mt-1">
        <input 
          type="text" 
          class="input-pos w-100" 
          placeholder="{t('searchDrug')} ({t('designation')} / {t('barcode')})"
          bind:value={cartStore.drugSearchQuery}
          oninput={handleDrugSearchInput}
        />
        {#if showDrugSuggestions && drugSuggestions.length > 0}
          <ul class="suggestions-list">
            {#each drugSuggestions as sugg}
              <li>
                <button class="patient-name-btn" onclick={() => selectDrugFromSearch(sugg)}>
                  <strong>{sugg.name}</strong> - {sugg.price_per_item_da.toFixed(2)} DA <span class="patient-birth-tag">({sugg.total_stock_pcs} en stock)</span>
                </button>
              </li>
            {/each}
          </ul>
        {/if}
      </div>

      <div class="table-scroll-container">
        <table class="pos-table mt-1">
          <thead>
            <tr>
              <th>{t("designation")}</th>
              <th>{t("ppa")}</th>
              <th>{t("quantity")}</th>
              <th>{t("total")}</th>
              <th>Action</th>
            </tr>
          </thead>
          <tbody>
            {#each cart as item}
              <tr>
                <td>
                  <div class="drug-name-cell">
                    <span>{item.drug.name}</span>
                    {#if item.drug.requires_prescription}
                      <span class="presc-badge">Rx</span>
                    {/if}
                  </div>
                </td>
                <td>{item.drug.price_per_item_da} {t("da")}</td>
                <td>
                  <div class="qty-adjuster">
                    <button onclick={() => updateQuantity(item.drug.id, -1)} class="qty-btn">-</button>
                    <span class="qty-val">{item.quantity}</span>
                    <button onclick={() => updateQuantity(item.drug.id, 1)} class="qty-btn">+</button>
                  </div>
                </td>
                <td class="bold">{(item.drug.price_per_item_da * item.quantity).toFixed(2)} {t("da")}</td>
                <td>
                  <button onclick={() => removeFromCart(item.drug.id)} class="btn-remove" aria-label={t("remove")} title={t("remove")}>
                    <img src={trashIcon} alt="Delete" class="trash-icon-img" />
                  </button>
                </td>
              </tr>
            {/each}
            {#each Array(Math.max(0, 10 - cart.length)) as _, i}
              <tr class="placeholder-row">
                <td>&nbsp;</td>
                <td>&nbsp;</td>
                <td>&nbsp;</td>
                <td>&nbsp;</td>
                <td>
                  {#if i === 0}
                    <button onclick={handleManualAdd} class="btn-add-manual" aria-label="Ajouter médicament" title="Ajouter manuellement">
                      <img src={plusIcon} alt="Add" class="plus-icon-img" />
                    </button>
                  {:else}
                    &nbsp;
                  {/if}
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    </div>

    <!-- Checkout section with Prescription details -->
    {#if cart.length > 0 && requiresPrescription}
      <div class="checkout-details card mt-1">
        {#if requiresPrescription}
          <div class="prescription-info-banner">
            Contient des médicaments sous ordonnance. Veuillez renseigner :
          </div>
          <div class="prescription-inputs mt-1">
            <div class="form-group">
              <label for="doctor">{t("doctorName")} *</label>
              <input
                id="doctor"
                type="text"
                class="input-pos"
                bind:value={cartStore.doctorName}
                placeholder="Dr. ..."
                required
              />
            </div>
            <div class="form-group">
              <label for="patient">{t("patientName")} *</label>
              <input
                id="patient"
                type="text"
                class="input-pos"
                bind:value={cartStore.patientName}
                placeholder="Nom du patient..."
                required
              />
            </div>
          </div>
        {/if}
      </div>
    {/if}
  </div>

  <!-- Customer search & reference history side panel -->
  <aside class="customer-sidebar card">
    <div class="customer-header">
      <h3>Patient</h3>
      <button onclick={() => showAddCustomerModal = true} class="btn-add-customer" title={t("addCustomer")}>
        <img src={plusIcon} alt="Add Client" class="plus-icon-img" />
      </button>
    </div>
    
    <div class="customer-search-box mt-1">
      <input
        type="text"
        class="input-pos"
        placeholder={t("searchPatient")}
        bind:value={cartStore.customerInput}
        oninput={handleCustomerInput}
        disabled={selectedCustomer !== null}
      />
      {#if selectedCustomer}
        <button onclick={clearCustomer} class="clear-cust-btn" title="Désélectionner">
          <img src={closepIcon} alt="Unselect" class="clear-cust-icon-img" />
        </button>
      {/if}

      {#if showSuggestions && customerSuggestions.length > 0}
        <ul class="suggestions-list">
          {#each customerSuggestions as sugg}
            <li>
              <button class="patient-name-btn" onclick={() => selectCustomer(sugg)}>
                {sugg.name} <span class="patient-birth-tag">({formatDateFr(sugg.birth_date)})</span>
              </button>
              <div class="patient-actions-wrapper">
                <button class="patient-action-btn" onclick={(e) => { e.stopPropagation(); viewPatientDetails(sugg); }} title="Voir détails">
                  <img src={viewIcon} alt="View" class="patient-action-icon" />
                </button>
                <button class="patient-action-btn" onclick={(e) => { e.stopPropagation(); deletePatient(sugg.id); }} title="Supprimer">
                  <img src={closeIcon} alt="Delete" class="patient-action-icon" />
                </button>
              </div>
            </li>
          {/each}
        </ul>
      {/if}
    </div>

    <!-- Last Invoice panel -->
    {#if lastInvoice}
      <div class="last-invoice-panel mt-2">
        <div class="last-invoice-header">
          <div style="display: flex; align-items: center; gap: 0.5rem; flex-wrap: wrap;">
            <h4>{lastInvoice.is_loan ? t("lastInvoiceLoan") : t("lastInvoice")}</h4>
            {#if lastInvoice.is_loan}
              <span class="badge-loan {lastInvoice.loan_status === 'settled' ? 'status-ok' : 'badge-warning'}">
                {lastInvoice.loan_status === 'settled' ? t("loanBadgeSettled") : t("filterActiveLoans")}
              </span>
            {/if}
          </div>
          <button 
            type="button" 
            class="patient-action-btn" 
            onclick={() => completedInvoice = lastInvoice} 
            title={t("viewDetails")}
          >
            <img src={viewIcon} alt="View" class="patient-action-icon" />
          </button>
        </div>
        <div class="invoice-box mt-1">
          {#if lastInvoice.is_loan}
            <p>
              <strong>{t("status")}:</strong>
              <span class="badge-loan {lastInvoice.loan_status === 'settled' ? 'status-ok' : 'badge-warning'}">
                {t("loanBadge")} - {lastInvoice.loan_status === 'settled' ? t("loanBadgeSettled") : t("filterActiveLoans")}
              </span>
              {#if lastInvoice.loan_status === 'settled' && lastInvoice.loan_settled_at}
                <span class="settled-date-text">({t("settledAt")} {formatDateFr(lastInvoice.loan_settled_at)})</span>
              {/if}
            </p>
          {/if}
          <p><strong>N°:</strong> #{lastInvoice.id}</p>
          <p><strong>Date:</strong> {formatDateFr(lastInvoice.created_at)}</p>
          {#if lastInvoice.treatment_period_days}
            <p><strong>{t("treatmentPeriodDays")}:</strong> {lastInvoice.treatment_period_days} {t("daysUnit")}</p>
            {#if lastInvoiceTreatmentStatus}
              <p>
                <strong>{t("periodStatus")}:</strong>
                {#if lastInvoiceTreatmentStatus.isPassed}
                  <span class="status-period-passed">{t("periodPassed")}</span>
                {:else}
                  <span class="status-days-remaining">{t("daysRemaining", { days: lastInvoiceTreatmentStatus.daysLeft })}</span>
                {/if}
              </p>
            {/if}
          {/if}
          <p><strong>{t("total")}:</strong> {lastInvoice.total_da.toFixed(2)} {t("da")}</p>
          
          <h5 class="mt-1">Articles :</h5>
          <ul class="invoice-items-mini">
            {#each lastInvoice.items as item}
              <li>{item.drug_name} x{item.quantity_items}</li>
            {/each}
          </ul>
        </div>
      </div>
    {/if}
  </aside>
</div>

<!-- Add Customer Modal -->
{#if showAddCustomerModal}
  <div class="modal-overlay">
    <div class="modal-content client-modal">
      <h3>{t("addCustomer")}</h3>
      <form onsubmit={handleAddCustomerSubmit} class="client-form mt-1">
        {#if addCustomerError}
          <div class="error-banner">{addCustomerError}</div>
        {/if}
        
        <div class="form-group">
          <label for="cust-name">{t("name")} *</label>
          <input
            id="cust-name"
            type="text"
            class="input-pos"
            bind:value={newCustomerName}
            required
            placeholder="Nom et Prénom"
          />
        </div>
        
        <div class="form-group mt-1">
          <label for="cust-birth">{t("birthDate")} *</label>
          <input
            id="cust-birth"
            type="date"
            class="input-pos"
            bind:value={newCustomerBirthDate}
            required
          />
        </div>

        <div class="form-actions mt-2">
          <button type="button" class="btn-action btn-action-danger" onclick={() => { showAddCustomerModal = false; addCustomerError = ""; }}>
            {t("cancel")}
          </button>
          <button type="submit" class="btn-action btn-action-primary">
            {t("save")}
          </button>
        </div>
      </form>
    </div>
  </div>
{/if}

<!-- View Patient Details Modal -->
{#if showViewPatientModal && selectedPatientForView}
  <div class="modal-overlay">
    <div class="modal-content client-modal">
      <h3>Détails du Patient</h3>
      <div class="client-form mt-1" style="font-size: 1.15rem; gap: 0.75rem;">
        <p><strong>{t("name")}:</strong> {selectedPatientForView.name}</p>
        <p><strong>{t("birthDate")}:</strong> {formatDateFr(selectedPatientForView.birth_date)}</p>
      </div>
      <div class="form-actions mt-2">
        <button class="btn-action btn-action-primary w-100" onclick={() => showViewPatientModal = false}>
          Fermer
        </button>
      </div>
    </div>
  </div>
{/if}

<!-- Active Loan Deduction Modal -->
{#if showLoanDeductionModal && detectedLoanMatches.length > 0}
  <div class="modal-overlay">
    <div class="modal-content client-modal">
      <h3>{t("loanDetectedTitle")}</h3>

      {#if selectedCustomer}
        <ul class="patient-bullet-points mt-1">
          <li><strong>{t("patientName")}:</strong> {selectedCustomer.name}</li>
          {#if selectedCustomer.birth_date}
            <li><strong>{t("birthDate")}:</strong> {formatDateFr(selectedCustomer.birth_date)}</li>
          {/if}
        </ul>
      {/if}

      <div class="loan-matches-list mt-1">
        {#each detectedLoanMatches as match}
          <div class="loan-match-item">
            <div class="loan-match-header">
              <span class="drug-name-bold">{match.cartItem.drug.name}</span>
              <span class="loan-badge-tag">{t("loanRefShort", { id: match.loan.sale_id })}</span>
            </div>

            <div class="loan-grid">
              <div class="loan-grid-cell">
                <span class="cell-label">{t("qtyRequested")}</span>
                <span class="cell-val">{match.cartItem.quantity}</span>
              </div>
              <div class="loan-grid-cell">
                <span class="cell-label">{t("qtyInLoan")}</span>
                <span class="cell-val">{match.deductQty}</span>
              </div>
              <div class="loan-grid-cell cell-deliver">
                <span class="cell-label">{t("qtyToDeliver")}</span>
                <span class="cell-val">{match.deliverQty}</span>
              </div>
            </div>

            <div class="loan-match-footer">
              {t("chargeSummary", { qty: match.cartItem.quantity })}
            </div>
          </div>
        {/each}
      </div>

      <div class="form-actions mt-2" style="display: flex; flex-direction: column; gap: 0.75rem;">
        <button
          type="button"
          class="btn-action btn-action-primary w-100"
          onclick={confirmLoanDeductionAction}
        >
          {t("deductLoanBtn")}
        </button>

        <div style="display: flex; gap: 0.75rem; width: 100%;">
          <button
            type="button"
            class="btn-action btn-action-secondary"
            style="flex: 1;"
            onclick={declineLoanDeductionAction}
          >
            {t("keepFullQtyBtn")}
          </button>
          <button
            type="button"
            class="btn-action btn-action-danger"
            style="flex: 1;"
            onclick={cancelLoanDeductionAction}
          >
            {t("cancel")}
          </button>
        </div>
      </div>
    </div>
  </div>
{/if}

<!-- Loan Confirmation Modal (When previous treatment period is not finished yet) -->
{#if showLoanConfirmModal}
  <div class="modal-overlay">
    <div class="modal-content client-modal">
      <h3>{t("loanConfirmTitle")}</h3>

      {#if selectedCustomer}
        <ul class="patient-bullet-points mt-1">
          <li><strong>{t("patientName")}:</strong> {selectedCustomer.name}</li>
          {#if selectedCustomer.birth_date}
            <li><strong>{t("birthDate")}:</strong> {formatDateFr(selectedCustomer.birth_date)}</li>
          {/if}
        </ul>
      {/if}

      <div class="alert-period-red mt-1">
        {t("periodNotDoneAlert", { days: lastInvoiceTreatmentStatus?.daysLeft ?? 0 })}
      </div>

      <div class="form-actions mt-2" style="display: flex; flex-direction: column; gap: 0.75rem;">
        <button
          type="button"
          class="btn-action btn-action-primary w-100"
          onclick={confirmAsLoanAction}
        >
          {t("confirmAsLoan")}
        </button>

        <div style="display: flex; gap: 0.75rem; width: 100%;">
          <button
            type="button"
            class="btn-action btn-action-secondary"
            style="flex: 1;"
            onclick={proceedNormalSaleAction}
          >
            {t("proceedNormalSale")}
          </button>
          <button
            type="button"
            class="btn-action btn-action-danger"
            style="flex: 1;"
            onclick={() => showLoanConfirmModal = false}
          >
            {t("cancel")}
          </button>
        </div>
      </div>
    </div>
  </div>
{/if}

<!-- Treatment Period Modal (Asked upon validating cart for patient) -->
{#if showTreatmentPeriodModal}
  <div class="modal-overlay">
    <div class="modal-content client-modal">
      <h3>{t("treatmentPeriodTitle")}</h3>

      {#if selectedCustomer}
        <div class="mt-1" style="font-weight: 600; font-size: 1.15rem; color: var(--color-primary);">
          Patient : {selectedCustomer.name} ({formatDateFr(selectedCustomer.birth_date)})
        </div>
      {/if}

      <form onsubmit={handleConfirmTreatmentPeriod} class="client-form mt-1">
        {#if treatmentDaysError}
          <div class="error-banner">{treatmentDaysError}</div>
        {/if}

        <div class="form-group mt-1">
          <input
            id="treatment-days"
            type="number"
            min="1"
            max="365"
            class="input-pos"
            bind:value={cartStore.treatmentDaysInput}
            required
            placeholder={t("treatmentDaysLabel")}
          />
        </div>

        <div class="form-actions mt-2">
          <button
            type="button"
            class="btn-action btn-action-danger"
            onclick={() => { showTreatmentPeriodModal = false; treatmentDaysError = ""; }}
          >
            {t("cancel")}
          </button>
          <button type="submit" class="btn-action btn-action-primary">
            {t("checkout")}
          </button>
        </div>
      </form>
    </div>
  </div>
{/if}

<!-- Digital Invoice Mockup Modal (On-screen receipt modal) -->
{#if completedInvoice}
  <InvoiceModal
    invoice={completedInvoice}
    onClose={() => completedInvoice = null}
    title="TICKET DE CAISSE"
  />
{/if}

<style>
  .sell-workspace {
    display: grid;
    grid-template-columns: 3fr 1fr;
    gap: 1.5rem;
    width: 100%;
    height: 100%;
    box-sizing: border-box;
  }

  .main-sale-area {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    height: 100%;
    overflow: hidden;
  }

  .card {
    background-color: var(--color-bg-card);
    border: var(--border-width) solid var(--color-border);
    border-radius: var(--border-radius);
    padding: 1.5rem;
    box-shadow: 0 4px 6px var(--color-shadow);
  }

  .cart-section {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .table-scroll-container {
    flex: 1;
    overflow-y: auto;
    width: 100%;
  }

  .customer-sidebar {
    height: 100%;
    display: flex;
    flex-direction: column;
    overflow-y: auto;
  }

  .drug-name-cell {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .presc-badge {
    background-color: var(--color-danger);
    color: var(--color-text-light);
    font-size: 0.8rem;
    font-weight: bold;
    padding: 0.1rem 0.4rem;
    border-radius: 4px;
  }

  .qty-adjuster {
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }

  .qty-btn {
    width: 32px;
    height: 32px;
    border: 2px solid var(--color-border);
    border-radius: 6px;
    background-color: var(--color-bg-app);
    font-weight: bold;
    font-size: 1.2rem;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .qty-btn:active {
    background-color: #e2e6ea;
  }

  .qty-val {
    font-weight: bold;
    font-size: 1.15rem;
  }

  .btn-remove {
    background-color: transparent;
    border: none;
    padding: 0.25rem;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: transform 0.15s ease;
  }

  .btn-remove:hover {
    background-color: transparent;
    transform: scale(1.1);
  }

  .trash-icon-img {
    width: 32px;
    height: 32px;
    object-fit: contain;
  }

  .btn-add-manual {
    background-color: transparent;
    border: none;
    padding: 0;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: transform 0.15s ease;
  }

  .btn-add-manual:hover {
    background-color: transparent;
    transform: scale(1.15);
  }

  .plus-icon-img {
    width: 32px;
    height: 32px;
    object-fit: contain;
  }

  .scanner-error-toast {
    position: fixed;
    top: 2rem;
    right: 2rem;
    background-color: var(--color-danger);
    color: var(--color-text-light);
    padding: 1rem 2rem;
    border-radius: var(--border-radius);
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
    z-index: 9999;
    font-weight: bold;
    animation: slideIn 0.3s ease;
  }

  @keyframes slideIn {
    from {
      transform: translateX(100%);
      opacity: 0;
    }
    to {
      transform: translateX(0);
      opacity: 1;
    }
  }

  .placeholder-row td {
    height: 53px;
    color: transparent;
  }

  .cart-total-header {
    display: flex;
    justify-content: center;
    align-items: center;
    position: relative;
    margin-bottom: 1.5rem;
  }

  .cart-total-top {
    font-size: 3rem;
    font-weight: 900;
    color: var(--color-primary);
    font-family: monospace;
    letter-spacing: 0.05em;
    text-align: center;
  }

  .btn-validate-cart {
    position: absolute;
    background-color: transparent;
    border: none;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: transform 0.15s ease;
    padding: 0;
  }

  :global([dir="ltr"]) .btn-validate-cart {
    right: 1.5rem;
    left: auto;
  }

  :global([dir="rtl"]) .btn-validate-cart {
    left: 1.5rem;
    right: auto;
  }

  .btn-validate-cart:hover {
    transform: scale(1.15);
  }

  .check-icon-img {
    width: 48px;
    height: 48px;
    object-fit: contain;
  }

  /* Prescription Section */
  .prescription-info-banner {
    background-color: #fff3cd;
    border-left: 5px solid var(--color-warning);
    color: #856404;
    padding: 0.75rem;
    border-radius: 6px;
    font-weight: bold;
  }

  :global([dir="rtl"]) .prescription-info-banner {
    border-left: none;
    border-right: 5px solid var(--color-warning);
  }

  .prescription-inputs {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1rem;
  }

  .form-group {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }

  .form-group label {
    font-weight: bold;
  }

  /* Customer sidebar */
  .customer-search-box {
    position: relative;
    display: flex;
    align-items: center;
  }

  .clear-cust-btn {
    position: absolute;
    right: 1rem;
    background: none;
    border: none;
    font-weight: bold;
    cursor: pointer;
    color: var(--color-danger);
  }

  :global([dir="rtl"]) .clear-cust-btn {
    left: 1rem;
    right: auto;
  }

  .suggestions-list {
    position: absolute;
    top: 100%;
    left: 0;
    right: 0;
    background-color: var(--color-bg-card);
    border: 2px solid var(--color-border);
    border-radius: 8px;
    margin-top: 4px;
    max-height: 200px;
    overflow-y: auto;
    z-index: 50;
    box-shadow: 0 4px 10px rgba(0,0,0,0.15);
    list-style: none;
  }

  .suggestions-list button {
    width: 100%;
    padding: 0.75rem 1rem;
    text-align: left;
    border: none;
    background: none;
    cursor: pointer;
    font-size: 1.1rem;
    font-weight: 600;
  }

  :global([dir="rtl"]) .suggestions-list button {
    text-align: right;
  }

  .suggestions-list button:hover {
    background-color: var(--color-bg-app);
    color: var(--color-primary);
  }

  /* Last Invoice Panel */
  .last-invoice-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    width: 100%;
  }

  .badge-loan {
    padding: 0.15rem 0.5rem;
    border-radius: 4px;
    font-weight: bold;
    display: inline-block;
    font-size: 0.85rem;
  }

  .status-ok {
    background-color: var(--color-primary);
    color: #ffffff;
  }

  .badge-warning {
    background-color: var(--color-warning);
    color: var(--color-text-dark);
  }

  .settled-date-text {
    font-size: 0.85rem;
    color: var(--color-text-secondary);
    margin-left: 0.35rem;
  }

  :global([dir="rtl"]) .settled-date-text {
    margin-left: 0;
    margin-right: 0.35rem;
  }

  .invoice-box {
    background-color: var(--color-bg-app);
    border: 2px solid var(--color-border);
    padding: 1rem;
    border-radius: 8px;
    font-size: 1rem;
  }

  .invoice-box p {
    margin-bottom: 0.4rem;
  }

  .invoice-items-mini {
    list-style: square;
    padding-left: 1.2rem;
    font-size: 0.95rem;
    margin-top: 0.25rem;
  }

  /* Table styling */
  .pos-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 1.1rem;
  }

  .pos-table th, .pos-table td {
    padding: 0.75rem 1rem;
    border-bottom: 1px solid var(--color-border);
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



  .mt-1 {
    margin-top: 1rem;
  }

  .mt-2 {
    margin-top: 2rem;
  }



  .customer-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    width: 100%;
    margin-bottom: 0.5rem;
  }

  .btn-add-customer {
    background-color: transparent;
    border: none;
    padding: 0;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: transform 0.15s ease;
  }

  .btn-add-customer:hover {
    transform: scale(1.15);
  }

  .client-modal {
    max-width: 450px;
    padding: 1.5rem;
  }

  .client-form {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  /* Patient suggestions styling */
  .suggestions-list {
    position: absolute;
    top: 100%;
    left: 0;
    right: 0;
    background-color: var(--color-bg-card);
    border: 2px solid var(--color-border);
    border-radius: var(--border-radius);
    z-index: 1000;
    max-height: 250px;
    overflow-y: auto;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .suggestions-list li {
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-bottom: 1px solid var(--color-bg-app);
    padding: 0.5rem 0.75rem;
  }

  .suggestions-list li:hover {
    background-color: var(--color-bg-app);
  }

  .patient-name-btn {
    flex: 1;
    background: transparent;
    border: none;
    text-align: left;
    font-size: 1.1rem;
    font-weight: 600;
    color: var(--color-text-dark);
    cursor: pointer;
    padding: 0.25rem 0;
  }

  :global([dir="rtl"]) .patient-name-btn {
    text-align: right;
  }

  .patient-actions-wrapper {
    display: flex;
    gap: 0.15rem;
    align-items: center;
  }

  .patient-action-btn {
    background: transparent;
    border: none;
    padding: 0.1rem;
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

  .clear-cust-btn {
    position: absolute;
    background: transparent;
    border: none;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0;
    transition: transform 0.1s ease;
  }

  :global([dir="ltr"]) .clear-cust-btn {
    right: 0.75rem;
    left: auto;
  }

  :global([dir="rtl"]) .clear-cust-btn {
    left: 0.75rem;
    right: auto;
  }

  .clear-cust-btn:hover {
    transform: scale(1.15);
  }

  .clear-cust-icon-img {
    width: 24px;
    height: 24px;
    object-fit: contain;
  }

  .customer-search-box input:disabled {
    background-color: #e9ecef !important;
    color: #212529 !important;
    border-color: #ced4da;
    cursor: not-allowed;
  }

  .patient-name-total {
    position: absolute;
    font-size: 1.8rem;
    font-weight: 850;
    color: #000;
    background: none;
    border: none;
    padding: 0;
  }

  :global([dir="ltr"]) .patient-name-total {
    left: 1.5rem;
    right: auto;
  }

  :global([dir="rtl"]) .patient-name-total {
    right: 1.5rem;
    left: auto;
  }

  .error-banner {
    background-color: rgba(220, 53, 69, 0.1);
    color: #dc3545;
    border: 1px solid rgba(220, 53, 69, 0.25);
    padding: 0.75rem 1rem;
    border-radius: var(--border-radius);
    font-weight: 600;
    margin-bottom: 1rem;
    text-align: center;
  }

  .patient-birth-tag {
    font-size: 0.82rem;
    color: var(--color-text-secondary);
    font-weight: normal;
    margin-left: 0.4rem;
  }

  :global([dir="rtl"]) .patient-birth-tag {
    margin-left: 0;
    margin-right: 0.4rem;
  }

  .status-days-remaining {
    color: #dc2626;
    font-weight: 750;
  }

  .status-period-passed {
    color: #16a34a;
    font-weight: 750;
  }

  .patient-bullet-points {
    color: #000000;
    font-size: 1.15rem;
    margin: 0.75rem 0 0.75rem 1.25rem;
    padding: 0;
    list-style-type: disc;
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }

  :global([dir="rtl"]) .patient-bullet-points {
    margin: 0.75rem 1.25rem 0.75rem 0;
  }

  .patient-bullet-points li {
    color: #000000;
  }

  .alert-period-red {
    color: #dc2626;
    font-size: 1.1rem;
    font-weight: 700;
    text-align: center;
    margin: 0.75rem 0 1rem 0;
  }

  .loan-matches-list {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    max-height: 280px;
    overflow-y: auto;
  }

  .loan-match-item {
    background-color: var(--color-bg-app);
    border: 1px solid var(--color-border);
    padding: 0.85rem 1rem;
    border-radius: var(--border-radius);
  }

  .loan-match-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 0.75rem;
  }

  .drug-name-bold {
    font-weight: 750;
    font-size: 1.05rem;
    color: var(--color-text-dark);
    word-break: break-word;
  }

  .loan-badge-tag {
    font-size: 0.85rem;
    font-weight: 600;
    color: var(--color-text-secondary);
    background-color: var(--color-bg-card);
    border: 1px solid var(--color-border);
    padding: 0.2rem 0.55rem;
    border-radius: 4px;
    white-space: nowrap;
  }

  .loan-grid {
    display: grid;
    grid-template-columns: 1fr 1fr 1fr;
    gap: 0.5rem;
    margin-bottom: 0.75rem;
  }

  .loan-grid-cell {
    background-color: var(--color-bg-card);
    border: 1px solid var(--color-border);
    border-radius: var(--border-radius);
    padding: 0.6rem 0.25rem;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
  }

  .loan-grid-cell .cell-label {
    font-size: 0.8rem;
    font-weight: 600;
    color: var(--color-text-secondary);
  }

  .loan-grid-cell .cell-val {
    font-size: 1.5rem;
    font-weight: 800;
    color: var(--color-text-dark);
    line-height: 1.2;
    margin-top: 0.2rem;
  }

  .loan-grid-cell.cell-deliver .cell-label {
    color: #dc2626;
    font-weight: 700;
  }

  .loan-grid-cell.cell-deliver .cell-val {
    color: #dc2626;
  }

  .loan-match-footer {
    font-size: 0.9rem;
    color: var(--color-text-secondary);
    text-align: center;
    font-weight: 600;
    padding-top: 0.5rem;
    border-top: 1px dashed var(--color-border);
  }
</style>
