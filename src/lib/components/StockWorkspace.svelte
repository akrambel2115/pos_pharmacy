<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { t } from "$lib/i18n/index.svelte";
  import DrugModal from "./DrugModal.svelte";
  import { onMount } from "svelte";
  import plusIcon from "../../public/icons/plus.png";
  import trashIcon from "../../public/icons/trash.png";
  import viewIcon from "../../public/icons/view.png";
  import checkIcon from "../../public/icons/check.png";
  import editIcon from "../../public/icons/edit.png";
  import closepIcon from "../../public/icons/closep.png";
  import uploadIcon from "../../public/icons/upload.png";
  import printerIcon from "../../public/icons/printer.png";
  import historyIcon from "../../public/icons/history.png";
  import stockIcon from "../../public/icons/stock.png";
  import pdfIcon from "../../public/icons/pdf.png";
  import BarcodeScanner from "./BarcodeScanner.svelte";
  import { formatDateFr } from "../utils";
  import InvoiceImportModal from "./InvoiceImportModal.svelte";
  import JsBarcode from "jsbarcode";
  import { permissionsStore } from "$lib/permissions.svelte";

  let { userRole, triggerAdminPIN } = $props<{
    userRole: "cashier" | "admin";
    triggerAdminPIN: (onSuccess: () => void) => void;
  }>();

  // Full Stock List State
  let drugsList = $state<any[]>([]);
  let expiryThresholdDays = $state(30);

  // Selected Invoice Details Modal State
  let selectedInvoiceForView = $state<any | null>(null);
  let invoiceDrugsForView = $state<any[]>([]);
  let isLoadingInvoiceDrugs = $state(false);

  async function openInvoiceDetailsModal(invoiceId?: number | null) {
    if (!invoiceId) return;
    isLoadingInvoiceDrugs = true;
    selectedInvoiceForView = null;
    invoiceDrugsForView = [];
    try {
      selectedInvoiceForView = await invoke<any>("get_imported_invoice_by_id", { invoiceId });
      invoiceDrugsForView = await invoke<any[]>("get_imported_invoice_drugs", { invoiceId });
    } catch (err: any) {
      console.error("Error loading invoice details:", err);
    } finally {
      isLoadingInvoiceDrugs = false;
    }
  }

  function closeInvoiceDetailsModal() {
    selectedInvoiceForView = null;
    invoiceDrugsForView = [];
  }

  async function handleOpenPdfFromStock(pdfPath: string) {
    try {
      await invoke("open_invoice_file", { path: pdfPath });
    } catch (err: any) {
      alert("Erreur lors de l'ouverture du PDF : " + err.toString());
    }
  }

  // Search State
  let searchQuery = $state("");

  // Sort State
  let sortColumn = $state("name");
  let sortDirection = $state<"asc" | "desc">("asc");

  // Pagination State
  let currentPage = $state(1);
  const pageSize = 10;

  // Invoice Import Modal State
  let showInvoiceImportModal = $state(false);

  // Modal Dialogs State
  let showNewDrugModal = $state(false);
  let selectedBarcode = $state("");

  // View Batches Modal State
  let showViewBatchesModal = $state(false);
  let selectedDrugForBatches = $state<any | null>(null);
  let drugBatches = $state<any[]>([]);

  // Expandable row state for stock separation
  let expandedDrugIds = $state<Set<number>>(new Set());
  let drugBatchesMap = $state<Map<number, any[]>>(new Map());

  async function toggleExpandDrug(drugId: number) {
    const next = new Set(expandedDrugIds);
    if (next.has(drugId)) {
      next.delete(drugId);
      expandedDrugIds = next;
    } else {
      next.add(drugId);
      expandedDrugIds = next;
      await loadBatchesForDrug(drugId);
    }
  }

  async function loadBatchesForDrug(drugId: number) {
    try {
      const batches = await invoke<any[]>("get_stock_batches", { drugId });
      const newMap = new Map(drugBatchesMap);
      newMap.set(drugId, batches);
      drugBatchesMap = newMap;
    } catch (err) {
      console.error("Failed to load batches for drug:", err);
    }
  }

  // Add Stock Modal State
  let showAddStockModal = $state(false);
  let selectedDrugForIntake = $state<any | null>(null);
  let intakeExpiry = $state("");
  let intakeBoxes = $state<number>(1);
  let intakePpa = $state<number | null>(null);
  let intakePuht = $state<number | null>(null);
  let intakeTva = $state<number>(9);
  let intakeMg = $state<number | null>(null);
  let itemsPerPackage = $state<number>(1);
  let pageErrorMsg = $state("");
  let intakeErrorMsg = $state("");

  // Inline edit row state
  let showInlineRow = $state(false);
  let inlineBarcode = $state("");
  let inlineName = $state("");
  let inlineStock = $state<number | null>(1);
  let inlineBatchNumber = $state("LOT-01");
  let inlinePpa = $state<number | null>(null);
  let inlinePuht = $state<number | null>(null);
  let inlineExpiry = $state("");
  let inlineTva = $state<number>(9);
  let inlineMg = $state<number | null>(null);
  let inlineIsExisting = $state(false);

  function computeMg(ppa: number | null, puht: number | null, tvaRate: number = 9): number | null {
    if (ppa === null || puht === null || puht <= 0) return null;
    const costTtc = puht * (1 + tvaRate / 100);
    if (costTtc <= 0) return null;
    const margin = ((ppa - costTtc) / costTtc) * 100;
    return Math.round(margin * 10) / 10;
  }

  function handleIntakePpaInput(e: Event) {
    const val = parseFloat((e.target as HTMLInputElement).value);
    intakePpa = isNaN(val) ? null : val;
    intakeMg = computeMg(intakePpa, intakePuht, intakeTva);
  }

  function handleIntakePuhtInput(e: Event) {
    const val = parseFloat((e.target as HTMLInputElement).value);
    intakePuht = isNaN(val) ? null : val;
    intakeMg = computeMg(intakePpa, intakePuht, intakeTva);
  }

  function handleIntakeTvaChange(e: Event) {
    const val = parseFloat((e.target as HTMLSelectElement).value);
    intakeTva = isNaN(val) ? 9 : val;
    intakeMg = computeMg(intakePpa, intakePuht, intakeTva);
  }

  function handleInlinePpaInput(e: Event) {
    const val = parseFloat((e.target as HTMLInputElement).value);
    inlinePpa = isNaN(val) ? null : val;
    inlineMg = computeMg(inlinePpa, inlinePuht, inlineTva);
  }

  function handleInlinePuhtInput(e: Event) {
    const val = parseFloat((e.target as HTMLInputElement).value);
    inlinePuht = isNaN(val) ? null : val;
    inlineMg = computeMg(inlinePpa, inlinePuht, inlineTva);
  }

  function handleInlineTvaChange(e: Event) {
    const val = parseFloat((e.target as HTMLSelectElement).value);
    inlineTva = isNaN(val) ? 9 : val;
    inlineMg = computeMg(inlinePpa, inlinePuht, inlineTva);
  }

  // Edit Drug Modal State
  let showEditDrugModal = $state(false);
  let editDrugId = $state<number | null>(null);
  let editName = $state("");
  let editBarcode = $state("");
  let editPpa = $state<number | null>(null);
  let editPuht = $state<number | null>(null);
  let editTva = $state<number>(9);
  let editMg = $state<number | null>(null);
  let editBatchNumber = $state("");
  let editExpiryDate = $state("");
  let editStockQuantity = $state<number | null>(null);
  let editBatchesCount = $state<number>(1);
  let editErrorMsg = $state("");

  function openEditDrug(drug: any) {
    const proceed = () => {
      editErrorMsg = "";
      editDrugId = drug.id;
      editName = drug.name;
      editBarcode = drug.barcode;
      editPpa = drug.price_per_item_da;
      editPuht = drug.cost_price_da;
      editTva = drug.tva ?? 9;
      editMg = drug.mg ?? computeMg(drug.price_per_item_da, drug.cost_price_da, editTva);
      editBatchNumber = drug.batch_number || "";
      editExpiryDate = drug.nearest_expiry_date || "";
      editStockQuantity = drug.total_stock_pcs ?? 0;
      editBatchesCount = drug.batches_count ?? 1;
      showEditDrugModal = true;
    };

    if (permissionsStore.isAllowed(userRole, "can_edit_drug")) {
      proceed();
    } else if (triggerAdminPIN) {
      triggerAdminPIN(proceed);
    } else {
      alert("Action réservée à l'administrateur");
    }
  }

  function handleEditPpaInput(e: Event) {
    const val = parseFloat((e.target as HTMLInputElement).value);
    editPpa = isNaN(val) ? null : val;
    editMg = computeMg(editPpa, editPuht, editTva);
  }

  function handleEditPuhtInput(e: Event) {
    const val = parseFloat((e.target as HTMLInputElement).value);
    editPuht = isNaN(val) ? null : val;
    editMg = computeMg(editPpa, editPuht, editTva);
  }

  function handleEditTvaChange(e: Event) {
    const val = parseFloat((e.target as HTMLSelectElement).value);
    editTva = isNaN(val) ? 9 : val;
    editMg = computeMg(editPpa, editPuht, editTva);
  }

  async function submitEditDrug(e: SubmitEvent) {
    e.preventDefault();
    editErrorMsg = "";
    if (!editDrugId || !editName.trim()) {
      editErrorMsg = "La désignation du médicament est requise.";
      return;
    }
    if (!editBarcode.trim()) {
      editErrorMsg = "Le code-barres est requis.";
      return;
    }
    if (editPpa === null || editPpa <= 0) {
      editErrorMsg = "Un PPA valide est requis.";
      return;
    }
    if (editPuht === null || editPuht < 0) {
      editErrorMsg = "Un PUHT valide est requis.";
      return;
    }

    try {
      await invoke("update_drug", {
        id: editDrugId,
        name: editName.trim(),
        barcode: editBarcode.trim(),
        pricePerItemDa: editPpa,
        costPriceDa: editPuht,
        tva: editTva,
        mg: editMg ?? 0,
        batchNumber: editBatchNumber.trim() || undefined,
        expiryDate: editExpiryDate || undefined,
        stockQuantity: editBatchesCount <= 1 ? (editStockQuantity ?? 0) : undefined,
      });
      showEditDrugModal = false;
      await loadDrugsList();
    } catch (err: any) {
      editErrorMsg = err.toString();
    }
  }

  // Print Barcode State
  let isPrintMode = $state(false);
  let selectedPrintDrugIds = $state<Set<number>>(new Set());
  let showPrintChoiceModal = $state(false);
  let printChoiceView = $state<"options" | "select_invoice">("options");
  let importedInvoicesList = $state<any[]>([]);
  let isLoadingInvoices = $state(false);
  let showPrintBarcodeModal = $state(false);
  let labelCopiesMap = $state<Record<string, number>>({});
  let customPrintItems = $state<any[] | null>(null);

  function openPrintChoice() {
    printChoiceView = "options";
    showPrintChoiceModal = true;
  }

  function handlePrintAll() {
    showPrintChoiceModal = false;
    customPrintItems = null;
    selectedPrintDrugIds = new Set(drugsList.map(d => d.id));
    const copies: Record<string, number> = {};
    drugsList.forEach(d => {
      copies[String(d.id)] = 1;
    });
    labelCopiesMap = copies;
    showPrintBarcodeModal = true;
  }

  function handleStartSelection() {
    showPrintChoiceModal = false;
    customPrintItems = null;
    isPrintMode = true;
    selectedPrintDrugIds = new Set(filteredDrugs.map(d => d.id));
    const copies: Record<string, number> = {};
    filteredDrugs.forEach(d => {
      copies[String(d.id)] = 1;
    });
    labelCopiesMap = copies;
  }

  async function handlePrintLastInvoice() {
    try {
      isLoadingInvoices = true;
      const items = await invoke<any[]>("get_last_imported_invoice_drugs");
      isLoadingInvoices = false;
      if (!items || items.length === 0) {
        alert("Aucun médicament trouvé dans la dernière facture.");
        return;
      }
      prepareCustomPrintItems(items);
      showPrintChoiceModal = false;
      showPrintBarcodeModal = true;
    } catch (err: any) {
      isLoadingInvoices = false;
      alert("Erreur: " + err.toString());
    }
  }

  async function handleViewSpecificInvoices() {
    try {
      isLoadingInvoices = true;
      importedInvoicesList = await invoke<any[]>("get_imported_invoices");
      isLoadingInvoices = false;
      printChoiceView = "select_invoice";
    } catch (err: any) {
      isLoadingInvoices = false;
      alert("Erreur: " + err.toString());
    }
  }

  async function handleSelectSpecificInvoice(invoiceId: number) {
    try {
      isLoadingInvoices = true;
      const items = await invoke<any[]>("get_imported_invoice_drugs", { invoiceId });
      isLoadingInvoices = false;
      if (!items || items.length === 0) {
        alert("Aucun médicament trouvé dans cette facture.");
        return;
      }
      prepareCustomPrintItems(items);
      showPrintChoiceModal = false;
      showPrintBarcodeModal = true;
    } catch (err: any) {
      isLoadingInvoices = false;
      alert("Erreur: " + err.toString());
    }
  }

  function prepareCustomPrintItems(items: any[]) {
    const formatted = items.map((item, idx) => ({
      id: item.drug_id,
      printKey: `${item.drug_id}_${idx}`,
      name: item.name,
      barcode: item.barcode,
      price_per_item_da: item.price_per_item_da,
      batch_number: item.batch_number,
      expiry_date: item.expiry_date,
    }));
    customPrintItems = formatted;
    const copies: Record<string, number> = {};
    formatted.forEach((item, idx) => {
      const orig = items[idx];
      copies[item.printKey] = orig && orig.packages_received > 0 ? orig.packages_received : 1;
    });
    labelCopiesMap = copies;
  }

  function toggleSelectDrug(id: number) {
    const next = new Set(selectedPrintDrugIds);
    if (next.has(id)) {
      next.delete(id);
    } else {
      next.add(id);
    }
    selectedPrintDrugIds = next;
  }

  let allVisibleSelected = $derived.by(() => {
    if (filteredDrugs.length === 0) return false;
    return filteredDrugs.every(d => selectedPrintDrugIds.has(d.id));
  });

  function toggleSelectAll() {
    if (allVisibleSelected) {
      selectedPrintDrugIds = new Set();
    } else {
      selectedPrintDrugIds = new Set(filteredDrugs.map(d => d.id));
    }
  }

  let selectedDrugsToPrint = $derived.by(() => {
    if (customPrintItems !== null) {
      return customPrintItems;
    }
    return drugsList.filter(d => selectedPrintDrugIds.has(d.id)).map(d => ({
      ...d,
      printKey: String(d.id),
    }));
  });

  function openBarcodePrintPreview() {
    if (selectedPrintDrugIds.size === 0) return;
    customPrintItems = null;
    const copies: Record<string, number> = { ...labelCopiesMap };
    selectedDrugsToPrint.forEach(d => {
      const key = d.printKey || String(d.id);
      if (!copies[key] || copies[key] < 1) {
        copies[key] = 1;
      }
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

  onMount(async () => {
    await loadSettings();
    await loadDrugsList();
  });

  async function loadSettings() {
    try {
      const settings = await invoke<any>("get_settings");
      expiryThresholdDays = settings.expiry_warning_days;
    } catch (err) {
      console.error("Failed to load settings:", err);
    }
  }

  async function loadDrugsList() {
    try {
      drugsList = await invoke<any[]>("get_drugs_stock_list");
    } catch (err) {
      console.error("Failed to load drugs stock list:", err);
    }
  }

  // Filtered list
  let filteredDrugs = $derived.by(() => {
    return drugsList.filter(drug => {
      const q = searchQuery.toLowerCase().trim();
      if (!q) return true;
      const matchesSearch = drug.name.toLowerCase().includes(q) ||
                            (drug.batch_number && drug.batch_number.toLowerCase().includes(q)) ||
                            (drug.barcode && drug.barcode.toLowerCase().includes(q)) ||
                            (drug.invoice_number && drug.invoice_number.toLowerCase().includes(q)) ||
                            (drug.supplier_name && drug.supplier_name.toLowerCase().includes(q));
      return matchesSearch;
    });
  });

  // Sorted list
  let sortedDrugs = $derived.by(() => {
    let list = [...filteredDrugs];
    list.sort((a, b) => {
      let valA: any = "";
      let valB: any = "";

      if (sortColumn === "barcode") {
        valA = a.barcode;
        valB = b.barcode;
      } else if (sortColumn === "name") {
        valA = a.name.toLowerCase();
        valB = b.name.toLowerCase();
      } else if (sortColumn === "price") {
        valA = a.price_per_item_da;
        valB = b.price_per_item_da;
      } else if (sortColumn === "puht") {
        valA = a.cost_price_da;
        valB = b.cost_price_da;
      } else if (sortColumn === "batch") {
        valA = a.batch_number || "";
        valB = b.batch_number || "";
      } else if (sortColumn === "invoice") {
        valA = (a.invoice_number || "").toLowerCase();
        valB = (b.invoice_number || "").toLowerCase();
      } else if (sortColumn === "tva") {
        valA = a.tva ?? 9;
        valB = b.tva ?? 9;
      } else if (sortColumn === "mg") {
        valA = a.mg ?? 0;
        valB = b.mg ?? 0;
      } else if (sortColumn === "stock") {
        valA = a.total_stock_pcs;
        valB = b.total_stock_pcs;
      } else if (sortColumn === "expiration") {
        valA = a.nearest_expiry_date || "9999-12-31";
        valB = b.nearest_expiry_date || "9999-12-31";
      }

      if (valA < valB) return sortDirection === "asc" ? -1 : 1;
      if (valA > valB) return sortDirection === "asc" ? 1 : -1;
      return 0;
    });
    return list;
  });

  // Paginated list
  let totalPages = $derived(Math.ceil(sortedDrugs.length / pageSize) || 1);
  let paginatedDrugs = $derived(
    sortedDrugs.slice((currentPage - 1) * pageSize, currentPage * pageSize)
  );

  function toggleSort(col: string) {
    if (sortColumn === col) {
      sortDirection = sortDirection === "asc" ? "desc" : "asc";
    } else {
      sortColumn = col;
      sortDirection = "asc";
    }
    currentPage = 1;
  }

  function getSortIndicator(col: string) {
    if (sortColumn !== col) return "";
    return sortDirection === "asc" ? " ▲" : " ▼";
  }

  async function checkBarcodeExists(code: string) {
    if (!code) return;
    try {
      const drug = await invoke<any>("get_drug_by_barcode", { barcode: code });
      if (drug) {
        inlineName = drug.name;
        inlinePpa = drug.price_per_item_da;
        inlinePuht = drug.cost_price_da;
        inlineTva = drug.tva ?? 9;
        inlineMg = drug.mg ?? computeMg(inlinePpa, inlinePuht, inlineTva);
        inlineIsExisting = true;
      } else {
        if (inlineIsExisting) {
          inlineName = "";
          inlinePpa = null;
          inlinePuht = null;
          inlineMg = null;
          inlineIsExisting = false;
        }
      }
    } catch (err) {
      console.error(err);
    }
  }

  function handleManualAdd() {
    const proceed = () => {
      pageErrorMsg = "";
      showInlineRow = true;
      inlineBarcode = "";
      inlineName = "";
      inlineStock = 1;
      inlineBatchNumber = "LOT-01";
      inlinePpa = null;
      inlinePuht = null;
      inlineExpiry = "";
      inlineTva = 9;
      inlineMg = null;
      inlineIsExisting = false;
    };

    if (permissionsStore.isAllowed(userRole, "can_add_drug")) {
      proceed();
    } else if (triggerAdminPIN) {
      triggerAdminPIN(proceed);
    } else {
      alert("Action réservée à l'administrateur");
    }
  }

  async function handleBarcodeScan(barcode: string) {
    pageErrorMsg = "";
    showInlineRow = true;
    inlineBarcode = barcode;
    inlineStock = 1;
    inlineBatchNumber = "LOT-01";
    inlineExpiry = "";
    inlineTva = 9;
    inlineMg = null;
    inlineIsExisting = false;
    await checkBarcodeExists(barcode);
  }

  async function submitInlineRow() {
    pageErrorMsg = "";
    if (!inlineName.trim()) {
      pageErrorMsg = "La désignation du médicament est requise.";
      return;
    }
    if (inlineStock === null || inlineStock <= 0) {
      pageErrorMsg = "La quantité de stock est requise.";
      return;
    }
    if (!inlineBatchNumber.trim()) {
      pageErrorMsg = "Le numéro de lot est requis.";
      return;
    }
    if (inlinePpa === null || inlinePpa <= 0) {
      pageErrorMsg = "Un PPA valide est requis.";
      return;
    }
    if (inlinePuht === null || inlinePuht < 0) {
      pageErrorMsg = "Un PUHT valide est requis.";
      return;
    }
    if (!inlineExpiry) {
      pageErrorMsg = "La date d'expiration (Exp) est requise.";
      return;
    }

    const today = new Date();
    today.setHours(0, 0, 0, 0);
    const tomorrow = new Date(today);
    tomorrow.setDate(tomorrow.getDate() + 1);

    const [year, month, day] = inlineExpiry.split("-").map(Number);
    const expiry = new Date(year, month - 1, day);
    expiry.setHours(0, 0, 0, 0);

    if (expiry < tomorrow) {
      pageErrorMsg = t("expiryDateTomorrowError");
      return;
    }

    try {
      await invoke("add_drug_with_batch", {
        barcode: inlineBarcode.trim() || undefined,
        name: inlineName.trim(),
        requiresPrescription: false,
        pricePerItemDa: inlinePpa,
        costPriceDa: inlinePuht ?? 0,
        itemsPerPackage: 1,
        expiryDate: inlineExpiry,
        packagesReceived: inlineStock,
        batchNumber: inlineBatchNumber.trim(),
        tva: inlineTva,
        mg: inlineMg ?? 0,
      });
      showInlineRow = false;
      inlineBarcode = "";
      inlineName = "";
      inlineStock = 1;
      inlineBatchNumber = "LOT-01";
      inlinePpa = null;
      inlinePuht = null;
      inlineExpiry = "";
      inlineTva = 9;
      inlineMg = null;
      inlineIsExisting = false;
      await loadDrugsList();
    } catch (err: any) {
      pageErrorMsg = "Error adding stock: " + err.toString();
    }
  }

  function handleNewDrugSubmit() {
    showNewDrugModal = false;
    loadDrugsList();
  }

  async function openViewBatches(drug: any) {
    selectedDrugForBatches = drug;
    try {
      drugBatches = await invoke<any[]>("get_stock_batches", { drugId: drug.id });
      showViewBatchesModal = true;
    } catch (err) {
      console.error(err);
    }
  }

  let intakeBatchNumber = $state("LOT-01");

  function openAddStock(drug: any) {
    const proceed = () => {
      selectedDrugForIntake = drug;
      intakeExpiry = "";
      intakeBoxes = 1;
      intakeBatchNumber = "LOT-01";
      intakePpa = drug.price_per_item_da;
      intakePuht = drug.cost_price_da;
      intakeTva = drug.tva ?? 9;
      intakeMg = drug.mg ?? computeMg(intakePpa, intakePuht, intakeTva);
      itemsPerPackage = drug.items_per_package;
      intakeErrorMsg = "";
      showAddStockModal = true;
    };

    if (permissionsStore.isAllowed(userRole, "can_add_drug")) {
      proceed();
    } else if (triggerAdminPIN) {
      triggerAdminPIN(proceed);
    } else {
      alert("Action réservée à l'administrateur");
    }
  }

  async function submitAddStock(e: SubmitEvent) {
    e.preventDefault();
    if (!intakeExpiry || !selectedDrugForIntake) return;
    intakeErrorMsg = "";

    const today = new Date();
    today.setHours(0, 0, 0, 0);
    const tomorrow = new Date(today);
    tomorrow.setDate(tomorrow.getDate() + 1);

    const [year, month, day] = intakeExpiry.split("-").map(Number);
    const expiry = new Date(year, month - 1, day);
    expiry.setHours(0, 0, 0, 0);

    if (expiry < tomorrow) {
      intakeErrorMsg = t("expiryDateTomorrowError");
      return;
    }

    try {
      await invoke("add_drug_with_batch", {
        barcode: selectedDrugForIntake.barcode,
        name: selectedDrugForIntake.name,
        requiresPrescription: selectedDrugForIntake.requires_prescription,
        pricePerItemDa: intakePpa ?? selectedDrugForIntake.price_per_item_da,
        costPriceDa: intakePuht ?? selectedDrugForIntake.cost_price_da,
        itemsPerPackage: 1,
        expiryDate: intakeExpiry,
        packagesReceived: intakeBoxes,
        batchNumber: intakeBatchNumber.trim() || "LOT-01",
        tva: intakeTva,
        mg: intakeMg ?? 0,
      });
      showAddStockModal = false;
      await loadDrugsList();
    } catch (err: any) {
      intakeErrorMsg = "Error adding stock: " + err.toString();
    }
  }

  async function confirmDelete(drug: any) {
    pageErrorMsg = "";
    const proceed = async () => {
      if (confirm(t("confirmDeleteDrug"))) {
        try {
          await invoke("delete_drug", { id: drug.id });
          await loadDrugsList();
        } catch (err: any) {
          pageErrorMsg = err.toString();
        }
      }
    };

    if (permissionsStore.isAllowed(userRole, "can_delete_drug")) {
      await proceed();
    } else if (triggerAdminPIN) {
      triggerAdminPIN(proceed);
    } else {
      alert("Action réservée à l'administrateur");
    }
  }

  async function confirmDeleteBatch(batchId: number, drugId?: number) {
    pageErrorMsg = "";
    const proceed = async () => {
      if (confirm("Êtes-vous sûr de vouloir supprimer ce lot de stock ?")) {
        try {
          await invoke("delete_stock_batch", { batchId });
          await loadDrugsList();
          if (drugId) {
            await loadBatchesForDrug(drugId);
            if (selectedDrugForBatches && selectedDrugForBatches.id === drugId) {
              drugBatches = await invoke<any[]>("get_stock_batches", { drugId });
              if (drugBatches.length === 0) {
                showViewBatchesModal = false;
              }
            }
          }
        } catch (err: any) {
          pageErrorMsg = err.toString();
        }
      }
    };

    if (permissionsStore.isAllowed(userRole, "can_delete_drug")) {
      await proceed();
    } else if (triggerAdminPIN) {
      triggerAdminPIN(proceed);
    } else {
      alert("Action réservée à l'administrateur");
    }
  }

  function getDaysUntilExpiry(expiryDateStr: string): number {
    const today = new Date();
    today.setHours(0, 0, 0, 0);
    const expiry = new Date(expiryDateStr);
    expiry.setHours(0, 0, 0, 0);
    const diffTime = expiry.getTime() - today.getTime();
    return Math.ceil(diffTime / (1000 * 60 * 60 * 24));
  }
</script>

<div class="stock-workspace">
  <!-- Hidden scanner wedge to capture global scans -->
  <div style="position: absolute; opacity: 0; pointer-events: none; width: 1px; height: 1px; overflow: hidden;">
    <BarcodeScanner onScan={handleBarcodeScan} />
  </div>

  {#if pageErrorMsg}
    <div class="error-banner">{pageErrorMsg}</div>
  {/if}

  <!-- Stock Search Filters & Add Actions -->
  <div class="stock-header-actions">
    <div class="search-filter-row">
      <input 
        type="text" 
        class="input-pos search-input" 
        placeholder={t("searchBarcodeOrName")} 
        bind:value={searchQuery}
        oninput={() => currentPage = 1}
      />
      <button onclick={handleManualAdd} class="btn-add-manual-stock" title={t("addNewDrug")}>
        <img src={plusIcon} alt="Add" class="plus-icon-img" />
      </button>
    </div>
  </div>

  <!-- Stock list Table -->
  <div class="cart-section">
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
            <th onclick={() => toggleSort("name")} class="sortable-th">
              {t("designation")}{getSortIndicator("name")}
            </th>
            <th onclick={() => toggleSort("stock")} class="sortable-th">
              {t("quantity")}{getSortIndicator("stock")}
            </th>
            <th onclick={() => toggleSort("batch")} class="sortable-th">
              {t("noLot")}{getSortIndicator("batch")}
            </th>
            <th onclick={() => toggleSort("invoice")} class="sortable-th">
              Facture{getSortIndicator("invoice")}
            </th>
            <th onclick={() => toggleSort("price")} class="sortable-th">
              {t("ppa")} (DA){getSortIndicator("price")}
            </th>
            <th onclick={() => toggleSort("puht")} class="sortable-th">
              {t("puht")} (DA){getSortIndicator("puht")}
            </th>
            <th onclick={() => toggleSort("expiration")} class="sortable-th">
              {t("exp")}{getSortIndicator("expiration")}
            </th>
            <th onclick={() => toggleSort("tva")} class="sortable-th">
              {t("tva")} (%){getSortIndicator("tva")}
            </th>
            <th onclick={() => toggleSort("mg")} class="sortable-th">
              {t("mg")} (%){getSortIndicator("mg")}
            </th>
            <th>{t("actions")}</th>
          </tr>
        </thead>
        <tbody>
          {#if showInlineRow}
            <tr class="inline-edit-row">
              {#if isPrintMode}
                <td>&nbsp;</td>
              {/if}
              <td>
                <input 
                  type="text" 
                  class="input-pos table-input bold" 
                  placeholder={t("designation")} 
                  bind:value={inlineName} 
                  disabled={inlineIsExisting}
                />
              </td>
              <td>
                <input 
                  type="number" 
                  min="1"
                  class="input-pos table-input bold" 
                  placeholder={t("quantity")} 
                  bind:value={inlineStock} 
                />
              </td>
              <td>
                <input 
                  type="text" 
                  class="input-pos table-input" 
                  placeholder={t("noLot")} 
                  bind:value={inlineBatchNumber} 
                />
              </td>
              <td>
                <span class="badge-manual">-</span>
              </td>
              <td>
                <input 
                  type="number" 
                  step="0.01"
                  min="0"
                  class="input-pos table-input" 
                  placeholder={t("ppa")} 
                  value={inlinePpa ?? ""} 
                  oninput={handleInlinePpaInput}
                />
              </td>
              <td>
                <input 
                  type="number" 
                  step="0.01"
                  min="0"
                  class="input-pos table-input" 
                  placeholder={t("puht")} 
                  value={inlinePuht ?? ""} 
                  oninput={handleInlinePuhtInput}
                />
              </td>
              <td>
                <input 
                  type="date" 
                  class="input-pos table-input" 
                  bind:value={inlineExpiry} 
                />
              </td>
              <td>
                <select class="input-pos table-input" bind:value={inlineTva} onchange={handleInlineTvaChange}>
                  <option value={0}>0%</option>
                  <option value={9}>9%</option>
                  <option value={19}>19%</option>
                </select>
              </td>
              <td>
                <input 
                  type="number" 
                  step="0.1"
                  class="input-pos table-input" 
                  placeholder="Auto" 
                  bind:value={inlineMg} 
                />
              </td>
              <td>
                <div class="patient-actions-wrapper">
                  <button onclick={submitInlineRow} class="patient-action-btn" title="Enregistrer">
                    <img src={checkIcon} alt="Save" class="patient-action-icon" />
                  </button>
                  <button onclick={() => showInlineRow = false} class="patient-action-btn" title="Annuler">
                    <img src={closepIcon} alt="Cancel" class="patient-action-icon" />
                  </button>
                </div>
              </td>
            </tr>
          {/if}

          {#each paginatedDrugs as drug}
            {@const days = drug.nearest_expiry_date ? getDaysUntilExpiry(drug.nearest_expiry_date) : 9999}
            <tr>
              {#if isPrintMode}
                <td class="checkbox-td">
                  <input 
                    type="checkbox" 
                    class="pos-checkbox" 
                    checked={selectedPrintDrugIds.has(drug.id)} 
                    onchange={() => toggleSelectDrug(drug.id)} 
                  />
                </td>
              {/if}
              <td class="bold">
                <div class="drug-title-wrapper">
                  {#if (drug.batches_count ?? 0) > 1}
                    <button 
                      type="button" 
                      class="btn-expand-batches" 
                      onclick={() => toggleExpandDrug(drug.id)}
                      title={expandedDrugIds.has(drug.id) ? "Masquer les stocks" : "Afficher les stocks de ce médicament"}
                    >
                      <span class="expand-arrow">{expandedDrugIds.has(drug.id) ? "▼" : "▶"}</span>
                    </button>
                  {/if}
                  <span class="drug-name-text">{drug.name}</span>
                  <span class="drug-barcode-badge" title="Code-barres unique">{drug.barcode}</span>
                </div>
              </td>
              <td class="bold">
                {#if drug.total_stock_pcs <= 0}
                  <span class="badge-expired">0</span>
                {:else}
                  {drug.total_stock_pcs}
                {/if}
              </td>
              <td>{drug.batch_number || '-'}</td>
              <td class="invoice-td">
                {#if drug.invoice_number}
                  <button 
                    type="button" 
                    class="badge-invoice-link" 
                    onclick={() => openInvoiceDetailsModal(drug.invoice_id)}
                    title="{drug.supplier_name ? 'Fournisseur : ' + drug.supplier_name + ' | ' : ''}Cliquer pour voir la facture"
                  >
                    <span class="inv-num">{drug.invoice_number}</span>
                    {#if drug.supplier_name}
                      <span class="inv-sup">{drug.supplier_name}</span>
                    {/if}
                  </button>
                  {#if (drug.batches_count ?? 1) > 1}
                    <span class="inv-more-tag" title="Plusieurs lots/factures pour ce médicament">
                      +{(drug.batches_count ?? 1) - 1}
                    </span>
                  {/if}
                {:else}
                  <span class="badge-manual">-</span>
                {/if}
              </td>
              <td>{drug.price_per_item_da.toFixed(2)}</td>
              <td>{drug.cost_price_da.toFixed(2)}</td>
              <td>
                {#if drug.nearest_expiry_date}
                  {#if days <= 0}
                    <span class="badge-expired">{formatDateFr(drug.nearest_expiry_date)}</span>
                  {:else if days <= expiryThresholdDays}
                    <span class="badge-warning">{formatDateFr(drug.nearest_expiry_date)}</span>
                  {:else}
                    <span>{formatDateFr(drug.nearest_expiry_date)}</span>
                  {/if}
                {:else}
                  -
                {/if}
              </td>
              <td>{drug.tva ?? 9}%</td>
              <td>{drug.mg !== undefined && drug.mg !== null ? drug.mg.toFixed(1) + '%' : '-'}</td>
              <td>
                <div class="patient-actions-wrapper">
                  <!-- View batches -->
                  <button onclick={() => openViewBatches(drug)} class="patient-action-btn" title="Voir tous les stocks/lots">
                    <img src={viewIcon} alt="View" class="patient-action-icon" />
                  </button>
                  <!-- Edit drug -->
                  <button onclick={() => openEditDrug(drug)} class="patient-action-btn" title="Modifier le médicament">
                    <img src={editIcon} alt="Edit" class="patient-action-icon" />
                  </button>
                  <!-- Add stock -->
                  <button onclick={() => openAddStock(drug)} class="patient-action-btn" title="Ajouter du stock pour ce médicament">
                    <img src={plusIcon} alt="Add" class="patient-action-icon" />
                  </button>
                  <!-- Delete drug -->
                  <button onclick={() => confirmDelete(drug)} class="patient-action-btn" title="Supprimer médicament">
                    <img src={trashIcon} alt="Delete" class="patient-action-icon" />
                  </button>
                </div>
              </td>
            </tr>

            {#if expandedDrugIds.has(drug.id)}
              <tr class="sub-batches-row">
                <td colspan={isPrintMode ? 11 : 10} class="sub-batches-td">
                  <div class="sub-batches-container">
                    <div class="sub-batches-header">
                      <span class="sub-batches-title">Stocks de <strong>{drug.name}</strong> (Code-barres : <code>{drug.barcode}</code>)</span>
                    </div>
                    {#if !drugBatchesMap.get(drug.id) || drugBatchesMap.get(drug.id)?.length === 0}
                      <div class="sub-batches-empty">Aucun lot de stock enregistré pour ce médicament.</div>
                    {:else}
                      <table class="sub-batches-table">
                        <thead>
                          <tr>
                            <th>{t("noLot")}</th>
                            <th>Facture</th>
                            <th>{t("exp")}</th>
                            <th>Stock (pcs)</th>
                            <th>{t("ppa")} (DA)</th>
                            <th>{t("puht")} (DA)</th>
                            <th>TVA</th>
                            <th>Marge</th>
                            <th>État</th>
                            <th>{t("actions")}</th>
                          </tr>
                        </thead>
                        <tbody>
                          {#each drugBatchesMap.get(drug.id) || [] as b}
                            {@const bDays = getDaysUntilExpiry(b.expiry_date)}
                            <tr>
                              <td class="bold">{b.batch_number || '-'}</td>
                              <td>
                                {#if b.invoice_number}
                                  <button 
                                    type="button" 
                                    class="badge-invoice-link" 
                                    onclick={() => openInvoiceDetailsModal(b.invoice_id)}
                                    title="Voir facture {b.invoice_number}"
                                  >
                                    <span class="inv-num">{b.invoice_number}</span>
                                    {#if b.supplier_name}
                                      <span class="inv-sup">{b.supplier_name}</span>
                                    {/if}
                                  </button>
                                {:else}
                                  <span class="badge-manual">Manuel</span>
                                {/if}
                              </td>
                              <td>{formatDateFr(b.expiry_date)}</td>
                              <td class="bold">{b.items_remaining} pcs</td>
                              <td>{b.price_per_item_da > 0 ? b.price_per_item_da.toFixed(2) : drug.price_per_item_da.toFixed(2)}</td>
                              <td>{b.cost_price_da > 0 ? b.cost_price_da.toFixed(2) : drug.cost_price_da.toFixed(2)}</td>
                              <td>{b.tva ?? drug.tva ?? 9}%</td>
                              <td>{b.mg > 0 ? b.mg.toFixed(1) + '%' : '-'}</td>
                              <td>
                                {#if bDays <= 0}
                                  <span class="badge-expired">{t("expired")}</span>
                                {:else if bDays <= expiryThresholdDays}
                                  <span class="badge-warning">{t("expiresIn", { days: bDays })}</span>
                                {:else}
                                  <span class="status-ok">{t("statusNormal")}</span>
                                {/if}
                              </td>
                              <td>
                                <div class="patient-actions-wrapper">
                                  <button onclick={() => confirmDeleteBatch(b.id, drug.id)} class="patient-action-btn" title="Supprimer ce lot">
                                    <img src={trashIcon} alt="Delete" class="patient-action-icon" />
                                  </button>
                                </div>
                              </td>
                            </tr>
                          {/each}
                        </tbody>
                      </table>
                    {/if}
                  </div>
                </td>
              </tr>
            {/if}
          {/each}
          {#each Array(Math.max(0, 10 - paginatedDrugs.length)) as _}
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

<!-- Modal Dialogs -->

{#if showNewDrugModal}
  <DrugModal
    barcode={selectedBarcode}
    {userRole}
    onSubmit={handleNewDrugSubmit}
    onCancel={() => showNewDrugModal = false}
  />
{/if}

<!-- View Batches Modal -->
{#if showViewBatchesModal && selectedDrugForBatches}
  <div class="modal-overlay">
    <div class="modal-content card max-w-4xl">
      <div class="modal-header">
        <div>
          <h3>{selectedDrugForBatches.name} - Lots</h3>
          <span class="drug-barcode-badge mt-1">Code-barres : {selectedDrugForBatches.barcode}</span>
        </div>
        <button class="btn-close-modal" onclick={() => showViewBatchesModal = false}>&times;</button>
      </div>
      <div class="modal-body mt-1 modal-table-scroll">
        {#if drugBatches.length === 0}
          <p class="empty-text">Aucun lot de stock disponible.</p>
        {:else}
          <table class="pos-table">
            <thead>
              <tr>
                <th>{t("noLot")}</th>
                <th>Facture</th>
                <th>{t("exp")}</th>
                <th>{t("quantity")}</th>
                <th>{t("ppa")} (DA)</th>
                <th>{t("puht")} (DA)</th>
                <th>TVA</th>
                <th>Marge</th>
                <th>Status</th>
                <th>{t("actions")}</th>
              </tr>
            </thead>
            <tbody>
              {#each drugBatches as batch}
                {@const days = getDaysUntilExpiry(batch.expiry_date)}
                <tr>
                  <td class="bold">{batch.batch_number || '-'}</td>
                  <td>
                    {#if batch.invoice_number}
                      <button 
                        type="button" 
                        class="badge-invoice-link" 
                        onclick={() => openInvoiceDetailsModal(batch.invoice_id)}
                        title="Voir facture {batch.invoice_number}"
                      >
                        <span class="inv-num">{batch.invoice_number}</span>
                        {#if batch.supplier_name}
                          <span class="inv-sup">{batch.supplier_name}</span>
                        {/if}
                      </button>
                    {:else}
                      <span class="badge-manual">Manuel</span>
                    {/if}
                  </td>
                  <td>{formatDateFr(batch.expiry_date)}</td>
                  <td class="bold">{batch.items_remaining} pcs</td>
                  <td>{batch.price_per_item_da > 0 ? batch.price_per_item_da.toFixed(2) : '-'}</td>
                  <td>{batch.cost_price_da > 0 ? batch.cost_price_da.toFixed(2) : '-'}</td>
                  <td>{batch.tva}%</td>
                  <td>{batch.mg > 0 ? batch.mg.toFixed(1) + '%' : '-'}</td>
                  <td>
                    {#if days <= 0}
                      <span class="badge-expired">{t("expired")}</span>
                    {:else if days <= expiryThresholdDays}
                      <span class="badge-warning">{t("expiresIn", { days })}</span>
                    {:else}
                      <span class="status-ok">{t("statusNormal")}</span>
                    {/if}
                  </td>
                  <td>
                    <div class="patient-actions-wrapper">
                      <button onclick={() => confirmDeleteBatch(batch.id, selectedDrugForBatches?.id)} class="patient-action-btn" title="Supprimer ce lot">
                        <img src={trashIcon} alt="Delete" class="patient-action-icon" />
                      </button>
                    </div>
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        {/if}
      </div>
      <div class="form-actions mt-2">
        <button type="button" class="btn-action btn-action-danger" onclick={() => showViewBatchesModal = false}>
          Fermer
        </button>
      </div>
    </div>
  </div>
{/if}

<!-- Add Stock Modal -->
{#if showAddStockModal && selectedDrugForIntake}
  <div class="modal-overlay">
    <div class="modal-content card max-w-md">
      <div class="modal-header">
        <div>
          <h3>{t("addStock")} - {selectedDrugForIntake.name}</h3>
          <span class="drug-barcode-badge mt-1">Code-barres : {selectedDrugForIntake.barcode}</span>
        </div>
        <button class="btn-close-modal" onclick={() => showAddStockModal = false}>&times;</button>
      </div>
      <form onsubmit={submitAddStock} class="intake-form mt-1">
        {#if intakeErrorMsg}
          <div class="error-banner">{intakeErrorMsg}</div>
        {/if}

        <div class="form-row-2">
          <div class="form-group">
            <label for="modal-batch">{t("noLot")} *</label>
            <input
              id="modal-batch"
              type="text"
              class="input-pos"
              bind:value={intakeBatchNumber}
              placeholder="e.g. LOT-24A1"
              required
            />
          </div>

          <div class="form-group">
            <label for="modal-boxes">{t("quantity")} (bxs) *</label>
            <input
              id="modal-boxes"
              type="number"
              min="1"
              class="input-pos"
              bind:value={intakeBoxes}
              required
            />
          </div>
        </div>

        <div class="form-group mt-1">
          <label for="modal-expiry">{t("exp")} *</label>
          <input
            id="modal-expiry"
            type="date"
            class="input-pos"
            bind:value={intakeExpiry}
            required
          />
        </div>

        <div class="form-row-2 mt-1">
          <div class="form-group">
            <label for="modal-ppa">{t("ppa")} (DA) *</label>
            <input
              id="modal-ppa"
              type="number"
              step="0.01"
              min="0"
              class="input-pos"
              value={intakePpa ?? ""}
              oninput={handleIntakePpaInput}
              required
            />
          </div>

          <div class="form-group">
            <label for="modal-puht">{t("puht")} (DA) *</label>
            <input
              id="modal-puht"
              type="number"
              step="0.01"
              min="0"
              class="input-pos"
              value={intakePuht ?? ""}
              oninput={handleIntakePuhtInput}
              required
            />
          </div>
        </div>

        <div class="form-row-2 mt-1">
          <div class="form-group">
            <label for="modal-tva">TVA (%)</label>
            <select id="modal-tva" class="input-pos" bind:value={intakeTva} onchange={handleIntakeTvaChange}>
              <option value={0}>0%</option>
              <option value={9}>9%</option>
              <option value={19}>19%</option>
            </select>
          </div>

          <div class="form-group">
            <label for="modal-mg">Marge (%)</label>
            <input
              id="modal-mg"
              type="number"
              step="0.1"
              class="input-pos"
              bind:value={intakeMg}
              placeholder="Auto"
            />
          </div>
        </div>

        <div class="form-actions mt-2">
          <button type="button" class="btn-action btn-action-danger" onclick={() => showAddStockModal = false}>
            {t("cancel")}
          </button>
          <button type="submit" class="btn-action btn-action-primary">
            Ajouter
          </button>
        </div>
      </form>
    </div>
  </div>
{/if}

<!-- Edit Drug Modal -->
{#if showEditDrugModal && editDrugId}
  <div class="modal-overlay">
    <div class="modal-content card max-w-md">
      <div class="modal-header">
        <div>
          <h3>Modifier le médicament</h3>
          <span class="drug-barcode-badge mt-1">Code-barres : {editBarcode}</span>
        </div>
        <button class="btn-close-modal" onclick={() => showEditDrugModal = false}>&times;</button>
      </div>
      <form onsubmit={submitEditDrug} class="intake-form mt-1">
        {#if editErrorMsg}
          <div class="error-banner">{editErrorMsg}</div>
        {/if}

        <div class="form-group">
          <label for="edit-name">{t("designation")} *</label>
          <input
            id="edit-name"
            type="text"
            class="input-pos bold"
            bind:value={editName}
            placeholder={t("designation")}
            required
          />
        </div>

        <div class="form-group">
          <label for="edit-barcode">Code-barres *</label>
          <input
            id="edit-barcode"
            type="text"
            class="input-pos"
            bind:value={editBarcode}
            placeholder="Code-barres unique"
            required
          />
        </div>

        <div class="form-row-2">
          <div class="form-group">
            <label for="edit-ppa">{t("ppa")} (DA) *</label>
            <input
              id="edit-ppa"
              type="number"
              step="0.01"
              min="0"
              class="input-pos"
              value={editPpa ?? ""}
              oninput={handleEditPpaInput}
              required
            />
          </div>

          <div class="form-group">
            <label for="edit-puht">{t("puht")} (DA) *</label>
            <input
              id="edit-puht"
              type="number"
              step="0.01"
              min="0"
              class="input-pos"
              value={editPuht ?? ""}
              oninput={handleEditPuhtInput}
              required
            />
          </div>
        </div>

        <div class="form-row-2">
          <div class="form-group">
            <label for="edit-tva">TVA (%)</label>
            <select id="edit-tva" class="input-pos" bind:value={editTva} onchange={handleEditTvaChange}>
              <option value={0}>0%</option>
              <option value={9}>9%</option>
              <option value={19}>19%</option>
            </select>
          </div>

          <div class="form-group">
            <label for="edit-mg">Marge (%)</label>
            <input
              id="edit-mg"
              type="number"
              step="0.1"
              class="input-pos"
              bind:value={editMg}
              placeholder="Auto"
            />
          </div>
        </div>

        {#if editBatchesCount <= 1}
          <div class="form-row-2">
            <div class="form-group">
              <label for="edit-batch">{t("noLot")}</label>
              <input
                id="edit-batch"
                type="text"
                class="input-pos"
                bind:value={editBatchNumber}
                placeholder="Ex: LOT-01"
              />
            </div>

            <div class="form-group">
              <label for="edit-expiry">{t("exp")}</label>
              <input
                id="edit-expiry"
                type="date"
                class="input-pos"
                bind:value={editExpiryDate}
              />
            </div>
          </div>

          <div class="form-group">
            <label for="edit-qty">Quantité en stock</label>
            <input
              id="edit-qty"
              type="number"
              min="0"
              class="input-pos bold"
              bind:value={editStockQuantity}
            />
          </div>
        {/if}

        <div class="form-actions mt-2">
          <button type="button" class="btn-action btn-action-danger" onclick={() => showEditDrugModal = false}>
            {t("cancel")}
          </button>
          <button type="submit" class="btn-action btn-action-primary">
            {t("save") || "Enregistrer"}
          </button>
        </div>
      </form>
    </div>
  </div>
{/if}

<!-- Print Choice Modal (All / Selected / Last Invoice / Specific Invoice) -->
{#if showPrintChoiceModal}
  <div class="modal-overlay">
    <div class="modal-content card {printChoiceView === 'select_invoice' ? 'max-w-3xl' : 'max-w-2xl'}">
      <div class="modal-header">
        <div>
          <h3>Options d'impression</h3>
          <span class="modal-subtitle">Sélectionnez la source des étiquettes à imprimer</span>
        </div>
        <button class="btn-close-modal" onclick={() => showPrintChoiceModal = false}>&times;</button>
      </div>

      <div class="modal-body mt-1">
        {#if printChoiceView === "options"}
          <div class="pos-choice-grid">
            <button type="button" class="btn-pos-choice" onclick={handlePrintAll}>
              <img src={stockIcon} alt="Tout imprimer" class="btn-pos-choice-icon" />
              <span class="btn-pos-choice-label">Tout imprimer</span>
              <span class="btn-pos-choice-desc">Tous les médicaments en stock ({drugsList.length})</span>
            </button>

            <button type="button" class="btn-pos-choice" onclick={handleStartSelection}>
              <img src={checkIcon} alt="Sélectionner" class="btn-pos-choice-icon" />
              <span class="btn-pos-choice-label">Sélectionner</span>
              <span class="btn-pos-choice-desc">Par cases à cocher dans la liste</span>
            </button>

            <button type="button" class="btn-pos-choice" onclick={handlePrintLastInvoice} disabled={isLoadingInvoices}>
              <img src={uploadIcon} alt="Dernière facture" class="btn-pos-choice-icon" />
              <span class="btn-pos-choice-label">Dernière facture</span>
              <span class="btn-pos-choice-desc">Médicaments du dernier arrivage</span>
            </button>

            <button type="button" class="btn-pos-choice" onclick={handleViewSpecificInvoices} disabled={isLoadingInvoices}>
              <img src={historyIcon} alt="Facture spécifique" class="btn-pos-choice-icon" />
              <span class="btn-pos-choice-label">Facture spécifique</span>
              <span class="btn-pos-choice-desc">Choisir dans l'historique</span>
            </button>
          </div>
        {:else if printChoiceView === "select_invoice"}
          <div class="invoice-selection-view">
            {#if isLoadingInvoices}
              <div class="table-loading-msg">Chargement des factures...</div>
            {:else if importedInvoicesList.length === 0}
              <div class="table-empty-msg">Aucune facture enregistrée dans l'historique.</div>
            {:else}
              <div class="table-scroll-container invoice-table-scroll">
                <table class="pos-table">
                  <thead>
                    <tr>
                      <th>Fournisseur</th>
                      <th>N° Facture</th>
                      <th>Date</th>
                      <th>Total</th>
                      <th>Action</th>
                    </tr>
                  </thead>
                  <tbody>
                    {#each importedInvoicesList as inv}
                      <tr>
                        <td class="bold">{inv.supplier_name}</td>
                        <td>{inv.invoice_number || "—"}</td>
                        <td>{formatDateFr(inv.invoice_date || inv.created_at)}</td>
                        <td class="bold">{inv.total_amount_da.toFixed(2)} DA</td>
                        <td>
                          <button 
                            type="button" 
                            class="btn-action btn-action-primary btn-sm" 
                            onclick={() => handleSelectSpecificInvoice(inv.id)}
                          >
                            Choisir
                          </button>
                        </td>
                      </tr>
                    {/each}
                  </tbody>
                </table>
              </div>
            {/if}
          </div>
        {/if}
      </div>

      <div class="form-actions mt-2">
        {#if printChoiceView === "select_invoice"}
          <button type="button" class="btn-action btn-action-secondary" onclick={() => printChoiceView = "options"}>
            Retour
          </button>
        {/if}
        <button type="button" class="btn-action btn-action-danger" onclick={() => showPrintChoiceModal = false}>
          Annuler
        </button>
      </div>
    </div>
  </div>
{/if}

<!-- Print Barcode Preview Modal -->
{#if showPrintBarcodeModal}
  <div class="modal-overlay">
    <div class="modal-content card max-w-4xl">
      <div class="modal-header">
        <div>
          <h3>Impression des codes-barres</h3>
          <span class="drug-barcode-badge mt-1">{selectedDrugsToPrint.length} étiquette(s)</span>
        </div>
        <button class="btn-close-modal" onclick={() => showPrintBarcodeModal = false}>&times;</button>
      </div>

      <div class="modal-body modal-table-scroll mt-1">
        <p class="print-instruction">
          Vérifiez les étiquettes ci-dessous et ajustez le nombre d'exemplaires à imprimer si besoin avant de lancer l'impression.
        </p>

        <div class="barcode-preview-grid">
          {#each selectedDrugsToPrint as drug}
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

      <div class="form-actions mt-2">
        <button type="button" class="btn-action btn-action-danger" onclick={() => showPrintBarcodeModal = false}>
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
  {#each selectedDrugsToPrint as drug}
    {#each Array(Math.max(1, labelCopiesMap[drug.printKey || drug.id] || 1)) as _}
      <div class="barcode-print-label">
        <div class="print-label-name">{drug.name}</div>
        <svg class="print-label-svg" use:barcodeAction={drug.barcode}></svg>
        <div class="print-label-price">PPA: {drug.price_per_item_da.toFixed(2)} DA</div>
      </div>
    {/each}
  {/each}
</div>

{#if showInvoiceImportModal}
  <InvoiceImportModal
    onClose={() => showInvoiceImportModal = false}
    onImportSuccess={() => loadDrugsList()}
  />
{/if}

<!-- INVOICE DETAILS MODAL -->
{#if selectedInvoiceForView}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="modal-overlay" onclick={closeInvoiceDetailsModal}>
    <div class="modal-content invoice-details-modal card" onclick={(e) => e.stopPropagation()}>
      <!-- Modal Header -->
      <div class="modal-header">
        <div>
          <h3>{t("invoiceDetailsTitle") || "Détails de la facture"} - {selectedInvoiceForView.supplier_name || 'Fournisseur inconnu'}</h3>
          <span class="drug-barcode-badge mt-1">
            N° {selectedInvoiceForView.invoice_number || `#${selectedInvoiceForView.id}`} &bull; Total: {selectedInvoiceForView.total_amount_da.toFixed(2)} DA {#if selectedInvoiceForView.invoice_date}&bull; Date: {formatDateFr(selectedInvoiceForView.invoice_date)}{/if}
          </span>
        </div>
        <button class="btn-close-modal" onclick={closeInvoiceDetailsModal} aria-label="Fermer">
          &times;
        </button>
      </div>

      <!-- Drugs Table inside Modal -->
      <div class="modal-table-wrap">
        {#if isLoadingInvoiceDrugs}
          <p class="table-loading-msg">Chargement des articles...</p>
        {:else if invoiceDrugsForView.length === 0}
          <p class="table-empty-msg">Aucun article enregistré pour cette facture.</p>
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
                <th>{t("puht")} (DA)</th>
                <th>{t("lineTotal")} (DA)</th>
              </tr>
            </thead>
            <tbody>
              {#each invoiceDrugsForView as item}
                <tr>
                  <td>
                    <span class="drug-barcode-badge">{item.barcode || "-"}</span>
                  </td>
                  <td class="bold">{item.name}</td>
                  <td>{item.batch_number || "-"}</td>
                  <td>{item.expiry_date ? formatDateFr(item.expiry_date) : "-"}</td>
                  <td class="bold">{item.packages_received}</td>
                  <td>{item.price_per_item_da.toFixed(2)}</td>
                  <td>{item.cost_price_da > 0 ? item.cost_price_da.toFixed(2) : '-'}</td>
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
        {#if selectedInvoiceForView.pdf_path}
          <button 
            type="button" 
            class="btn-pos-action" 
            onclick={() => handleOpenPdfFromStock(selectedInvoiceForView?.pdf_path || "")}
          >
            <img src={pdfIcon} alt="PDF" class="action-btn-icon" />
            <span>{t("openPdfFile") || "Ouvrir PDF"}</span>
          </button>
        {/if}

        <button type="button" class="btn-pos-action btn-danger" onclick={closeInvoiceDetailsModal}>
          {t("cancel") || "Annuler"}
        </button>
      </div>
    </div>
  </div>
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

  .stock-header-actions {
    display: flex;
    justify-content: space-between;
    align-items: center;
    width: 100%;
    box-sizing: border-box;
  }

  .search-filter-row {
    display: flex;
    gap: 1rem;
    flex: 1;
    max-width: 600px;
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
    width: 32px;
    height: 32px;
    object-fit: contain;
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

  /* Table styling */
  .pos-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 1.1rem;
  }

  .pos-table th, .pos-table td {
    padding: 0.75rem 1rem;
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
    height: 53px;
  }

  .status-ok {
    background-color: var(--color-primary);
    color: #ffffff;
    padding: 0.2rem 0.6rem;
    border-radius: 4px;
    font-weight: bold;
    display: inline-block;
  }
 
  .badge-expired {
    background-color: var(--color-danger);
    color: #ffffff;
    padding: 0.2rem 0.6rem;
    border-radius: 4px;
    font-weight: bold;
    display: inline-block;
  }
 
  .badge-warning {
    background-color: #f59f00;
    color: #ffffff;
    padding: 0.2rem 0.6rem;
    border-radius: 4px;
    font-weight: bold;
    display: inline-block;
  }

  .empty-text {
    color: var(--color-border);
    font-style: italic;
  }

  /* Actions wrapper */
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

  /* Pagination */
  .pagination-container {
    display: flex;
    justify-content: center;
    align-items: center;
    gap: 1.5rem;
    margin-top: 1rem;
  }

  .btn-pos-action {
    background-color: var(--color-bg-app);
    border: var(--border-width) solid var(--color-border);
    border-radius: var(--border-radius);
    padding: 0.5rem 1rem;
    font-size: 1.1rem;
    font-weight: bold;
    cursor: pointer;
  }

  .btn-pos-action:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .page-indicator {
    font-size: 1.15rem;
    font-weight: bold;
  }

  /* Modal Close button */
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
    color: var(--color-danger);
    transition: transform 0.15s ease;
  }
  .btn-close-modal:hover {
    transform: scale(1.15);
  }

  /* Intake modal adjustments */
  .max-w-4xl {
    max-width: 960px;
    width: 92%;
  }

  .modal-table-scroll {
    overflow-x: auto;
    width: 100%;
  }

  .max-w-md {
    max-width: 450px;
    width: 90%;
  }

  .intake-form {
    display: flex;
    flex-direction: column;
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

  .form-actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.75rem;
  }

  .error-banner {
    background-color: rgba(220, 53, 69, 0.15);
    color: var(--color-danger);
    padding: 0.75rem;
    border-radius: var(--border-radius);
    text-align: center;
    font-weight: bold;
    margin-bottom: 1rem;
    width: 100%;
    box-sizing: border-box;
  }

  .table-input {
    width: 100%;
    height: 38px;
    padding: 0.25rem 0.5rem;
    font-size: 0.95rem;
    box-sizing: border-box;
  }

  .inline-edit-row td {
    padding: 0.4rem 0.5rem;
    vertical-align: middle;
  }

  .form-row-2 {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.75rem;
  }

  .drug-title-wrapper {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    width: 100%;
  }

  .btn-expand-batches {
    background: transparent;
    border: none;
    cursor: pointer;
    padding: 0.2rem 0.4rem;
    font-size: 0.85rem;
    color: var(--color-primary);
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 4px;
    transition: background 0.15s ease;
  }

  .btn-expand-batches:hover {
    background-color: var(--color-bg-card);
  }

  .expand-arrow {
    display: inline-block;
  }

  .drug-name-text {
    flex: 1;
  }

  .drug-barcode-badge {
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--color-text-secondary, #6b7280);
    background-color: rgba(0, 0, 0, 0.05);
    padding: 0.15rem 0.4rem;
    border-radius: 4px;
    font-family: monospace;
    display: inline-block;
  }

  .sub-batches-row {
    background-color: rgba(0, 0, 0, 0.02);
  }

  .sub-batches-td {
    padding: 0.5rem 1rem 1rem 2.5rem !important;
  }

  .sub-batches-container {
    background-color: var(--color-bg-card);
    border: 1px solid var(--color-border);
    border-radius: var(--border-radius);
    padding: 0.75rem 1rem;
    box-shadow: 0 2px 4px var(--color-shadow);
  }

  .sub-batches-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 0.5rem;
  }

  .sub-batches-title {
    font-size: 0.95rem;
    font-weight: bold;
    color: var(--color-primary);
  }

  .sub-batches-empty {
    color: var(--color-border);
    font-style: italic;
    padding: 0.5rem 0;
  }

  .sub-batches-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.95rem;
  }

  .sub-batches-table th, .sub-batches-table td {
    padding: 0.4rem 0.6rem;
    border-bottom: 1px solid var(--color-border);
    text-align: left;
  }

  :global([dir="rtl"]) .sub-batches-table th, :global([dir="rtl"]) .sub-batches-table td {
    text-align: right;
  }

  .sub-batches-table th {
    font-weight: 600;
    background-color: var(--color-bg-app);
  }

  /* Invoice badge & link styling */
  .badge-invoice-link {
    display: inline-flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 0.1rem;
    padding: 0.25rem 0.55rem;
    background-color: rgba(30, 77, 43, 0.08);
    border: 1px solid rgba(30, 77, 43, 0.25);
    border-radius: 6px;
    cursor: pointer;
    text-align: left;
    transition: all 0.15s ease-in-out;
    max-width: 170px;
    color: inherit;
    font-family: inherit;
  }

  .badge-invoice-link:hover {
    background-color: var(--color-primary);
    border-color: var(--color-primary);
    transform: translateY(-1px);
    box-shadow: 0 2px 5px var(--color-shadow);
  }

  .badge-invoice-link:hover .inv-num,
  .badge-invoice-link:hover .inv-sup {
    color: #ffffff;
  }

  .inv-num {
    font-size: 0.85rem;
    font-weight: 800;
    color: var(--color-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    width: 100%;
    display: block;
  }

  .inv-sup {
    font-size: 0.72rem;
    font-weight: 600;
    color: #4b5563;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    width: 100%;
    display: block;
  }

  .inv-more-tag {
    display: inline-block;
    margin-left: 0.35rem;
    font-size: 0.72rem;
    font-weight: 700;
    background-color: #e5e7eb;
    color: #374151;
    padding: 0.1rem 0.35rem;
    border-radius: 4px;
    vertical-align: middle;
  }

  .badge-manual {
    font-size: 0.82rem;
    color: #9ca3af;
    font-style: italic;
  }

  .invoice-td {
    white-space: nowrap;
  }

  /* Invoice details modal */
  .invoice-details-modal {
    max-width: 900px;
    width: 90%;
    max-height: 85vh;
    display: flex;
    flex-direction: column;
    padding: 1.5rem;
  }

  .modal-table-wrap {
    flex: 1;
    overflow-y: auto;
    border: 1px solid var(--color-border);
    border-radius: 8px;
    margin: 1rem 0;
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

  .pos-checkbox {
    width: 18px;
    height: 18px;
    cursor: pointer;
    accent-color: var(--color-primary);
  }

  .checkbox-th, .checkbox-td {
    width: 44px;
    text-align: center !important;
  }

  /* Barcode Preview Grid */
  .print-instruction {
    font-size: 0.95rem;
    color: var(--color-text-secondary, #6b7280);
    margin-bottom: 0.75rem;
  }

  .barcode-preview-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
    gap: 1rem;
    width: 100%;
    box-sizing: border-box;
  }

  .barcode-card-item {
    border: 1.5px solid var(--color-border);
    border-radius: var(--border-radius);
    padding: 0.75rem;
    background: #ffffff;
    display: flex;
    flex-direction: column;
    align-items: center;
    box-shadow: 0 2px 4px var(--color-shadow);
  }

  .barcode-card-header {
    width: 100%;
    text-align: center;
    margin-bottom: 0.35rem;
  }

  .label-drug-name {
    font-weight: 800;
    font-size: 0.95rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    display: block;
    color: #111827;
  }

  .barcode-card-body {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 100%;
    margin: 0.35rem 0;
  }

  .barcode-card-svg {
    max-width: 100%;
    height: auto;
  }

  .barcode-card-footer {
    width: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.4rem;
    margin-top: 0.35rem;
    border-top: 1px dashed #e5e7eb;
    padding-top: 0.4rem;
  }

  .label-price {
    font-weight: 800;
    font-size: 1rem;
    color: #111827;
  }

  .copies-control {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    font-size: 0.85rem;
    color: #4b5563;
  }

  .copies-input {
    width: 60px;
    height: 28px;
    padding: 0.1rem 0.3rem;
    text-align: center;
    font-weight: bold;
  }

  .modal-subtitle {
    display: block;
    font-size: 0.95rem;
    font-weight: 600;
    color: #42526e;
    margin-top: 0.2rem;
  }

  .pos-choice-grid {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 1.25rem;
    margin: 1.25rem 0;
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

  .btn-pos-choice:active {
    transform: translateY(0);
  }

  .btn-pos-choice:disabled {
    opacity: 0.45;
    cursor: not-allowed;
    transform: none;
    box-shadow: none;
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

  .invoice-selection-view {
    margin: 0.5rem 0;
    width: 100%;
  }

  .invoice-table-scroll {
    max-height: 340px;
    overflow-y: auto;
  }

  .table-loading-msg, .table-empty-msg {
    text-align: center;
    padding: 2rem 1rem;
    font-weight: 600;
    font-size: 1.1rem;
    color: #42526e;
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