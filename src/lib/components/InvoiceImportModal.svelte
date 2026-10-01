<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import { t } from "$lib/i18n/index.svelte";
  import closeIcon from "../../public/icons/close.png";
  import checkIcon from "../../public/icons/check.png";
  import trashIcon from "../../public/icons/trash.png";
  import plusIcon from "../../public/icons/plus.png";
  import pdfIcon from "../../public/icons/pdf.png";

  let { onClose, onImportSuccess } = $props<{
    onClose: () => void;
    onImportSuccess: () => void;
  }>();

  // Processing state
  let isAnalyzing = $state(false);
  let statusStep = $state("");
  let generalError = $state("");
  let selectedPdfPath = $state("");
  let isDraggingOver = $state(false);
  let unlistenDragDrop: (() => void) | null = null;

  // Invoice metadata
  let supplierName = $state("");
  let invoiceNumber = $state("");
  let invoiceDate = $state("");
  let detectedGrandTotal = $state(0);
  let renderedPages = $state<string[]>([]);
  let currentPreviewPageIndex = $state(0);
  let previewZoom = $state(1.0);

  // Gemini AI Verification state
  let isCheckingWithAi = $state(false);
  let showApiKeyModal = $state(false);
  let geminiApiKeyInput = $state("");
  let aiSuccessMessage = $state("");
  let aiHighlightedCells = $state<Set<string>>(new Set());
  let highlightTimer: any = null;

  // Extracted and validated items
  interface ValidatedItemRow {
    raw_designation: string;
    drug_id: number | null;
    is_new_drug: boolean;
    new_drug_name: string;
    new_drug_barcode: string;
    new_drug_items_per_package: number;
    save_alias: boolean;
    batch_number: string;
    expiry_date: string;
    quantity_packages: number;
    cost_price_da: number;
    ppa_da: number;
    tva: number;
    mg: number;
    total_da: number;
    calculated_total_da: number;
    math_status: "valid" | "mismatch";
    overall_status: "valid" | "warning" | "error";
    validation_messages: string[];
    all_candidates: any[];
  }

  let items = $state<ValidatedItemRow[]>([]);

  function round(val: number): number {
    return Math.round((val || 0) * 100) / 100;
  }

  // Computed totals & math reconciliation
  let calculatedGrandTotal = $derived(
    round(items.reduce((sum, item) => sum + (item.quantity_packages * item.cost_price_da), 0))
  );

  let totalsDiff = $derived(
    round(Math.abs(calculatedGrandTotal - (detectedGrandTotal || 0)))
  );

  let hasCriticalErrors = $derived(
    items.length === 0 || items.some(i => i.overall_status === "error")
  );

  onMount(async () => {
    try {
      const webview = getCurrentWebview();
      unlistenDragDrop = await webview.onDragDropEvent((event) => {
        if (event.payload.type === 'over') {
          isDraggingOver = true;
        } else if (event.payload.type === 'drop') {
          isDraggingOver = false;
          const paths = event.payload.paths;
          if (paths && paths.length > 0) {
            const pdfPath = paths.find((p: string) => p.toLowerCase().endsWith('.pdf')) || paths[0];
            if (pdfPath) {
              selectedPdfPath = pdfPath;
              processInvoice(selectedPdfPath);
            }
          }
        } else {
          isDraggingOver = false;
        }
      });
    } catch (e) {
      console.warn("Tauri drag-drop listener not registered:", e);
    }
  });

  onDestroy(() => {
    if (unlistenDragDrop) {
      unlistenDragDrop();
    }
    if (highlightTimer) {
      clearTimeout(highlightTimer);
    }
  });

  function cleanErrorMessage(rawMsg: string): string {
    if (!rawMsg) return "Une erreur inattendue est survenue.";
    let s = rawMsg.replace(/^(Erreur\s*(Gemini\s*Flash|IA)?\s*:\s*)+/i, "").trim();

    // Check for JSON error payload from remote service
    if (s.includes('"error"') && s.includes('"message"')) {
      try {
        const jsonStart = s.indexOf('{');
        const parsed = JSON.parse(s.slice(jsonStart));
        if (parsed?.error) {
          const code = parsed.error.code;
          if (code === 400 || code === 403) return "Clé API invalide ou accès non autorisé. Vérifiez votre clé dans les Paramètres.";
          if (code === 404) return "Le service d'analyse IA est momentanément indisponible.";
          if (code === 429) return "Quota de requêtes dépassé. Veuillez patienter quelques instants avant de réessayer.";
          if (code === 503) return "Le service d'analyse IA est momentanément surchargé. Veuillez réessayer dans quelques instants.";
          return `Erreur du service IA (code ${code || 'inconnu'}).`;
        }
      } catch {
        // ignore json parse error
      }
    }

    if (s.includes("HTTP (503)") || s.includes("503") || s.toLowerCase().includes("surchargé") || s.toLowerCase().includes("saturé") || s.toLowerCase().includes("high demand") || s.toLowerCase().includes("unavailable")) {
      return "Le service d'analyse IA est momentanément surchargé. Veuillez réessayer dans quelques instants.";
    }
    if (s.includes("HTTP (404)") || s.includes("404")) {
      return "Le service d'analyse IA est momentanément indisponible.";
    }
    if (s.includes("400") || s.includes("403") || s.toLowerCase().includes("api_key_invalid")) {
      return "Clé API invalide ou accès refusé. Vérifiez votre clé dans les Paramètres.";
    }
    if (s.includes("429")) {
      return "Quota de requêtes dépassé. Veuillez patienter quelques instants avant de réessayer.";
    }
    if (s.toLowerCase().includes("connexion") || s.toLowerCase().includes("internet") || s.toLowerCase().includes("failed to fetch")) {
      return "Impossible de joindre le service IA. Vérifiez votre connexion Internet.";
    }

    // Always strip model names from user-facing error messages
    s = s.replace(/avec le modèle\s+gemini-[\w\.\-]+/gi, "par le service IA");
    s = s.replace(/modèle\s+gemini-[\w\.\-]+/gi, "service IA");
    s = s.replace(/gemini-[\w\.\-]+/gi, "service IA");
    s = s.replace(/le modèle\s+[a-zA-Z0-9_\-\.]+\s*:/gi, "l'analyse IA :");

    return s;
  }

  async function handleBrowseFile() {
    try {
      generalError = "";
      const selectedPath = await invoke<string | null>("pick_invoice_pdf_file");
      if (selectedPath) {
        selectedPdfPath = selectedPath;
        await processInvoice(selectedPdfPath);
      }
    } catch (err: any) {
      generalError = "Erreur lors de la sélection du fichier : " + cleanErrorMessage(err?.message || err?.toString() || "");
    }
  }

  async function processInvoice(pdfPath: string) {
    isAnalyzing = true;
    generalError = "";
    statusStep = t("analyzingPdf");

    try {
      // 1. OCR Extraction
      const rawPayload = await invoke<any>("scan_and_extract_invoice", {
        pdfPath: pdfPath,
      });

      if (!rawPayload.success) {
        throw new Error(rawPayload.error || "Échec de l'analyse du document.");
      }

      supplierName = rawPayload.supplier || "Grossiste";
      invoiceNumber = rawPayload.invoice_number || "";
      invoiceDate = rawPayload.invoice_date || "";
      detectedGrandTotal = rawPayload.grand_total || 0;
      renderedPages = rawPayload.pages_rendered || [];
      currentPreviewPageIndex = 0;

      // 2. Catalog Matching & Math Validation
      statusStep = t("matchingCatalog");
      const validatedRows = await invoke<any[]>("validate_and_match_invoice", {
        items: rawPayload.items || [],
      });

      items = validatedRows.map(r => ({
        raw_designation: r.raw_designation,
        drug_id: r.matched_drug ? r.matched_drug.id : null,
        is_new_drug: r.is_new_drug,
        new_drug_name: r.raw_designation,
        new_drug_barcode: "",
        new_drug_items_per_package: 10,
        save_alias: !r.is_new_drug,
        batch_number: r.batch_number || "LOT-AUTO",
        expiry_date: r.expiry_date || "",
        quantity_packages: r.quantity_packages || 1,
        cost_price_da: r.cost_price_da || 0,
        ppa_da: r.ppa_da || (r.cost_price_da ? round(r.cost_price_da * 1.25) : 0),
        tva: r.tva ?? 0.0,
        mg: r.mg ?? 0.0,
        total_da: r.total_da || (r.quantity_packages * r.cost_price_da),
        calculated_total_da: r.calculated_total_da || (r.quantity_packages * r.cost_price_da),
        math_status: r.math_status,
        overall_status: r.overall_status,
        validation_messages: r.validation_messages || [],
        all_candidates: r.all_candidates || [],
      }));

      // Background AI decision on new vs existing medicine if API key is present
      invoke<string>("get_gemini_api_key").then(savedKey => {
        if (savedKey && savedKey.trim() !== "") {
          performAiMedicineMatching(items, savedKey.trim()).then(() => {
            items = [...items];
          });
        }
      }).catch(() => {});

    } catch (err: any) {
      generalError = cleanErrorMessage(err?.message || err?.toString() || "");
    } finally {
      isAnalyzing = false;
      statusStep = "";
    }
  }

  async function performAiMedicineMatching(rows: ValidatedItemRow[], apiKey?: string) {
    const itemsToMatch = rows
      .map((item, idx) => ({
        index: idx,
        raw_designation: item.raw_designation,
        candidates: (item.all_candidates || []).map(c => ({
          id: c.id,
          name: c.name,
        })),
      }))
      .filter(it => it.candidates.length > 0);

    if (itemsToMatch.length === 0) return rows;

    try {
      let keyToUse = apiKey;
      if (!keyToUse) {
        keyToUse = await invoke<string>("get_gemini_api_key");
      }
      if (!keyToUse || !keyToUse.trim()) return rows;

      const decisions = await invoke<Array<{
        index: number;
        is_new_drug: boolean;
        matched_drug_id: number | null;
        reason?: string;
      }>>("ai_match_invoice_drugs", {
        items: itemsToMatch,
        apiKey: keyToUse.trim(),
      });

      for (const dec of decisions) {
        const item = rows[dec.index];
        if (!item) continue;

        if (dec.is_new_drug || !dec.matched_drug_id) {
          item.is_new_drug = true;
          item.drug_id = null;
          item.save_alias = false;
        } else {
          const cand = item.all_candidates.find(c => c.id === dec.matched_drug_id);
          if (cand) {
            item.is_new_drug = false;
            item.drug_id = cand.id;
            item.save_alias = true;
            if (!item.cost_price_da && cand.cost_price_da) {
              item.cost_price_da = cand.cost_price_da;
            }
            if (!item.ppa_da && cand.price_per_item_da) {
              item.ppa_da = cand.price_per_item_da;
            }
          }
        }
        aiHighlightedCells.add(`${dec.index}-desig`);
      }
    } catch (e: any) {
      console.warn("AI drug matching error:", e);
    }
    return rows;
  }

  async function handleCheckWithAi() {
    try {
      generalError = "";
      const savedKey = await invoke<string>("get_gemini_api_key");
      if (!savedKey || savedKey.trim() === "") {
        geminiApiKeyInput = "";
        showApiKeyModal = true;
        return;
      }
      await runGeminiVerification(savedKey);
    } catch (err: any) {
      generalError = cleanErrorMessage(err?.message || err?.toString() || "");
    }
  }

  async function handleSaveApiKeyAndRun() {
    if (!geminiApiKeyInput.trim()) {
      alert("Veuillez saisir votre clé API.");
      return;
    }
    try {
      await invoke("save_gemini_api_key", { apiKey: geminiApiKeyInput.trim() });
      showApiKeyModal = false;
      await runGeminiVerification(geminiApiKeyInput.trim());
    } catch (err: any) {
      generalError = cleanErrorMessage(err?.message || err?.toString() || "");
    }
  }

  async function runGeminiVerification(apiKey?: string) {
    if (!selectedPdfPath) return;
    isCheckingWithAi = true;
    generalError = "";
    aiSuccessMessage = "";

    try {
      // Snapshot current table values before AI updates
      const prevItems = items.map(it => ({ ...it }));

      const rawPayload = await invoke<any>("extract_invoice_with_gemini", {
        pdfPath: selectedPdfPath,
        apiKey: apiKey || null,
      });

      if (!rawPayload.success) {
        throw new Error(rawPayload.error || "Échec de l'analyse avec le service IA.");
      }

      supplierName = rawPayload.supplier || supplierName || "Grossiste";
      invoiceNumber = rawPayload.invoice_number || invoiceNumber || "";
      invoiceDate = rawPayload.invoice_date || invoiceDate || "";
      detectedGrandTotal = rawPayload.grand_total || detectedGrandTotal || 0;
      if (rawPayload.pages_rendered && rawPayload.pages_rendered.length > 0) {
        renderedPages = rawPayload.pages_rendered;
      }

      // Re-run matching against local pharmacy inventory
      const validatedRows = await invoke<any[]>("validate_and_match_invoice", {
        items: rawPayload.items || [],
      });

      const newItems = validatedRows.map(r => ({
        raw_designation: r.raw_designation,
        drug_id: r.matched_drug ? r.matched_drug.id : null,
        is_new_drug: r.is_new_drug,
        new_drug_name: r.raw_designation,
        new_drug_barcode: "",
        new_drug_items_per_package: 10,
        save_alias: !r.is_new_drug,
        batch_number: r.batch_number || "LOT-AUTO",
        expiry_date: r.expiry_date || "",
        quantity_packages: r.quantity_packages || 1,
        cost_price_da: r.cost_price_da || 0,
        ppa_da: r.ppa_da || (r.cost_price_da ? round(r.cost_price_da * 1.25) : 0),
        tva: r.tva ?? 0.0,
        mg: r.mg ?? 0.0,
        total_da: r.total_da || (r.quantity_packages * r.cost_price_da),
        calculated_total_da: r.calculated_total_da || (r.quantity_packages * r.cost_price_da),
        math_status: r.math_status,
        overall_status: r.overall_status,
        validation_messages: r.validation_messages || [],
        all_candidates: r.all_candidates || [],
      }));

      // Detect which cells changed between previous and new values
      const changed = new Set<string>();
      newItems.forEach((newItem, idx) => {
        const oldItem = prevItems[idx];
        if (!oldItem) {
          changed.add(`${idx}-desig`);
          changed.add(`${idx}-qty`);
          changed.add(`${idx}-lot`);
          changed.add(`${idx}-ppa`);
          changed.add(`${idx}-puht`);
          changed.add(`${idx}-exp`);
          changed.add(`${idx}-tva`);
          changed.add(`${idx}-mg`);
          changed.add(`${idx}-total`);
          return;
        }

        if (oldItem.raw_designation.trim() !== newItem.raw_designation.trim()) {
          changed.add(`${idx}-desig`);
        }
        if (oldItem.quantity_packages !== newItem.quantity_packages) {
          changed.add(`${idx}-qty`);
        }
        if (oldItem.batch_number.trim() !== newItem.batch_number.trim()) {
          changed.add(`${idx}-lot`);
        }
        if (Math.abs(oldItem.ppa_da - newItem.ppa_da) >= 0.01) {
          changed.add(`${idx}-ppa`);
        }
        if (Math.abs(oldItem.cost_price_da - newItem.cost_price_da) >= 0.01) {
          changed.add(`${idx}-puht`);
        }
        if (oldItem.expiry_date.trim() !== newItem.expiry_date.trim()) {
          changed.add(`${idx}-exp`);
        }
        if (Math.abs(oldItem.tva - newItem.tva) >= 0.1) {
          changed.add(`${idx}-tva`);
        }
        if (Math.abs(oldItem.mg - newItem.mg) >= 0.1) {
          changed.add(`${idx}-mg`);
        }
        if (Math.abs(oldItem.total_da - newItem.total_da) >= 0.01) {
          changed.add(`${idx}-total`);
        }
      });

      items = newItems;
      aiHighlightedCells = changed;

      // Let AI decide new medicine vs existing candidate for each item
      await performAiMedicineMatching(items, apiKey);

      if (highlightTimer) clearTimeout(highlightTimer);
      highlightTimer = setTimeout(() => {
        aiHighlightedCells = new Set();
      }, 2000);

      const diffNow = round(Math.abs(calculatedGrandTotal - (detectedGrandTotal || 0)));
      if (diffNow <= 1.0) {
        aiSuccessMessage = `Vérification IA réussie : ${newItems.length} lignes vérifiées et médicaments associés. Totaux conformes (Écart : 0.00 DA).`;
      } else {
        aiSuccessMessage = `Vérification IA : ${newItems.length} lignes vérifiées. Total calculé : ${calculatedGrandTotal.toFixed(2)} DA | Facture : ${detectedGrandTotal.toFixed(2)} DA (Écart : ${diffNow.toFixed(2)} DA).`;
      }
      setTimeout(() => {
        aiSuccessMessage = "";
      }, 7000);
    } catch (err: any) {
      generalError = cleanErrorMessage(err?.message || err?.toString() || "");
    } finally {
      isCheckingWithAi = false;
    }
  }

  function recalculateRow(index: number) {
    const item = items[index];
    item.calculated_total_da = round(item.quantity_packages * item.cost_price_da);
    const diff = Math.abs(item.calculated_total_da - item.total_da);
    
    if (diff <= 1.0) {
      item.math_status = "valid";
      item.total_da = item.calculated_total_da; // normalize
    } else {
      item.math_status = "mismatch";
    }

    // Refresh overall status
    if (item.math_status === "mismatch") {
      item.overall_status = "error";
    } else if (item.is_new_drug || !item.expiry_date.trim()) {
      item.overall_status = "warning";
    } else {
      item.overall_status = "valid";
    }
  }

  function onPriceOrCostChange(index: number) {
    const item = items[index];
    if (item.cost_price_da > 0 && item.ppa_da > item.cost_price_da) {
      const costTtc = item.cost_price_da * (1 + (item.tva || 0) / 100);
      item.mg = round(((item.ppa_da - costTtc) / costTtc) * 100);
      if (item.mg < 0) {
        item.mg = round(((item.ppa_da - item.cost_price_da) / item.cost_price_da) * 100);
      }
    }
    recalculateRow(index);
  }

  function onMarginChange(index: number) {
    const item = items[index];
    if (item.cost_price_da > 0) {
      const costTtc = item.cost_price_da * (1 + (item.tva || 0) / 100);
      item.ppa_da = round(costTtc * (1 + (item.mg || 0) / 100));
    }
    recalculateRow(index);
  }

  function handleSelectDrugCandidate(index: number, candidateIdStr: string) {
    const item = items[index];
    if (candidateIdStr === "NEW") {
      item.is_new_drug = true;
      item.drug_id = null;
      item.overall_status = "warning";
    } else {
      const candId = parseInt(candidateIdStr, 10);
      item.is_new_drug = false;
      item.drug_id = candId;
      const cand = item.all_candidates.find(c => c.id === candId);
      if (cand) {
        if (!item.cost_price_da && cand.cost_price_da) {
          item.cost_price_da = cand.cost_price_da;
        }
        if (!item.ppa_da && cand.price_per_item_da) {
          item.ppa_da = cand.price_per_item_da;
        }
      }
      recalculateRow(index);
    }
  }

  function removeRow(index: number) {
    items = items.filter((_, i) => i !== index);
  }

  function addNewRow() {
    items = [
      ...items,
      {
        raw_designation: "Nouveau Produit",
        drug_id: null,
        is_new_drug: true,
        new_drug_name: "",
        new_drug_barcode: "",
        new_drug_items_per_package: 10,
        save_alias: false,
        batch_number: "LOT-AUTO",
        expiry_date: "",
        quantity_packages: 1,
        cost_price_da: 0,
        ppa_da: 0,
        tva: 9.0,
        mg: 0.0,
        total_da: 0,
        calculated_total_da: 0,
        math_status: "valid",
        overall_status: "warning",
        validation_messages: [],
        all_candidates: [],
      }
    ];
  }

  let isSubmitting = $state(false);

  async function handleCommit() {
    if (items.length === 0) return;
    isSubmitting = true;
    generalError = "";

    try {
      const commitPayload = {
        supplier_name: supplierName || "Grossiste",
        invoice_number: invoiceNumber || "INV-SCAN",
        invoice_date: invoiceDate || "",
        total_amount_da: calculatedGrandTotal,
        pdf_path: selectedPdfPath,
        items: items.map(i => ({
          drug_id: i.is_new_drug ? null : i.drug_id,
          create_new_drug: i.is_new_drug,
          new_drug_name: i.is_new_drug ? (i.new_drug_name.trim() || i.raw_designation.trim()) : null,
          new_drug_barcode: i.is_new_drug && i.new_drug_barcode.trim() ? i.new_drug_barcode.trim() : null,
          new_drug_items_per_package: i.is_new_drug ? i.new_drug_items_per_package : null,
          alias_to_save: i.save_alias ? i.raw_designation.trim() : null,
          batch_number: i.batch_number.trim() || "LOT-AUTO",
          expiry_date: i.expiry_date.trim(),
          packages_received: i.quantity_packages,
          cost_price_da: i.cost_price_da,
          ppa_da: i.ppa_da,
          tva: i.tva,
          mg: i.mg,
        }))
      };

      await invoke("commit_imported_invoice", { invoiceData: commitPayload });
      alert(t("invoiceImportSuccess"));
      onImportSuccess();
      onClose();
    } catch (err: any) {
      generalError = cleanErrorMessage(err?.message || err?.toString() || "");
    } finally {
      isSubmitting = false;
    }
  }
</script>

<div class="modal-overlay" onclick={onClose} role="presentation">
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div 
    class="modal-content invoice-import-modal" 
    class:is-compact={items.length === 0 && !isAnalyzing}
    onclick={(e) => e.stopPropagation()} 
    role="dialog" 
    tabindex="-1" 
    aria-modal="true"
  >
    <!-- Header -->
    <div class="import-header">
      <div class="title-group">
        <h2>{t("importInvoiceTitle")}</h2>
        {#if selectedPdfPath}
          <span class="file-path-badge">{selectedPdfPath}</span>
        {/if}
      </div>
      <button class="btn-close" onclick={onClose} aria-label="Close">
        <img src={closeIcon} alt="Close" />
      </button>
    </div>

    <!-- Error Banner -->
    {#if generalError}
      <div class="error-banner">
        <div class="error-banner-content">
          <span class="error-badge">!</span>
          <span>{generalError}</span>
        </div>
        <button class="btn-dismiss-error" onclick={() => generalError = ""} title="Fermer">✕</button>
      </div>
    {/if}

    <!-- Content Area -->
    {#if items.length === 0 && !isAnalyzing}
      <!-- Upload / Select Stage -->
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <div 
        class="upload-dropzone" 
        class:is-dragging-over={isDraggingOver}
        onclick={handleBrowseFile}
        ondragover={(e) => { e.preventDefault(); isDraggingOver = true; }}
        ondragleave={() => { isDraggingOver = false; }}
        ondrop={(e) => {
          e.preventDefault();
          isDraggingOver = false;
          if (e.dataTransfer && e.dataTransfer.files.length > 0) {
            const file = e.dataTransfer.files[0];
            const nativePath = (file as any).path;
            if (nativePath) {
              selectedPdfPath = nativePath;
              processInvoice(selectedPdfPath);
            }
          }
        }}
        role="button"
        tabindex="0"
      >
        <img src={pdfIcon} alt="PDF" class="dropzone-pdf-img" />
        <h3 class="dropzone-title">{t("selectInvoicePdf")}</h3>
        <p class="dropzone-desc">{t("dropPdfHere")}</p>
        
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <div class="actions-group" onclick={(e) => e.stopPropagation()} role="presentation">
          <button type="button" class="btn-action btn-browse" onclick={handleBrowseFile}>
            <span>{t("browseFiles")}</span>
          </button>
        </div>
      </div>
    {:else if isAnalyzing}
      <!-- Loading Stage -->
      <div class="loading-container">
        <div class="spinner"></div>
        <h3>{statusStep}</h3>
        <p>Lecture et vérification de la facture en cours...</p>
      </div>
    {:else}
      <!-- Split Screen Review Stage -->
      <div class="split-container">
        <!-- Left: Scanned PDF Previewer -->
        <div class="preview-panel">
          <div class="preview-controls">
            <span>Page {currentPreviewPageIndex + 1} / {renderedPages.length || 1}</span>
            <div class="zoom-buttons">
              <button onclick={() => previewZoom = Math.max(0.6, previewZoom - 0.2)}>-</button>
              <span>{Math.round(previewZoom * 100)}%</span>
              <button onclick={() => previewZoom = Math.min(2.5, previewZoom + 0.2)}>+</button>
            </div>
            {#if renderedPages.length > 1}
              <div class="page-nav-buttons">
                <button disabled={currentPreviewPageIndex === 0} onclick={() => currentPreviewPageIndex--}>◀</button>
                <button disabled={currentPreviewPageIndex >= renderedPages.length - 1} onclick={() => currentPreviewPageIndex++}>▶</button>
              </div>
            {/if}
          </div>

          <div class="preview-viewport">
            {#if renderedPages.length > 0}
              <img 
                src={renderedPages[currentPreviewPageIndex]} 
                alt="Page de Facture" 
                style="transform: scale({previewZoom}); transform-origin: top center;"
                class="scanned-image"
              />
            {:else}
              <p class="no-preview">Aucun aperçu disponible</p>
            {/if}
          </div>
        </div>

        <!-- Right: Verification & Edit Table -->
        <div class="review-panel">
          <!-- Metadata Bar -->
          <div class="meta-row">
            <div class="meta-field">
              <label for="supplier-inp">{t("supplier")}</label>
              <input id="supplier-inp" type="text" class="input-pos-sm" bind:value={supplierName} />
            </div>
            <div class="meta-field">
              <label for="inv-num-inp">{t("invoiceNumber")}</label>
              <input id="inv-num-inp" type="text" class="input-pos-sm" bind:value={invoiceNumber} />
            </div>
            <div class="meta-field">
              <label for="inv-date-inp">{t("invoiceDate")}</label>
              <input id="inv-date-inp" type="text" class="input-pos-sm" bind:value={invoiceDate} placeholder="YYYY-MM-DD" />
            </div>
            <div class="meta-actions">
              <button 
                type="button" 
                class="btn-action btn-gemini-ai" 
                disabled={isCheckingWithAi || isAnalyzing}
                onclick={handleCheckWithAi}
                title="Vérifier la facture et associer les médicaments avec l'IA"
              >
                {#if isCheckingWithAi}
                  <span class="ai-spinner"></span>
                  <span>{t("aiChecking")}</span>
                {:else}
                  <span>{t("checkWithAi")}</span>
                {/if}
              </button>
            </div>
          </div>

          {#if aiSuccessMessage}
            <div class="ai-success-banner">
              <span>{aiSuccessMessage}</span>
            </div>
          {/if}

          <!-- Financial Math Summary Banner -->
          <div class="financial-summary {totalsDiff <= 1.0 ? 'summary-ok' : 'summary-diff'}">
            <div class="summary-details">
              <div><strong>{t("invoiceGrandTotal")} (Détecté):</strong> {detectedGrandTotal.toFixed(2)} DA</div>
              <div><strong>{t("calculatedTotal")}:</strong> {calculatedGrandTotal.toFixed(2)} DA</div>
              <div><strong>{t("difference")}:</strong> {totalsDiff.toFixed(2)} DA</div>
            </div>
          </div>

          <!-- Items Table -->
          <div class="table-wrapper">
            <table class="review-table">
              <thead>
                <tr>
                  <th class="th-status">{t("matchStatus")}</th>
                  <th class="th-designation">{t("designation")}</th>
                  <th class="th-qty">{t("quantity")}</th>
                  <th class="th-lot">{t("noLot")}</th>
                  <th class="th-price">{t("ppa")} (DA)</th>
                  <th class="th-price">{t("puht")} (DA)</th>
                  <th class="th-exp">{t("exp")}</th>
                  <th class="th-tva">{t("tva")} (%)</th>
                  <th class="th-mg">{t("mg")} (%)</th>
                  <th class="th-total">{t("lineTotal")}</th>
                  <th class="th-actions"></th>
                </tr>
              </thead>
              <tbody>
                {#each items as item, idx}
                  <tr class="row-{item.overall_status}">
                    <!-- Status Badge -->
                    <td class="text-center">
                      <span class="status-indicator {item.overall_status}" title={item.validation_messages.join(" • ") || "OK"}></span>
                    </td>

                    <!-- Designation & Association -->
                    <td>
                      <div class="designation-cell" class:cell-ai-highlighted={aiHighlightedCells.has(`${idx}-desig`)}>
                        <span class="raw-ocr-label" title={t("designation")}>{item.raw_designation}</span>
                        {#if item.all_candidates.length > 0}
                          <select 
                            class="candidate-select" 
                            value={item.is_new_drug ? "NEW" : (item.drug_id?.toString() || "NEW")}
                            onchange={(e) => handleSelectDrugCandidate(idx, (e.target as HTMLSelectElement).value)}
                          >
                            <option value="NEW">➕ {t("newDrug")}</option>
                            {#each item.all_candidates as cand}
                              <option value={cand.id.toString()}>
                                💊 {cand.name} ({Math.round(cand.match_score * 100)}%)
                              </option>
                            {/each}
                          </select>
                        {:else}
                          <span class="badge-new">{t("newDrug")}</span>
                        {/if}

                        {#if !item.is_new_drug}
                          <label class="alias-checkbox">
                            <input type="checkbox" bind:checked={item.save_alias} />
                            <span>{t("saveAlias")}</span>
                          </label>
                        {/if}
                      </div>
                    </td>

                    <!-- Quantity -->
                    <td>
                      <input 
                        type="number" 
                        min="1" 
                        class="cell-input num-cell" 
                        class:cell-ai-highlighted={aiHighlightedCells.has(`${idx}-qty`)}
                        bind:value={item.quantity_packages} 
                        oninput={() => recalculateRow(idx)}
                      />
                    </td>

                    <!-- Batch -->
                    <td>
                      <input 
                        type="text" 
                        class="cell-input" 
                        class:cell-ai-highlighted={aiHighlightedCells.has(`${idx}-lot`)}
                        bind:value={item.batch_number} 
                        placeholder="LOT"
                      />
                    </td>

                    <!-- PPA (Retail) -->
                    <td>
                      <input 
                        type="number" 
                        step="0.5" 
                        min="0" 
                        class="cell-input num-cell" 
                        class:cell-ai-highlighted={aiHighlightedCells.has(`${idx}-ppa`)}
                        bind:value={item.ppa_da} 
                        oninput={() => onPriceOrCostChange(idx)}
                      />
                    </td>

                    <!-- Cost Price (PUHT) -->
                    <td>
                      <input 
                        type="number" 
                        step="0.5" 
                        min="0" 
                        class="cell-input num-cell" 
                        class:cell-ai-highlighted={aiHighlightedCells.has(`${idx}-puht`)}
                        bind:value={item.cost_price_da} 
                        oninput={() => onPriceOrCostChange(idx)}
                      />
                    </td>

                    <!-- Expiry -->
                    <td>
                      <input 
                        type="text" 
                        class="cell-input {item.expiry_date ? '' : 'cell-warn'}" 
                        class:cell-ai-highlighted={aiHighlightedCells.has(`${idx}-exp`)}
                        bind:value={item.expiry_date} 
                        onchange={() => recalculateRow(idx)}
                        placeholder="YYYY-MM-DD"
                      />
                    </td>

                    <!-- TVA -->
                    <td>
                      <input 
                        type="number" 
                        step="1" 
                        min="0" 
                        class="cell-input num-cell" 
                        class:cell-ai-highlighted={aiHighlightedCells.has(`${idx}-tva`)}
                        bind:value={item.tva} 
                        oninput={() => onPriceOrCostChange(idx)}
                      />
                    </td>

                    <!-- MG -->
                    <td>
                      <input 
                        type="number" 
                        step="0.5" 
                        class="cell-input num-cell" 
                        class:cell-ai-highlighted={aiHighlightedCells.has(`${idx}-mg`)}
                        bind:value={item.mg} 
                        oninput={() => onMarginChange(idx)}
                      />
                    </td>

                    <!-- Total -->
                    <td class="bold {item.math_status === 'mismatch' ? 'text-danger' : ''}" class:cell-ai-highlighted={aiHighlightedCells.has(`${idx}-total`)}>
                      {item.total_da.toFixed(2)}
                    </td>

                    <!-- Delete Row -->
                    <td>
                      <button class="btn-delete-row" onclick={() => removeRow(idx)} title={t("deleteRow")}>
                        <img src={trashIcon} alt="Delete" />
                      </button>
                    </td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>

          <!-- Bottom Action Buttons -->
          <div class="review-footer">
            <button class="btn-action btn-action-secondary" onclick={addNewRow}>
              + {t("addRow")}
            </button>
            <div class="footer-right">
              <button class="btn-action btn-action-danger" onclick={onClose}>
                {t("cancelImport")}
              </button>
              <button 
                class="btn-action btn-action-primary" 
                disabled={hasCriticalErrors || isSubmitting} 
                onclick={handleCommit}
              >
                {#if isSubmitting}
                  Enregistrement...
                {:else}
                  {t("commitToStock")}
                {/if}
              </button>
            </div>
          </div>
        </div>
      </div>
    {/if}
  </div>
</div>

{#if showApiKeyModal}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div class="api-key-overlay" onclick={() => showApiKeyModal = false} role="presentation">
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <div class="api-key-dialog" onclick={(e) => e.stopPropagation()} role="dialog" tabindex="-1">
      <div class="api-key-header">
        <h3>{t("aiApiKeyPromptTitle")}</h3>
        <button class="btn-close-sm" onclick={() => showApiKeyModal = false}>✕</button>
      </div>
      <p class="api-key-desc">{t("aiApiKeyPromptDesc")}</p>
      <div class="api-key-input-group">
        <label for="gemini-key-input">{t("aiKeyLabel")}</label>
        <input 
          id="gemini-key-input"
          type="password" 
          class="api-key-input" 
          placeholder={t("aiApiKeyPlaceholder")} 
          bind:value={geminiApiKeyInput}
          onkeydown={(e) => { if (e.key === 'Enter') handleSaveApiKeyAndRun(); }}
        />
      </div>
      <div class="api-key-footer">
        <button type="button" class="btn-action btn-action-danger" onclick={() => showApiKeyModal = false}>
          {t("cancelImport")}
        </button>
        <button type="button" class="btn-action btn-action-primary" onclick={handleSaveApiKeyAndRun}>
          {t("saveAndVerify")}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .invoice-import-modal {
    width: 98vw;
    max-width: 1720px;
    height: 94vh;
    max-height: 960px;
    display: flex;
    flex-direction: column;
    padding: 1.25rem 1.5rem;
    overflow: hidden;
    transition: width 0.2s ease, max-width 0.2s ease, height 0.2s ease;
  }

  .invoice-import-modal.is-compact {
    width: 90vw;
    max-width: 540px;
    height: auto;
    min-height: 380px;
    max-height: 85vh;
    padding: 1.5rem 1.75rem 1.75rem;
  }

  .import-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-bottom: 2px solid var(--color-border);
    padding-bottom: 0.75rem;
    margin-bottom: 1rem;
  }

  .title-group {
    display: flex;
    align-items: center;
    gap: 1rem;
  }

  .file-path-badge {
    background: #e1e4e8;
    color: #444;
    padding: 0.25rem 0.6rem;
    border-radius: 6px;
    font-size: 0.85rem;
    font-family: monospace;
  }

  .btn-close {
    background: transparent;
    border: none;
    cursor: pointer;
    padding: 0.25rem;
  }

  .btn-close img {
    width: 24px;
    height: 24px;
  }

  .upload-dropzone {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    border: 2px dashed var(--color-border);
    border-radius: var(--border-radius);
    background: #f8fafc;
    gap: 1rem;
    padding: 2.25rem 1.5rem;
    cursor: pointer;
    transition: all 0.2s ease-in-out;
  }

  .upload-dropzone:hover {
    border-color: var(--color-primary);
    background: #f0fdf4;
    box-shadow: 0 4px 14px var(--color-shadow);
  }

  .upload-dropzone.is-dragging-over {
    border-color: var(--color-primary);
    background: #dcfce7;
    border-style: solid;
    transform: scale(1.01);
    box-shadow: 0 0 24px rgba(0, 135, 90, 0.35);
  }

  .dropzone-pdf-img {
    width: 72px;
    height: 72px;
    object-fit: contain;
    transition: transform 0.2s ease-in-out;
  }

  .upload-dropzone:hover .dropzone-pdf-img,
  .upload-dropzone.is-dragging-over .dropzone-pdf-img {
    transform: scale(1.08);
  }

  .dropzone-title {
    font-size: 1.35rem;
    font-weight: 800;
    color: var(--color-text-dark);
    margin: 0;
    text-align: center;
  }

  .dropzone-desc {
    font-size: 1.05rem;
    color: #4b5563;
    margin: 0;
    max-width: 440px;
    text-align: center;
  }

  .btn-browse {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0.7rem 2rem;
    background-color: var(--color-primary);
    color: #ffffff;
    border: 2px solid var(--color-primary-hover);
    border-radius: var(--border-radius);
    font-size: 1.1rem;
    font-weight: 750;
    cursor: pointer;
    box-shadow: 0 4px 6px var(--color-shadow);
    transition: all 0.15s ease-in-out;
  }

  .btn-browse:hover {
    background-color: var(--color-primary-hover);
    transform: translateY(-2px);
    box-shadow: 0 6px 12px var(--color-shadow);
  }

  .btn-browse:active {
    transform: translateY(0);
  }

  .loading-container {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 1.5rem;
  }

  .spinner {
    width: 50px;
    height: 50px;
    border: 5px solid #e2e8f0;
    border-top-color: var(--color-primary);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  .split-container {
    display: flex;
    flex: 1;
    gap: 1.25rem;
    overflow: hidden;
  }

  .preview-panel {
    flex: 0 0 24%;
    min-width: 220px;
    max-width: 330px;
    display: flex;
    flex-direction: column;
    background: #2d3748;
    border-radius: 8px;
    overflow: hidden;
  }

  .preview-controls {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.5rem 1rem;
    background: #1a202c;
    color: #fff;
    font-size: 0.9rem;
  }

  .zoom-buttons button, .page-nav-buttons button {
    background: #4a5568;
    color: #fff;
    border: none;
    padding: 0.2rem 0.6rem;
    border-radius: 4px;
    cursor: pointer;
  }

  .preview-viewport {
    flex: 1;
    overflow: auto;
    padding: 1rem;
    display: flex;
    justify-content: center;
    align-items: flex-start;
    background: #374151;
  }

  .scanned-image {
    max-width: 100%;
    height: auto;
    object-fit: contain;
    box-shadow: 0 4px 12px rgba(0,0,0,0.5);
    background: #fff;
    border-radius: 4px;
    transition: transform 0.15s ease;
  }

  .review-panel {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .meta-row {
    display: flex;
    gap: 1rem;
    margin-bottom: 0.75rem;
  }

  .meta-field {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
  }

  .meta-field label {
    font-size: 0.85rem;
    font-weight: 700;
  }

  .input-pos-sm {
    padding: 0.4rem 0.6rem;
    border: 2px solid var(--color-border);
    border-radius: 6px;
    font-size: 0.95rem;
  }

  .financial-summary {
    display: flex;
    justify-content: space-around;
    padding: 0.6rem;
    border-radius: 6px;
    font-size: 0.95rem;
    margin-bottom: 0.75rem;
  }

  .summary-details {
    display: flex;
    justify-content: space-around;
    width: 100%;
    align-items: center;
  }

  .summary-ok {
    background: #e3fcef;
    border: 1px solid var(--color-primary);
    color: var(--color-primary-hover);
  }

  .summary-diff {
    background: #fff0b3;
    border: 1px solid #ffab00;
    color: #8f4d00;
  }

  .table-wrapper {
    flex: 1;
    overflow: auto;
    border: 1.5px solid var(--color-border);
    border-radius: 8px;
    background: #ffffff;
  }

  .review-table {
    width: 100%;
    min-width: 1060px;
    border-collapse: collapse;
    font-size: 0.9rem;
  }

  .review-table th, .review-table td {
    padding: 0.45rem 0.5rem;
    border-bottom: 1px solid var(--color-border);
    text-align: left;
    vertical-align: middle;
  }

  .th-status { width: 36px; min-width: 36px; text-align: center; }
  .th-designation { min-width: 240px; }
  .th-qty { width: 75px; min-width: 75px; }
  .th-lot { width: 110px; min-width: 110px; }
  .th-price { width: 95px; min-width: 95px; }
  .th-exp { width: 115px; min-width: 115px; }
  .th-tva { width: 70px; min-width: 70px; }
  .th-mg { width: 75px; min-width: 75px; }
  .th-total { width: 100px; min-width: 100px; text-align: right; }
  .th-actions { width: 45px; min-width: 45px; text-align: center; }

  .status-indicator {
    display: inline-block;
    width: 12px;
    height: 12px;
    border-radius: 50%;
  }

  .status-indicator.valid { background-color: var(--color-primary); }
  .status-indicator.warning { background-color: #f59e0b; }
  .status-indicator.error { background-color: var(--color-danger); }

  .designation-cell {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
  }

  .raw-ocr-label {
    font-weight: 700;
    color: var(--color-text-dark);
  }

  .candidate-select {
    font-size: 0.85rem;
    padding: 0.2rem;
    border-radius: 4px;
    border: 1px solid var(--color-border);
  }

  .alias-checkbox {
    font-size: 0.75rem;
    color: #4a5568;
    display: flex;
    align-items: center;
    gap: 0.3rem;
  }

  .cell-input {
    box-sizing: border-box;
    width: 100%;
    min-width: 0;
    padding: 0.35rem 0.45rem;
    border: 1.5px solid var(--color-border);
    border-radius: 5px;
    font-size: 0.92rem;
    font-weight: 600;
    color: var(--color-text-dark);
    background: #ffffff;
    outline: none;
    transition: border-color 0.15s ease, box-shadow 0.15s ease;
  }

  .cell-input:focus {
    border-color: var(--color-primary);
    box-shadow: 0 0 0 2px rgba(0, 135, 90, 0.2);
  }

  .cell-input::-webkit-outer-spin-button,
  .cell-input::-webkit-inner-spin-button {
    -webkit-appearance: none;
    margin: 0;
  }

  .cell-input[type="number"] {
    -moz-appearance: textfield;
    appearance: textfield;
  }

  .cell-input.cell-warn {
    border-color: #f59e0b;
    background: #fffbeb;
  }

  .num-cell {
    text-align: right;
    font-variant-numeric: tabular-nums;
  }

  .btn-delete-row {
    background: transparent;
    border: none;
    cursor: pointer;
  }

  .btn-delete-row img {
    width: 16px;
    height: 16px;
  }

  .review-footer {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding-top: 0.75rem;
    border-top: 1px solid var(--color-border);
    margin-top: 0.5rem;
  }

  .footer-right {
    display: flex;
    gap: 1rem;
  }

  .badge-new {
    background: #dbeafe;
    color: #1e40af;
    font-size: 0.75rem;
    padding: 0.1rem 0.4rem;
    border-radius: 4px;
    width: fit-content;
  }

  /* Gemini AI Styles */
  .meta-actions {
    display: flex;
    align-items: flex-end;
    gap: 0.6rem;
    margin-left: auto;
  }

  .btn-gemini-ai {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    background-color: var(--color-secondary);
    color: #ffffff;
    border: 2px solid var(--color-secondary-hover);
    padding: 0.5rem 1.15rem;
    border-radius: var(--border-radius);
    font-weight: 700;
    font-size: 0.95rem;
    cursor: pointer;
    box-shadow: 0 2px 4px var(--color-shadow);
    transition: all 0.15s ease-in-out;
  }

  .btn-gemini-ai:hover:not(:disabled) {
    background-color: var(--color-secondary-hover);
    transform: translateY(-1px);
    box-shadow: 0 4px 8px var(--color-shadow);
  }

  .btn-gemini-ai:disabled {
    opacity: 0.6;
    cursor: not-allowed;
    transform: none;
  }

  .ai-spinner {
    width: 16px;
    height: 16px;
    border: 2px solid rgba(255, 255, 255, 0.4);
    border-top-color: white;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  .ai-success-banner {
    background-color: #e3fcef;
    border: 2px solid var(--color-primary);
    color: var(--color-primary-hover);
    padding: 0.6rem 1rem;
    border-radius: var(--border-radius);
    font-size: 0.95rem;
    font-weight: 700;
    margin-bottom: 0.75rem;
    text-align: center;
  }

  .summary-details {
    display: flex;
    gap: 1.5rem;
    align-items: center;
  }

  .btn-sm-gemini-fix {
    background-color: var(--color-secondary);
    color: #ffffff;
    border: 1px solid var(--color-secondary-hover);
    padding: 0.35rem 0.85rem;
    border-radius: 6px;
    font-size: 0.85rem;
    font-weight: 700;
    cursor: pointer;
    transition: all 0.15s ease-in-out;
  }

  .btn-sm-gemini-fix:hover:not(:disabled) {
    background-color: var(--color-secondary-hover);
  }

  /* API Key Setup Modal Dialog */
  .api-key-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(9, 30, 66, 0.54);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .api-key-dialog {
    background: #ffffff;
    border: var(--border-width) solid var(--color-border);
    border-radius: var(--border-radius);
    width: 90vw;
    max-width: 480px;
    padding: 1.75rem;
    box-shadow: 0 8px 24px var(--color-shadow);
    display: flex;
    flex-direction: column;
    gap: 1.1rem;
  }

  .api-key-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .api-key-header h3 {
    margin: 0;
    font-size: 1.15rem;
    color: #1e293b;
    font-weight: 600;
  }

  .btn-close-sm {
    background: transparent;
    border: none;
    font-size: 1.1rem;
    cursor: pointer;
    color: #64748b;
    padding: 0.2rem;
  }

  .api-key-desc {
    font-size: 0.88rem;
    color: #475569;
    line-height: 1.4;
    margin: 0;
  }

  .api-key-input-group {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }

  .api-key-input-group label {
    font-size: 0.82rem;
    font-weight: 600;
    color: #334155;
  }

  .api-key-input {
    width: 100%;
    padding: 0.65rem 0.85rem;
    border: 1px solid #cbd5e1;
    border-radius: 6px;
    font-size: 0.9rem;
    box-sizing: border-box;
    font-family: monospace;
  }

  .api-key-input:focus {
    outline: none;
    border-color: var(--color-primary);
    box-shadow: 0 0 0 3px rgba(0, 135, 90, 0.2);
  }

  .api-key-footer {
    display: flex;
    justify-content: flex-end;
    gap: 0.75rem;
    margin-top: 0.5rem;
    border-top: 1px solid #e2e8f0;
    padding-top: 1rem;
  }

  /* Error Banner */
  .error-banner {
    background-color: #ffebe6;
    border: 1.5px solid var(--color-danger);
    color: var(--color-danger);
    padding: 0.65rem 1rem;
    border-radius: var(--border-radius);
    font-size: 0.92rem;
    font-weight: 600;
    margin-bottom: 1rem;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
    box-shadow: 0 2px 6px rgba(222, 53, 11, 0.08);
  }

  .error-banner-content {
    display: flex;
    align-items: center;
    gap: 0.65rem;
  }

  .error-badge {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 20px;
    height: 20px;
    background: var(--color-danger);
    color: #ffffff;
    border-radius: 50%;
    font-size: 0.8rem;
    font-weight: 800;
    flex-shrink: 0;
  }

  .btn-dismiss-error {
    background: transparent;
    border: none;
    color: var(--color-danger);
    font-weight: bold;
    font-size: 1.1rem;
    cursor: pointer;
    padding: 0 0.25rem;
    line-height: 1;
    opacity: 0.7;
    transition: opacity 0.15s;
    border-radius: 4px;
  }

  .btn-dismiss-error:hover {
    opacity: 1;
    background: rgba(222, 53, 11, 0.1);
  }

  /* Yellow highlight for cells affected by AI correction */
  .cell-ai-highlighted {
    background-color: #fef08a !important;
    border-color: #eab308 !important;
    color: #713f12 !important;
    box-shadow: 0 0 0 2px rgba(234, 179, 8, 0.4) !important;
    transition: background-color 0.4s ease-out, border-color 0.4s ease-out, box-shadow 0.4s ease-out !important;
  }
</style>

