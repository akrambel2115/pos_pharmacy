import { invoke } from "@tauri-apps/api/core";
import { formatDateFr } from "./utils";

export interface InvoiceItem {
  drug_name: string;
  quantity_items: number;
  unit_price_da?: number;
  line_total_da: number;
}

export interface InvoiceData {
  id: number;
  created_at: string;
  total_da: number;
  patient_name?: string | null;
  patient_birth_date?: string | null;
  prescribing_doctor_name?: string | null;
  treatment_period_days?: number | null;
  is_loan?: boolean;
  loan_status?: string | null;
  loan_settled_at?: string | null;
  items: InvoiceItem[];
}

export function generateThermalReceiptHtml(
  invoice: InvoiceData,
  pharmacyName: string,
  pharmacyAddress: string
): string {
  const itemsHtml = (invoice.items || [])
    .map((item) => {
      const unitPrice =
        item.unit_price_da !== undefined && item.unit_price_da !== null
          ? item.unit_price_da.toFixed(2)
          : (item.line_total_da / (item.quantity_items || 1)).toFixed(2);
      const total = item.line_total_da.toFixed(2);

      return `
        <tr>
          <td class="col-name">${escapeHtml(item.drug_name)}</td>
          <td class="col-qty text-center">${item.quantity_items}</td>
          <td class="col-price text-right">${unitPrice}</td>
          <td class="col-total text-right">${total}</td>
        </tr>
      `;
    })
    .join("");

  let loanBadge = "";
  if (invoice.is_loan) {
    if (invoice.loan_status === "settled") {
      const settledDate = invoice.loan_settled_at
        ? formatDateFr(invoice.loan_settled_at)
        : "";
      loanBadge = `
        <div class="loan-banner">
          *** AVANCE REGLEE ${settledDate ? `LE ${settledDate}` : ""} ***
        </div>
      `;
    } else {
      loanBadge = `
        <div class="loan-banner font-bold">
          *** AVANCE EN COURS (NON REGLEE) ***
        </div>
      `;
    }
  }

  return `
    <div class="receipt-root">
      <!-- Entête Pharmacie -->
      <div class="receipt-header">
        ${pharmacyName ? `<div class="pharmacy-name">${escapeHtml(pharmacyName)}</div>` : `<div class="pharmacy-name">PHARMACIE</div>`}
        ${pharmacyAddress ? `<div class="pharmacy-address">${escapeHtml(pharmacyAddress)}</div>` : ""}
      </div>

      <div class="divider"></div>

      <!-- Informations Facture / Ticket -->
      <div class="ticket-title">${invoice.is_loan ? "AVANCE / CREDIT" : "TICKET DE CAISSE"}</div>
      
      <div class="meta-row">
        <span>Facture N°:</span>
        <span class="font-bold">#${invoice.id}</span>
      </div>
      <div class="meta-row">
        <span>Date:</span>
        <span>${formatDateFr(invoice.created_at)}</span>
      </div>

      ${invoice.prescribing_doctor_name ? `
        <div class="meta-row">
          <span>Medecin:</span>
          <span>${escapeHtml(invoice.prescribing_doctor_name)}</span>
        </div>
      ` : ""}

      ${invoice.treatment_period_days ? `
        <div class="meta-row">
          <span>Duree traitement:</span>
          <span>${invoice.treatment_period_days} jours</span>
        </div>
      ` : ""}

      <div class="divider"></div>

      <!-- Articles -->
      <table class="receipt-table">
        <thead>
          <tr>
            <th class="col-name">Designation</th>
            <th class="col-qty text-center">Qte</th>
            <th class="col-price text-right">P.U</th>
            <th class="col-total text-right">Total</th>
          </tr>
        </thead>
        <tbody>
          ${itemsHtml}
        </tbody>
      </table>

      <div class="divider"></div>

      <!-- Total -->
      <div class="total-row">
        <span>TOTAL :</span>
        <span class="total-amount">${invoice.total_da.toFixed(2)} DA</span>
      </div>

      ${loanBadge}

      <div class="divider"></div>

      <!-- Pied de page -->
      <div class="receipt-footer">
        <div>Merci de votre visite !</div>
        <div class="footer-sub">Veuillez conserver ce ticket</div>
      </div>
    </div>
  `;
}

function escapeHtml(str: string): string {
  if (!str) return "";
  return str
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#039;");
}

export async function printThermalInvoice(
  invoice: any,
  pharmacyName?: string,
  pharmacyAddress?: string
): Promise<void> {
  let name = pharmacyName;
  let addr = pharmacyAddress;

  if (name === undefined || addr === undefined) {
    try {
      const settings = await invoke<any>("get_settings");
      if (name === undefined) name = settings?.pharmacy_name || "";
      if (addr === undefined) addr = settings?.pharmacy_address || "";
    } catch (err) {
      console.error("Failed to load pharmacy settings for print:", err);
      name = name || "";
      addr = addr || "";
    }
  }

  const receiptHtml = generateThermalReceiptHtml(invoice, name || "", addr || "");

  // Create or retrieve hidden iframe
  let iframe = document.getElementById("thermal-print-iframe") as HTMLIFrameElement;
  if (!iframe) {
    iframe = document.createElement("iframe");
    iframe.id = "thermal-print-iframe";
    iframe.style.position = "fixed";
    iframe.style.right = "0";
    iframe.style.bottom = "0";
    iframe.style.width = "0";
    iframe.style.height = "0";
    iframe.style.border = "none";
    document.body.appendChild(iframe);
  }

  const doc = iframe.contentWindow?.document;
  if (!doc) {
    console.error("Cannot access print iframe document");
    return;
  }

  doc.open();
  doc.write(`
    <!DOCTYPE html>
    <html lang="fr">
    <head>
      <meta charset="utf-8">
      <title>Ticket de Caisse #${invoice.id}</title>
      <style>
        @page {
          size: 80mm auto;
          margin: 0;
        }
        * {
          box-sizing: border-box;
          margin: 0;
          padding: 0;
        }
        body {
          font-family: 'Courier New', Courier, monospace;
          font-size: 11px;
          line-height: 1.3;
          color: #000;
          background: #fff;
          width: 76mm;
          padding: 4mm 2mm;
          margin: 0 auto;
        }
        .receipt-root {
          width: 100%;
        }
        .receipt-header {
          text-align: center;
          margin-bottom: 4px;
        }
        .pharmacy-name {
          font-size: 15px;
          font-weight: 900;
          letter-spacing: 0.5px;
        }
        .pharmacy-address {
          font-size: 10px;
          margin-top: 2px;
          word-break: break-word;
        }
        .divider {
          border-top: 1px dashed #000;
          margin: 6px 0;
        }
        .ticket-title {
          font-size: 13px;
          font-weight: 900;
          text-align: center;
          margin: 4px 0 6px 0;
        }
        .meta-row {
          display: flex;
          justify-content: space-between;
          font-size: 10.5px;
          margin: 1.5px 0;
        }
        .font-bold {
          font-weight: bold;
        }
        .receipt-table {
          width: 100%;
          border-collapse: collapse;
          margin: 4px 0;
          font-size: 10.5px;
        }
        .receipt-table th {
          border-bottom: 1px dashed #000;
          padding-bottom: 3px;
          font-weight: bold;
          text-align: left;
        }
        .receipt-table td {
          padding: 2.5px 0;
          vertical-align: top;
          word-break: break-word;
        }
        .col-name {
          width: 48%;
        }
        .col-qty {
          width: 12%;
        }
        .col-price {
          width: 20%;
        }
        .col-total {
          width: 20%;
        }
        .text-center {
          text-align: center;
        }
        .text-right {
          text-align: right;
        }
        .total-row {
          display: flex;
          justify-content: space-between;
          font-size: 14px;
          font-weight: 900;
          margin: 4px 0;
        }
        .total-amount {
          font-size: 15px;
        }
        .loan-banner {
          text-align: center;
          border: 1px dashed #000;
          padding: 4px;
          margin: 6px 0;
          font-size: 10.5px;
        }
        .receipt-footer {
          text-align: center;
          font-size: 10.5px;
          margin-top: 6px;
        }
        .footer-sub {
          font-size: 9.5px;
          margin-top: 2px;
        }
      </style>
    </head>
    <body>
      ${receiptHtml}
    </body>
    </html>
  `);
  doc.close();

  // Trigger print after rendering
  setTimeout(() => {
    try {
      iframe.contentWindow?.focus();
      iframe.contentWindow?.print();
    } catch (e) {
      console.error("Print error:", e);
    }
  }, 250);
}
