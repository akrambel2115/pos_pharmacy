<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import { t } from "$lib/i18n/index.svelte";
  import { formatDateFr } from "$lib/utils";
  import { openPath } from "@tauri-apps/plugin-opener";
  import JsBarcode from "jsbarcode";

  // Icons (matching StockWorkspace)
  import pdfIcon from "../../public/icons/pdf.png";
  import viewIcon from "../../public/icons/view.png";
  import uploadIcon from "../../public/icons/upload.png";
  import printerIcon from "../../public/icons/printer.png";
  import trashIcon from "../../public/icons/trash.png";
  import closeIcon from "../../public/icons/close.png";
  import checkIcon from "../../public/icons/check.png";

  // Modal
  import InvoiceImportModal from "./InvoiceImportModal.svelte";

  let { userRole = "cashier" } = $props<{ userRole?: "cashier" | "admin" | null }>();
  let isAdmin = $derived(userRole === "admin");

  // Invoices list state
  interface ImportedInvoiceItem {
    id: number;
    supplier_name: string;
    invoice_number: string;
    invoice_date: string;
    total_amount_da: number;
    pdf_path: string;
    created_at: string;
    items_count: number;
    total_packages: number;
  }

  interface InvoiceDrugDetail {
    drug_id: number;
    name: string;
    barcode: string;
    price_per_item_da: number;
    packages_received: number;
    batch_number: string;
    expiry_date: string;
  }

  let invoices = $state<ImportedInvoiceItem[]>([]);
  let isLoading = $state(false);
  let errorMsg = $state("");
  let successMsg = $state("");

  // Search & Filter State
  let searchQuery = $state("");
  let selectedSupplier = $state("all");
  let dateFilter = $state<"all" | "today" | "week" | "month">("all");

  // Sorting State (matching StockWorkspace)
  let sortColumn = $state<"number" | "supplier" | "date" | "items" | "packages" | "total" | "created">("created");
  let sortDirection = $state<"asc" | "desc">("desc");

  // Pagination State (matching StockWorkspace: pageSize = 10)
  let currentPage = $state(1);
  const pageSize = 10;

  // Details Modal State
  let selectedInvoice = $state<ImportedInvoiceItem | null>(null);
  let invoiceDrugs = $state<InvoiceDrugDetail[]>([]);
  let isLoadingDrugs = $state(false);
  let showImportModal = $state(false);

  // Print Barcode State
  let isPrintMode = $state(false);
  let selectedInvoiceIds = $state<Set<number>>(new Set());
  let showPrintChoiceModal = $state(false);
  let showPrintBarcodeModal = $state(false);
  let printDrugsList = $state<any[]>([]);
  let labelCopiesMap = $state<Record<string, number>>({});
  let printModalTitle = $state("Impression des codes-barres");
  let isBulkLoadingDrugs = $state(false);

  // Delete Confirmation State
  let invoiceToDelete = $state<ImportedInvoiceItem | null>(null);
  let deleteWithStock = $state<boolean | null>(null);
  let isDeleting = $state(false);

  async function loadInvoices() {
    isLoading = true;
    errorMsg = "";
    try {
      invoices = await invoke<ImportedInvoiceItem[]>("get_imported_invoices");
    } catch (err: any) {
      errorMsg = err.toString();
    } finally {
      isLoading = false;
    }
  }

  onMount(() => {
    loadInvoices();
  });

  // Unique suppliers list for filter dropdown
  let supplierOptions = $derived.by(() => {
    const set = new Set<string>();
    for (const inv of invoices) {
      if (inv.supplier_name && inv.supplier_name.trim()) {
        set.add(inv.supplier_name.trim());
      }
    }
    return Array.from(set).sort((a, b) => a.localeCompare(b));
  });

  // Filter & Sort
  let filteredInvoices = $derived.by(() => {
    const q = searchQuery.toLowerCase().trim();
    const now = new Date();
    const todayStr = now.toISOString().slice(0, 10);

    return invoices.filter((inv) => {
      if (q) {
        const supMatch = inv.supplier_name?.toLowerCase().includes(q) ?? false;
        const numMatch = inv.invoice_number?.toLowerCase().includes(q) ?? false;
        const dateMatch = (inv.invoice_date && inv.invoice_date.toLowerCase().includes(q)) || inv.created_at.includes(q);
        const amountMatch = inv.total_amount_da.toString().includes(q);
        const idMatch = inv.id.toString().includes(q);
        if (!supMatch && !numMatch && !dateMatch && !amountMatch && !idMatch) {
          return false;
        }
      }

      if (selectedSupplier !== "all") {
        if (inv.supplier_name.trim() !== selectedSupplier) {
          return false;
        }
      }

      const invDateStr = inv.invoice_date && inv.invoice_date.length >= 10
        ? inv.invoice_date.slice(0, 10)
        : inv.created_at.slice(0, 10);

      if (dateFilter === "today") {
        if (invDateStr !== todayStr) return false;
      } else if (dateFilter === "week") {
        const invDate = new Date(invDateStr);
        const weekAgo = new Date();
        weekAgo.setDate(weekAgo.getDate() - 7);
        if (isNaN(invDate.getTime()) || invDate < weekAgo) return false;
      } else if (dateFilter === "month") {
        const invDate = new Date(invDateStr);
        const monthAgo = new Date();
        monthAgo.setDate(monthAgo.getDate() - 30);
        if (isNaN(invDate.getTime()) || invDate < monthAgo) return false;
      }

      return true;
    });
  });

  let sortedInvoices = $derived.by(() => {
    return [...filteredInvoices].sort((a, b) => {
      let comparison = 0;
      switch (sortColumn) {
        case "number":
          comparison = (a.invoice_number || "").localeCompare(b.invoice_number || "");
          break;
        case "supplier":
          comparison = a.supplier_name.localeCompare(b.supplier_name);
          break;
        case "date":
          comparison = (a.invoice_date || "").localeCompare(b.invoice_date || "");
          break;
        case "items":
          comparison = (a.items_count || 0) - (b.items_count || 0);
          break;
        case "packages":
          comparison = (a.total_packages || 0) - (b.total_packages || 0);
          break;
        case "total":
          comparison = a.total_amount_da - b.total_amount_da;
          break;
        case "created":
        default:
          comparison = a.created_at.localeCompare(b.created_at);
          break;
      }
      return sortDirection === "asc" ? comparison : -comparison;
    });
  });

  // Pagination
  let totalPages = $derived(Math.ceil(sortedInvoices.length / pageSize) || 1);
  let paginatedInvoices = $derived(
    sortedInvoices.slice((currentPage - 1) * pageSize, currentPage * pageSize)
  );

  function toggleSort(column: typeof sortColumn) {
    if (sortColumn === column) {
      sortDirection = sortDirection === "asc" ? "desc" : "asc";
    } else {
      sortColumn = column;
      sortDirection = "asc";
    }
    currentPage = 1;
  }

  function getSortIndicator(column: typeof sortColumn): string {
    if (sortColumn !== column) return "";
    return sortDirection === "asc" ? " ▲" : " ▼";
  }

  // Open Details Modal
  async function openInvoiceDetails(invoice: ImportedInvoiceItem) {
    selectedInvoice = invoice;
    isLoadingDrugs = true;
    invoiceDrugs = [];
    try {
      invoiceDrugs = await invoke<InvoiceDrugDetail[]>("get_imported_invoice_drugs", {
        invoiceId: invoice.id,
      });
    } catch (err: any) {
      console.error("Error loading invoice drugs:", err);
    } finally {
      isLoadingDrugs = false;
    }
  }

  function closeInvoiceDetails() {
    selectedInvoice = null;
    invoiceDrugs = [];
  }

  // Print Handlers
  function openPrintChoice() {
    showPrintChoiceModal = true;
  }

  function handleStartSelection() {
    showPrintChoiceModal = false;
    isPrintMode = true;
    selectedInvoiceIds = new Set(filteredInvoices.map(inv => inv.id));
  }

  function toggleSelectInvoice(id: number) {
    const next = new Set(selectedInvoiceIds);
    if (next.has(id)) {
      next.delete(id);
    } else {
      next.add(id);
    }
    selectedInvoiceIds = next;
  }

  let allVisibleSelected = $derived.by(() => {
    if (filteredInvoices.length === 0) return false;
    return filteredInvoices.every(inv => selectedInvoiceIds.has(inv.id));
  });

  function toggleSelectAll() {
    if (allVisibleSelected) {
      selectedInvoiceIds = new Set();
    } else {
      selectedInvoiceIds = new Set(filteredInvoices.map(inv => inv.id));
    }
  }

  async function handlePrintSelectedInvoices() {
    if (selectedInvoiceIds.size === 0) {
      alert("Veuillez sélectionner au moins une facture.");
      return;
    }

    try {
      isBulkLoadingDrugs = true;
      const allItems: any[] = [];
      const selectedList = invoices.filter(inv => selectedInvoiceIds.has(inv.id));

      for (const inv of selectedList) {
        const items = await invoke<any[]>("get_imported_invoice_drugs", { invoiceId: inv.id });
        if (items && items.length > 0) {
          allItems.push(...items);
        }
      }

      isBulkLoadingDrugs = false;
      if (allItems.length === 0) {
        alert("Aucun médicament trouvé dans les factures sélectionnées.");
        return;
      }

      prepareBarcodePrint(allItems, `Factures sélectionnées (${selectedInvoiceIds.size})`);
    } catch (err: any) {
      isBulkLoadingDrugs = false;
      alert("Erreur: " + err.toString());
    }
  }

  async function handlePrintLastInvoice() {
    try {
      const items = await invoke<any[]>("get_last_imported_invoice_drugs");
      if (!items || items.length === 0) {
        alert("Aucun médicament trouvé dans la dernière facture.");
        return;
      }
      prepareBarcodePrint(items, "Dernière facture importée");
      showPrintChoiceModal = false;
    } catch (err: any) {
      alert("Erreur: " + err.toString());
    }
  }

  async function handlePrintInvoiceBarcodes(invoice: ImportedInvoiceItem) {
    try {
      const items = await invoke<any[]>("get_imported_invoice_drugs", { invoiceId: invoice.id });
      if (!items || items.length === 0) {
        alert("Aucun médicament trouvé pour cette facture.");
        return;
      }
      prepareBarcodePrint(items, `Facture ${invoice.invoice_number || '#' + invoice.id} (${invoice.supplier_name})`);
    } catch (err: any) {
      alert("Erreur: " + err.toString());
    }
  }

  function prepareBarcodePrint(items: any[], title: string) {
    printModalTitle = title;
    printDrugsList = items.map((item, idx) => ({
      id: item.drug_id,
      printKey: `${item.drug_id}_${idx}`,
      name: item.name,
      barcode: item.barcode,
      price_per_item_da: item.price_per_item_da,
      batch_number: item.batch_number,
      expiry_date: item.expiry_date,
      packages_received: item.packages_received,
    }));
    const copies: Record<string, number> = {};
    printDrugsList.forEach(item => {
      copies[item.printKey] = item.packages_received > 0 ? item.packages_received : 1;
    });
    labelCopiesMap = copies;
    showPrintBarcodeModal = true;
  }

  function triggerDirectPrint() {
    window.print();
  }

  function barcodeAction(node: SVGSVGElement, code: string) {
    function draw(c: string) {
      try {
        JsBarcode(node, String(c || "000000"), {
          format: "CODE128",
          width: 1.5,
          height: 38,
          displayValue: true,
          fontSize: 12,
          textMargin: 2,
          margin: 2,
        });
      } catch (e) {
        console.error("Barcode render error:", e);
      }
    }
    draw(code);
    return {
      update(newCode: string) {
        draw(newCode);
      }
    };
  }

  // Open PDF File
  async function handleOpenPdf(pdfPath: string) {
    if (!pdfPath || !pdfPath.trim()) {
      alert(t("pdfNotAvailable"));
      return;
    }
    try {
      await openPath(pdfPath);
    } catch (e) {
      try {
        await invoke("open_invoice_file", { path: pdfPath });
      } catch (err: any) {
        alert("Impossible d'ouvrir le fichier PDF : " + err.toString());
      }
    }
  }

  // Delete invoice
  async function confirmDelete() {
    if (!invoiceToDelete || deleteWithStock === null) return;
    isDeleting = true;
    try {
      await invoke("delete_imported_invoice", { 
        invoiceId: invoiceToDelete.id,
        deleteStock: deleteWithStock 
      });
      invoices = invoices.filter((i) => i.id !== invoiceToDelete?.id);
      const wasWithStock = deleteWithStock;
      invoiceToDelete = null;
      deleteWithStock = null;
      successMsg = wasWithStock
        ? "Facture et stock importé supprimés avec succès."
        : "Facture supprimée. Le stock a été conservé avec facture inconnue.";
      setTimeout(() => (successMsg = ""), 3500);
    } catch (err: any) {
      errorMsg = err.toString();
    } finally {
      isDeleting = false;
    }
  }
</script>

<div class="stock-workspace">
  {#if errorMsg}
    <div class="error-banner">{errorMsg}</div>
  {/if}
  {#if successMsg}
    <div class="success-banner">{successMsg}</div>
  {/if}

  <!-- Header Actions Row (matching StockWorkspace layout with Imprimer & Importer buttons) -->
  <div class="stock-header-actions">
    <div class="search-filter-row">
      <input
        type="text"
        class="input-pos search-input"
        placeholder={t("searchInvoicesPlaceholder")}
        bind:value={searchQuery}
        oninput={() => currentPage = 1}
      />

      <select
        class="select-pos supplier-select"
        bind:value={selectedSupplier}
        onchange={() => currentPage = 1}
      >
        <option value="all">{t("allSuppliers")}</option>
        {#each supplierOptions as sup}
          <option value={sup}>{sup}</option>
        {/each}
      </select>

      <select
        class="select-pos date-select"
        bind:value={dateFilter}
        onchange={() => currentPage = 1}
      >
        <option value="all">{t("filterAll")}</option>
        <option value="today">{t("filterToday")}</option>
        <option value="week">{t("filterWeek")}</option>
        <option value="month">{t("filterMonth")}</option>
      </select>
    </div>

    <div class="stock-header-right-actions">
      <!-- Imprimer Button -->
      <button 
        type="button" 
        class="btn-header-icon-action" 
        onclick={openPrintChoice}
        title="Imprimer"
      >
        <img src={printerIcon} alt="Imprimer" class="header-action-icon-img" />
        <span class="header-action-icon-label">Imprimer</span>
      </button>

      <!-- Importer Button -->
      <button 
        type="button"
        onclick={() => showImportModal = true} 
        class="btn-header-icon-action" 
        title={t("importInvoice")}
      >
        <img src={uploadIcon} alt="Importer" class="header-action-icon-img" />
        <span class="header-action-icon-label">{t("importInvoice")}</span>
      </button>
    </div>
  </div>

  <!-- Invoices Table (matching StockWorkspace pos-table) -->
  <div class="cart-section">
    {#if isPrintMode}
      <div class="print-selection-toolbar">
        <div class="print-selection-info">
          <strong>Mode sélection d'impression :</strong>
          <span>{selectedInvoiceIds.size} facture(s) cochée(s)</span>
        </div>
        <div class="print-selection-actions">
          <button
            type="button"
            class="btn-action btn-action-primary"
            onclick={handlePrintSelectedInvoices}
            disabled={isBulkLoadingDrugs || selectedInvoiceIds.size === 0}
          >
            {isBulkLoadingDrugs ? "Chargement..." : `Imprimer les étiquettes (${selectedInvoiceIds.size})`}
          </button>
          <button
            type="button"
            class="btn-action btn-action-secondary"
            onclick={() => { isPrintMode = false; selectedInvoiceIds = new Set(); }}
          >
            Quitter la sélection
          </button>
        </div>
      </div>
    {/if}

    <div class="table-scroll-container">
      <table class="pos-table">
        <thead>
          <tr>
            {#if isPrintMode}
              <th class="checkbox-th">
                <input 
                  type="checkbox" 
                  class="pos-checkbox" 
                  checked={allVisibleSelected} 
                  onchange={toggleSelectAll} 
                  title={allVisibleSelected ? "Tout décocher" : "Tout cocher"}
                />
              </th>
            {/if}
            <th onclick={() => toggleSort("number")} class="sortable-th">
              {t("invoiceNumber")}{getSortIndicator("number")}
            </th>
            <th onclick={() => toggleSort("supplier")} class="sortable-th">
              {t("supplier")}{getSortIndicator("supplier")}
            </th>
            <th onclick={() => toggleSort("date")} class="sortable-th">
              {t("invoiceDate")}{getSortIndicator("date")}
            </th>
            <th onclick={() => toggleSort("items")} class="sortable-th">
              {t("itemsCount")}{getSortIndicator("items")}
            </th>
            <th onclick={() => toggleSort("packages")} class="sortable-th">
              {t("packagesCount")}{getSortIndicator("packages")}
            </th>
            <th onclick={() => toggleSort("total")} class="sortable-th">
              {t("total")} (DA){getSortIndicator("total")}
            </th>
            <th onclick={() => toggleSort("created")} class="sortable-th">
              {t("importedOn")}{getSortIndicator("created")}
            </th>
            <th>{t("actions")}</th>
          </tr>
        </thead>
        <tbody>
          {#if sortedInvoices.length === 0}
            <tr>
              <td colspan={isPrintMode ? 9 : 8} class="empty-row-text">
                {t("noInvoicesFound")}
              </td>
            </tr>
          {:else}
            {#each paginatedInvoices as inv}
              <tr class={isPrintMode && selectedInvoiceIds.has(inv.id) ? "row-selected" : ""}>
                {#if isPrintMode}
                  <td class="checkbox-td" onclick={(e) => e.stopPropagation()}>
                    <input 
                      type="checkbox" 
                      class="pos-checkbox" 
                      checked={selectedInvoiceIds.has(inv.id)} 
                      onchange={() => toggleSelectInvoice(inv.id)}
                    />
                  </td>
                {/if}
                <td class="bold">
                  <div class="drug-title-wrapper">
                    <span class="drug-name-text">
                      {inv.invoice_number ? inv.invoice_number : `#${inv.id}`}
                    </span>
                  </div>
                </td>
                <td class="bold">{inv.supplier_name || "-"}</td>
                <td>{inv.invoice_date ? formatDateFr(inv.invoice_date) : "-"}</td>
                <td>{inv.items_count || 0}</td>
                <td class="bold">{inv.total_packages || 0}</td>
                <td class="bold">{inv.total_amount_da.toFixed(2)}</td>
                <td>{formatDateFr(inv.created_at)}</td>
                <td>
                  <div class="patient-actions-wrapper">
                    <!-- View details -->
                    <button 
                      onclick={() => openInvoiceDetails(inv)} 
                      class="patient-action-btn" 
                      title={t("viewDetails")}
                    >
                      <img src={viewIcon} alt="View" class="patient-action-icon" />
                    </button>

                    <!-- Print barcode labels for this invoice -->
                    <button 
                      onclick={() => handlePrintInvoiceBarcodes(inv)} 
                      class="patient-action-btn" 
                      title="Imprimer les codes-barres"
                    >
                      <img src={printerIcon} alt="Print" class="patient-action-icon" />
                    </button>

                    <!-- Open PDF -->
                    {#if inv.pdf_path}
                      <button 
                        onclick={() => handleOpenPdf(inv.pdf_path)} 
                        class="patient-action-btn" 
                        title={t("openPdfFile")}
                      >
                        <img src={pdfIcon} alt="PDF" class="patient-action-icon" />
                      </button>
                    {/if}

                    <!-- Delete invoice (admin only) -->
                    {#if isAdmin}
                      <button 
                        onclick={() => { invoiceToDelete = inv; deleteWithStock = null; }} 
                        class="patient-action-btn" 
                        title={t("deleteInvoice")}
                      >
                        <img src={trashIcon} alt="Delete" class="patient-action-icon" />
                      </button>
                    {/if}
                  </div>
                </td>
              </tr>
            {/each}
          {/if}

          <!-- Placeholder rows to maintain fixed height matching StockWorkspace -->
          {#each Array(Math.max(0, pageSize - paginatedInvoices.length)) as _}
            <tr class="placeholder-row">
              {#if isPrintMode}
                <td>&nbsp;</td>
              {/if}
              <td>&nbsp;</td>
              <td>&nbsp;</td>
              <td>&nbsp;</td>
              <td>&nbsp;</td>
              <td>&nbsp;</td>
              <td>&nbsp;</td>
              <td>&nbsp;</td>
              <td>&nbsp;</td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>

    <!-- Pagination controls (matching StockWorkspace) -->
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

<!-- PRINT CHOICE MODAL -->
{#if showPrintChoiceModal}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="modal-overlay" onclick={() => showPrintChoiceModal = false}>
    <div class="modal-content card" onclick={(e) => e.stopPropagation()} style="max-width: 520px; padding: 1.5rem;">
      <div class="modal-header">
        <div>
          <h3>Options d'impression</h3>
          <span class="modal-subtitle">Choisissez le type d'impression à effectuer</span>
        </div>
        <button class="btn-close-modal" onclick={() => showPrintChoiceModal = false}>&times;</button>
      </div>

      <div class="pos-choice-grid">
        <button type="button" class="btn-pos-choice" onclick={handlePrintLastInvoice}>
          <img src={uploadIcon} alt="Dernière facture" class="btn-pos-choice-icon" />
          <span class="btn-pos-choice-label">Dernière facture</span>
          <span class="btn-pos-choice-desc">Étiquettes des médicaments du dernier arrivage</span>
        </button>

        <button type="button" class="btn-pos-choice" onclick={handleStartSelection}>
          <img src={checkIcon} alt="Sélectionner des factures" class="btn-pos-choice-icon" />
          <span class="btn-pos-choice-label">Sélectionner des factures</span>
          <span class="btn-pos-choice-desc">Choisir par cases à cocher dans la liste</span>
        </button>
      </div>

      <div class="confirm-actions">
        <button type="button" class="btn-pos-action btn-cancel" onclick={() => showPrintChoiceModal = false}>
          {t("cancel")}
        </button>
      </div>
    </div>
  </div>
{/if}

<!-- PRINT BARCODE PREVIEW MODAL -->
{#if showPrintBarcodeModal}
  <div class="modal-overlay">
    <div class="modal-content card max-w-4xl" style="max-width: 900px; width: 90%; max-height: 85vh; display: flex; flex-direction: column; padding: 1.5rem;">
      <div class="modal-header">
        <div>
          <h3>{printModalTitle}</h3>
          <span class="drug-barcode-badge mt-1">{printDrugsList.length} référence(s)</span>
        </div>
        <button class="btn-close-modal" onclick={() => showPrintBarcodeModal = false}>&times;</button>
      </div>

      <div class="modal-body modal-table-scroll mt-1" style="flex: 1; overflow-y: auto;">
        <p class="print-instruction">
          Vérifiez les étiquettes ci-dessous et ajustez le nombre d'exemplaires à imprimer si besoin avant de lancer l'impression.
        </p>

        <div class="barcode-preview-grid">
          {#each printDrugsList as drug}
            <div class="barcode-card-item">
              <div class="barcode-card-header">
                <span class="label-drug-name" title={drug.name}>{drug.name}</span>
              </div>
              <div class="barcode-card-body">
                <svg class="barcode-card-svg" use:barcodeAction={drug.barcode}></svg>
              </div>
              <div class="barcode-card-footer">
                <span class="label-price">PPA: {drug.price_per_item_da.toFixed(2)} DA</span>
                <div class="copies-control">
                  <label for={"copies-" + (drug.printKey || drug.id)}>Exemplaires :</label>
                  <input
                    id={"copies-" + (drug.printKey || drug.id)}
                    type="number"
                    min="1"
                    max="999"
                    class="input-pos copies-input"
                    bind:value={labelCopiesMap[drug.printKey || drug.id]}
                  />
                </div>
              </div>
            </div>
          {/each}
        </div>
      </div>

      <div class="confirm-actions" style="margin-top: 1rem;">
        <button type="button" class="btn-pos-action btn-cancel" onclick={() => showPrintBarcodeModal = false}>
          Fermer
        </button>
        <button type="button" class="btn-action btn-action-primary" onclick={triggerDirectPrint}>
          Imprimer
        </button>
      </div>
    </div>
  </div>
{/if}

<!-- Hidden Area formatted strictly for Thermal / Barcode Printers on window.print() -->
<div id="printable-barcode-area">
  {#each printDrugsList as drug}
    {#each Array(Math.max(1, labelCopiesMap[drug.printKey || drug.id] || 1)) as _}
      <div class="barcode-print-label">
        <div class="print-label-name">{drug.name}</div>
        <svg class="print-label-svg" use:barcodeAction={drug.barcode}></svg>
        <div class="print-label-price">PPA: {drug.price_per_item_da.toFixed(2)} DA</div>
      </div>
    {/each}
  {/each}
</div>

<!-- INVOICE DETAILS MODAL -->
{#if selectedInvoice}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="modal-overlay" onclick={closeInvoiceDetails}>
    <div class="modal-content invoice-details-modal" onclick={(e) => e.stopPropagation()}>
      <!-- Modal Header -->
      <div class="modal-header">
        <div>
          <h3>{t("invoiceDetailsTitle")} - {selectedInvoice.supplier_name}</h3>
          <span class="drug-barcode-badge mt-1">
            N° {selectedInvoice.invoice_number || `#${selectedInvoice.id}`} &bull; Total: {selectedInvoice.total_amount_da.toFixed(2)} DA
          </span>
        </div>
        <button class="btn-close-modal" onclick={closeInvoiceDetails} aria-label="Close">
          &times;
        </button>
      </div>

      <!-- Drugs Table inside Modal -->
      <div class="modal-table-wrap">
        {#if isLoadingDrugs}
          <p class="loading-text">Chargement des articles...</p>
        {:else if invoiceDrugs.length === 0}
          <p class="empty-text">Aucun article enregistré pour cette facture.</p>
        {:else}
          <table class="pos-table modal-table">
            <thead>
              <tr>
                <th>{t("barcode")}</th>
                <th>{t("designation")}</th>
                <th>{t("noLot")}</th>
                <th>{t("exp")}</th>
                <th>{t("quantity")}</th>
                <th>{t("ppa")} (DA)</th>
                <th>{t("lineTotal")} (DA)</th>
              </tr>
            </thead>
            <tbody>
              {#each invoiceDrugs as item}
                <tr>
                  <td>
                    <span class="drug-barcode-badge">{item.barcode || "-"}</span>
                  </td>
                  <td class="bold">{item.name}</td>
                  <td>{item.batch_number || "-"}</td>
                  <td>{item.expiry_date ? formatDateFr(item.expiry_date) : "-"}</td>
                  <td class="bold">{item.packages_received}</td>
                  <td>{item.price_per_item_da.toFixed(2)}</td>
                  <td class="bold">
                    {(item.packages_received * item.price_per_item_da).toFixed(2)}
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        {/if}
      </div>

      <!-- Modal Actions -->
      <div class="modal-actions-row">
        <button 
          type="button" 
          class="btn-pos-action" 
          onclick={() => { 
            const inv = selectedInvoice; 
            closeInvoiceDetails(); 
            if (inv) handlePrintInvoiceBarcodes(inv); 
          }}
        >
          <img src={printerIcon} alt="Print" class="action-btn-icon" />
          <span>Imprimer étiquettes</span>
        </button>

        {#if selectedInvoice.pdf_path}
          <button 
            type="button" 
            class="btn-pos-action" 
            onclick={() => handleOpenPdf(selectedInvoice?.pdf_path || "")}
          >
            <img src={pdfIcon} alt="PDF" class="action-btn-icon" />
            <span>{t("openPdfFile")}</span>
          </button>
        {/if}

        <button type="button" class="btn-pos-action btn-danger" onclick={closeInvoiceDetails}>
          {t("cancel")}
        </button>
      </div>
    </div>
  </div>
{/if}

<!-- DELETE CONFIRMATION MODAL -->
{#if invoiceToDelete}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="modal-overlay" onclick={() => invoiceToDelete = null}>
    <div class="modal-content confirm-modal card" onclick={(e) => e.stopPropagation()}>
      <div class="modal-header">
        <h3 class="danger-title">Supprimer la facture</h3>
        <button class="btn-close-modal" onclick={() => invoiceToDelete = null} aria-label="Fermer">&times;</button>
      </div>

      <p class="confirm-message">
        Que souhaitez-vous faire concernant le stock issu de cette facture ?
      </p>
      
      <div class="delete-summary-box">
        <div><strong>{t("supplier")} :</strong> {invoiceToDelete.supplier_name}</div>
        <div><strong>{t("invoiceNumber")} :</strong> {invoiceToDelete.invoice_number || `#${invoiceToDelete.id}`}</div>
        <div><strong>{t("total")} :</strong> {invoiceToDelete.total_amount_da.toFixed(2)} DA</div>
        {#if invoiceToDelete.total_packages > 0}
          <div><strong>Stock lié :</strong> {invoiceToDelete.total_packages} boîte(s) ({invoiceToDelete.items_count} référence(s))</div>
        {/if}
      </div>

      <div class="delete-options-list">
        <button 
          type="button"
          class="delete-option-card {deleteWithStock === false ? 'selected' : ''}"
          onclick={() => deleteWithStock = false}
        >
          <span class="option-title">Supprimer uniquement la facture</span>
        </button>

        <button 
          type="button"
          class="delete-option-card {deleteWithStock === true ? 'selected danger-card' : ''}"
          onclick={() => deleteWithStock = true}
        >
          <span class="option-title danger-text">Supprimer la facture avec son contenu</span>
        </button>
      </div>

      <div class="confirm-actions">
        <button 
          type="button" 
          class="btn-action btn-action-secondary" 
          disabled={isDeleting}
          onclick={() => invoiceToDelete = null}
        >
          {t("cancel")}
        </button>
        <button 
          type="button" 
          class="btn-action btn-action-danger" 
          disabled={isDeleting || deleteWithStock === null}
          onclick={confirmDelete}
        >
          {isDeleting ? "Suppression..." : "Confirmer la suppression"}
        </button>
      </div>
    </div>
  </div>
{/if}

<!-- IMPORT INVOICE MODAL -->
{#if showImportModal}
  <InvoiceImportModal
    onClose={() => showImportModal = false}
    onImportSuccess={() => {
      showImportModal = false;
      loadInvoices();
    }}
  />
{/if}

<style>
  .stock-workspace {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    width: 100%;
    height: 100%;
    box-sizing: border-box;
  }

  /* Header row matching StockWorkspace */
  .stock-header-actions {
    display: flex;
    justify-content: space-between;
    align-items: center;
    width: 100%;
    gap: 1rem;
    box-sizing: border-box;
    flex-wrap: wrap;
  }

  .search-filter-row {
    display: flex;
    gap: 0.75rem;
    align-items: center;
    flex: 1;
    min-width: 280px;
    flex-wrap: wrap;
  }

  .search-input {
    width: clamp(200px, 35vw, 600px);
    max-width: 600px;
    min-width: 180px;
    flex: 1 1 240px;
    font-size: clamp(0.95rem, 1.1vw, 1.15rem);
  }

  .supplier-select,
  .date-select {
    width: auto;
    flex-shrink: 0;
    font-size: clamp(0.9rem, 1vw, 1.15rem);
    font-weight: 600;
    cursor: pointer;
    white-space: nowrap;
  }

  .stock-header-right-actions {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex-shrink: 0;
  }

  .btn-header-icon-action {
    background: transparent;
    border: none;
    padding: clamp(0.25rem, 0.5vw, 0.35rem) clamp(0.4rem, 0.8vw, 0.65rem);
    cursor: pointer;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 0.25rem;
    transition: transform 0.15s ease, opacity 0.15s ease;
  }

  .btn-header-icon-action:hover {
    transform: translateY(-2px);
  }

  .btn-header-icon-action:active {
    transform: translateY(0);
  }

  .header-action-icon-img {
    width: clamp(24px, 2.5vw, 30px);
    height: clamp(24px, 2.5vw, 30px);
    object-fit: contain;
  }

  .header-action-icon-label {
    font-size: clamp(0.75rem, 0.85vw, 0.85rem);
    font-weight: 700;
    color: var(--color-primary);
    line-height: 1.1;
    white-space: nowrap;
  }

  /* Print Selection Toolbar */
  .print-selection-toolbar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    background-color: rgba(0, 135, 90, 0.08);
    border: 1.5px solid var(--color-primary);
    border-radius: var(--border-radius);
    padding: clamp(0.5rem, 0.8vh, 0.75rem) clamp(0.75rem, 1.2vw, 1.25rem);
    margin-bottom: 0.75rem;
    gap: 1rem;
    flex-wrap: wrap;
  }

  .print-selection-info {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: clamp(0.9rem, 1vw, 1.05rem);
    color: var(--color-primary);
  }

  .print-selection-actions {
    display: flex;
    align-items: center;
    gap: 0.6rem;
  }

  .btn-action {
    padding: clamp(0.4rem, 0.6vh, 0.6rem) clamp(0.8rem, 1vw, 1.2rem);
    border-radius: var(--border-radius);
    font-size: clamp(0.85rem, 0.95vw, 1rem);
    font-weight: 700;
    cursor: pointer;
    transition: all 0.15s ease;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border: none;
  }

  .btn-action-primary {
    background-color: var(--color-primary);
    color: #ffffff;
  }

  .btn-action-primary:hover:not(:disabled) {
    background-color: var(--color-primary-hover);
    transform: translateY(-1px);
  }

  .btn-action-primary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn-action-secondary {
    background-color: var(--color-bg-card);
    border: var(--border-width) solid var(--color-border);
    color: var(--color-text-dark);
  }

  .btn-action-secondary:hover {
    background-color: var(--color-bg-app);
    border-color: var(--color-text-secondary);
  }

  .pos-checkbox {
    width: 18px;
    height: 18px;
    cursor: pointer;
    accent-color: var(--color-primary);
  }

  .checkbox-th, .checkbox-td {
    width: 44px;
    min-width: 44px;
    text-align: center !important;
  }

  .row-selected {
    background-color: rgba(0, 135, 90, 0.08) !important;
  }

  /* Table container matching StockWorkspace */
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

  .pos-table {
    width: 100%;
    min-width: 860px;
    border-collapse: collapse;
    font-size: clamp(0.9rem, 1.05vw, 1.1rem);
  }

  .pos-table th, .pos-table td {
    padding: clamp(0.4rem, 0.8vh, 0.75rem) clamp(0.5rem, 0.8vw, 1rem);
    border-bottom: var(--border-width) solid var(--color-border);
    text-align: left;
    vertical-align: middle;
    box-sizing: border-box;
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

  .sortable-th {
    cursor: pointer;
    user-select: none;
  }

  .sortable-th:hover {
    background-color: var(--color-bg-card);
  }

  .pos-table td.bold {
    font-weight: bold;
  }

  .placeholder-row td {
    height: clamp(32px, 3.8vh, 50px);
  }

  .empty-row-text {
    text-align: center;
    padding: clamp(1rem, 2vh, 2.5rem) 1rem;
    color: #6b778c;
    font-style: italic;
  }

  .drug-title-wrapper {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .drug-name-text {
    font-size: clamp(0.95rem, 1.1vw, 1.1rem);
  }

  .drug-barcode-badge {
    font-size: 0.75rem;
    font-weight: 600;
    background: #ebecf0;
    color: #42526e;
    padding: 0.1rem 0.4rem;
    border-radius: 4px;
    letter-spacing: 0.5px;
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

  /* Pagination matching StockWorkspace */
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
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
  }

  .btn-pos-action:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .page-indicator {
    font-size: clamp(0.95rem, 1.1vw, 1.15rem);
    font-weight: bold;
  }

  /* Modal Close button & Header */
  .modal-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-bottom: 2px solid var(--color-bg-app);
    padding-bottom: 0.75rem;
    margin-bottom: 1rem;
  }

  .modal-subtitle {
    display: block;
    font-size: 0.95rem;
    font-weight: 600;
    color: #42526e;
    margin-top: 0.2rem;
  }

  .btn-close-modal {
    background: transparent;
    border: none;
    font-size: 1.8rem;
    cursor: pointer;
    font-weight: bold;
    line-height: 1;
    padding: 0;
    color: var(--color-danger);
  }

  .btn-close-modal:hover {
    transform: scale(1.1);
  }

  .invoice-details-modal {
    max-width: 900px;
    width: 92vw;
    max-height: 90vh;
    display: flex;
    flex-direction: column;
    padding: clamp(1rem, 1.8vw, 1.5rem);
    overflow: hidden;
  }

  .modal-table-wrap {
    flex: 1;
    overflow: auto;
    border: 1px solid #ebecf0;
    border-radius: 8px;
    margin-bottom: 1rem;
    min-height: 0;
  }

  .modal-table {
    min-width: 650px;
  }

  .modal-actions-row {
    display: flex;
    justify-content: flex-end;
    gap: 0.75rem;
    padding-top: 0.5rem;
  }

  .action-btn-icon {
    width: 20px;
    height: 20px;
    object-fit: contain;
  }

  .btn-cancel {
    background-color: transparent;
  }

  .btn-pos-action.btn-danger {
    background-color: var(--color-danger);
    color: #ffffff;
    border-color: var(--color-danger-hover);
    transition: background-color 0.15s ease;
  }

  .btn-pos-action.btn-danger:hover {
    background-color: var(--color-danger-hover);
    color: #ffffff;
  }

  .confirm-modal {
    max-width: 650px;
    width: 95%;
    padding: 1.5rem;
  }

  .danger-title {
    color: var(--color-danger);
    font-size: 1.35rem;
    margin: 0;
  }

  .confirm-message {
    color: #42526e;
    font-size: 1.05rem;
    margin: 1rem 0;
  }

  /* Choice Modal Grid */
  .pos-choice-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: 1.25rem;
    margin: 1.25rem 0;
  }

  .delete-summary-box {
    background-color: var(--color-bg-app);
    border: 1px solid var(--color-border);
    border-radius: 8px;
    padding: 0.85rem 1.1rem;
    margin-bottom: 1.25rem;
    font-size: 0.95rem;
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
    gap: 0.5rem 1.25rem;
  }

  .delete-options-list {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: 1rem;
    margin-bottom: 1.5rem;
  }

  .delete-option-card {
    display: flex;
    align-items: center;
    justify-content: center;
    text-align: center;
    padding: 1.25rem 1rem;
    border: 2px solid var(--color-border);
    border-radius: 10px;
    cursor: pointer;
    transition: all 0.15s ease;
    background-color: var(--color-bg-card);
    box-sizing: border-box;
    font-family: inherit;
    min-height: 80px;
  }

  .delete-option-card:hover {
    border-color: var(--color-primary);
    background-color: #f7fafc;
    transform: translateY(-2px);
  }

  .delete-option-card.selected {
    border-color: var(--color-primary);
    background-color: rgba(0, 135, 90, 0.08);
    box-shadow: 0 3px 8px rgba(0, 135, 90, 0.2);
  }

  .delete-option-card.selected.danger-card {
    border-color: var(--color-danger);
    background-color: rgba(222, 53, 11, 0.08);
    box-shadow: 0 3px 8px rgba(222, 53, 11, 0.2);
  }

  .option-title {
    font-weight: 700;
    font-size: 1.05rem;
    color: var(--color-text-dark);
    line-height: 1.35;
    text-align: center;
  }

  .option-title.danger-text {
    color: var(--color-danger);
  }

  .confirm-actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.75rem;
  }

  .btn-action-danger {
    background-color: var(--color-danger);
    color: #ffffff;
    border: none;
    border-radius: 8px;
    padding: 0.6rem 1.25rem;
    font-weight: 700;
    cursor: pointer;
  }

  .btn-action-danger:disabled {
    opacity: 0.45;
    cursor: not-allowed;
    transform: none;
    box-shadow: none;
  }

  .btn-pos-choice {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 1.5rem 1rem;
    border: var(--border-width) solid var(--color-border);
    border-radius: var(--border-radius);
    background-color: var(--color-bg-card);
    color: var(--color-text-dark);
    cursor: pointer;
    transition: all 0.15s ease-in-out;
    box-shadow: 0 4px 6px var(--color-shadow);
    gap: 0.65rem;
    text-align: center;
    width: 100%;
    min-height: 155px;
    box-sizing: border-box;
  }

  .btn-pos-choice:hover {
    transform: translateY(-3px);
    box-shadow: 0 8px 16px var(--color-shadow);
    border-color: var(--color-primary);
  }

  .btn-pos-choice-icon {
    width: 48px;
    height: 48px;
    object-fit: contain;
  }

  .btn-pos-choice-label {
    font-size: 1.2rem;
    font-weight: 700;
    color: var(--color-text-dark);
  }

  .btn-pos-choice-desc {
    font-size: 0.95rem;
    font-weight: 600;
    color: #42526e;
    line-height: 1.3;
  }

  /* Barcode Preview Grid */
  .barcode-preview-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    gap: 1rem;
    padding: 0.5rem;
  }

  .barcode-card-item {
    border: 2px solid var(--color-border);
    border-radius: 8px;
    background: #ffffff;
    padding: 0.5rem;
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    box-shadow: 0 2px 4px var(--color-shadow);
  }

  .barcode-card-header {
    width: 100%;
    margin-bottom: 0.25rem;
  }

  .label-drug-name {
    font-size: 0.8rem;
    font-weight: 800;
    display: block;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .barcode-card-body {
    margin: 0.25rem 0;
  }

  .barcode-card-svg {
    max-width: 100%;
    height: 38px;
  }

  .barcode-card-footer {
    width: 100%;
    margin-top: 0.25rem;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .label-price {
    font-size: 0.85rem;
    font-weight: 800;
    color: var(--color-primary);
  }

  .copies-control {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.4rem;
    font-size: 0.85rem;
  }

  .copies-input {
    width: 55px;
    height: 26px;
    text-align: center;
    padding: 0.1rem;
  }

  .print-instruction {
    font-size: 0.95rem;
    color: #5e6c84;
    margin-bottom: 0.75rem;
  }

  /* Direct Barcode Printer Styling on window.print() */
  #printable-barcode-area {
    display: none;
  }

  @media print {
    :global(body *) {
      visibility: hidden !important;
    }
    :global(#printable-barcode-area), :global(#printable-barcode-area *) {
      visibility: visible !important;
    }
    :global(#printable-barcode-area) {
      display: block !important;
      position: absolute !important;
      left: 0 !important;
      top: 0 !important;
      width: 100% !important;
      margin: 0 !important;
      padding: 0 !important;
      background: #ffffff !important;
    }
    .barcode-print-label {
      width: 50mm;
      height: 30mm;
      box-sizing: border-box;
      padding: 2mm 2.5mm;
      margin: 0 auto;
      display: flex;
      flex-direction: column;
      align-items: center;
      justify-content: center;
      text-align: center;
      page-break-inside: avoid;
      page-break-after: always;
      overflow: hidden;
      background: #ffffff !important;
    }
    .print-label-name {
      font-size: 7.5pt;
      font-weight: 800;
      line-height: 1.15;
      max-height: 2.3em;
      overflow: hidden;
      text-transform: uppercase;
      color: #000000 !important;
      margin-bottom: 1mm;
      width: 100%;
      word-break: break-word;
    }
    .print-label-svg {
      max-width: 44mm;
      height: 16mm;
      margin: 0 auto;
    }
    .print-label-price {
      font-size: 8.5pt;
      font-weight: 900;
      color: #000000 !important;
      margin-top: 1mm;
      line-height: 1;
    }
  }
</style>
