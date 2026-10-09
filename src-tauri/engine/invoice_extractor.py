"""
Pharmacy POS - Wholesaler Invoice Extraction Engine
Designed for Algerian pharmacy invoices (Millennium Medic, Pharma Spot, Setif Medic, Biopharm, CPA, etc.)
Extracts the 8 core medicine fields:
  1. Designation
  2. Quantity
  3. No Lot
  4. PPA
  5. PUHT
  6. Exp
  7. TVA
  8. MG
"""

import sys
import os
import json
import re
import io
import base64
import urllib.request
import urllib.error
from typing import List, Dict, Any, Optional

# On Windows, ensure PyInstaller unpack directory, executable directory,
# and current directory are in the DLL search path for onnxruntime/pdfium
if sys.platform == "win32":
    if hasattr(sys, "_MEIPASS"):
        try:
            os.add_dll_directory(sys._MEIPASS)
        except Exception:
            pass
    if getattr(sys, "frozen", False):
        try:
            exe_dir = os.path.dirname(sys.executable)
            os.add_dll_directory(exe_dir)
        except Exception:
            pass


def clean_num(val_str: Optional[str]) -> Optional[float]:
    if not val_str:
        return None
    s = val_str.replace(" ", "").replace(",", ".").replace("DA", "").replace("da", "").replace("%", "")
    s = re.sub(r"^[^\d\-+]+", "", s)
    m = re.search(r"[-+]?\d*\.?\d+", s)
    if m:
        try:
            return float(m.group(0))
        except:
            return None
    return None


def parse_expiry(text: Optional[str]) -> str:
    if not text:
        return ""
    cleaned = text.strip().replace("o", "0").replace("O", "0")
    m = re.search(r"(\d{1,2})[/\-\.](\d{2,4})", cleaned)
    if m:
        m_part, y_part = m.groups()
        if len(y_part) == 2:
            year = 2000 + int(y_part)
        else:
            year = int(y_part)
        month = int(m_part)
        if 1 <= month <= 12 and 2024 <= year <= 2045:
            return f"{year:04d}-{month:02d}-01"
    return ""


def extract_invoice(file_path: str, output_dir: Optional[str] = None) -> Dict[str, Any]:
    file_path = os.path.abspath(file_path)
    if not os.path.exists(file_path):
        return {"success": False, "error": f"Fichier introuvable: {file_path}"}

    try:
        import pypdfium2 as pdfium
        from rapidocr_onnxruntime import RapidOCR
        import numpy as np
    except ImportError as e:
        return {
            "success": False,
            "error": f"Le moteur OCR local requiert les bibliothèques C++ Windows. Veuillez installer le package Microsoft Visual C++ 2015-2022 Redistributable (x64) ou activer l'option 'Appliquer directement l'IA' dans les Paramètres. Détail: {str(e)}"
        }

    engine = RapidOCR()
    rendered_pages = []
    all_extracted_items = []
    supplier_name = ""
    invoice_number = ""
    invoice_date = ""
    detected_grand_total = 0.0

    is_pdf = file_path.lower().endswith(".pdf")
    pages_to_process = []

    if is_pdf:
        try:
            pdf = pdfium.PdfDocument(file_path)
            num_pages = len(pdf)
            for page_idx in range(min(num_pages, 8)):
                page = pdf[page_idx]
                bitmap = page.render(scale=2.5)
                pil_image = bitmap.to_pil()

                # Generate base64 data URL for fast and reliable UI preview
                buf = io.BytesIO()
                pil_image.save(buf, format="PNG", optimize=True)
                b64_str = base64.b64encode(buf.getvalue()).decode("ascii")
                data_url = f"data:image/png;base64,{b64_str}"
                rendered_pages.append(data_url)

                np_image = np.array(pil_image)
                pages_to_process.append((page_idx, np_image, pil_image.size))
        except Exception as e:
            return {"success": False, "error": f"Échec de lecture du PDF: {str(e)}"}
    else:
        from PIL import Image
        pil_image = Image.open(file_path)
        buf = io.BytesIO()
        pil_image.save(buf, format="PNG", optimize=True)
        b64_str = base64.b64encode(buf.getvalue()).decode("ascii")
        rendered_pages.append(f"data:image/png;base64,{b64_str}")
        np_image = np.array(pil_image)
        pages_to_process.append((0, np_image, pil_image.size))

    for page_idx, np_image, (w, h) in pages_to_process:
        ocr_result, _ = engine(np_image)
        if not ocr_result:
            continue

        boxes = []
        for item in ocr_result:
            box, text, score = item[0], item[1].strip(), float(item[2])
            if not text:
                continue
            x_min = min(p[0] for p in box)
            y_min = min(p[1] for p in box)
            x_max = max(p[0] for p in box)
            y_max = max(p[1] for p in box)
            boxes.append({
                "text": text,
                "score": score,
                "x_min": x_min, "y_min": y_min,
                "x_max": x_max, "y_max": y_max,
                "cx": (x_min + x_max) / 2,
                "cy": (y_min + y_max) / 2,
            })

        # 1. Locate Table Header (DESIGNATION / DÉSIGNATION)
        header_box = None
        for b in sorted(boxes, key=lambda x: x["cy"]):
            txt = b["text"].upper()
            if (re.search(r"D[EÉ]SIGNAT", txt) or "DESIGNATION" in txt) and b["cy"] < h * 0.75:
                header_box = b
                break

        if not header_box:
            if page_idx > 0 and all_extracted_items:
                header_cy = h * 0.10
                header_y_max = h * 0.12
            else:
                header_cy = h * 0.32
                header_y_max = h * 0.34
        else:
            header_cy = header_box["cy"]
            header_y_max = header_box["y_max"]

        # 2. Extract column headers
        cols = []
        if header_box:
            header_line_boxes = [b for b in boxes if abs(b["cy"] - header_cy) <= 25]
            header_line_boxes.sort(key=lambda x: x["cx"])
            for b in header_line_boxes:
                t = b["text"].upper()
                c_name = None
                if re.search(r"D[EÉ]SIGNAT", t): c_name = "designation"
                elif any(k in t for k in ["QTE", "QTÉ", "QT", "QUANT"]): c_name = "quantity"
                elif any(k in t for k in ["LOT", "N'LOT", "N°LOT", "NOLOT"]): c_name = "lot"
                elif "PPA" in t: c_name = "ppa"
                elif any(k in t for k in ["P.U", "PU", "PRIX U", "PRIX"]): c_name = "puht"
                elif any(k in t for k in ["EXP", "PEREM"]): c_name = "exp"
                elif "TVA" in t: c_name = "tva"
                elif any(k in t for k in ["MG", "MARGE", "MGE"]): c_name = "mg"
                elif any(k in t for k in ["MONT", "TOTAL", "THT", "MT HT"]): c_name = "total"
                elif any(k in t for k in ["SHP", "DCI", "REMISE"]): c_name = t.lower()
                if c_name:
                    cols.append((b["cx"], c_name, b["text"]))

        # 3. Locate Table Footer
        footer_box = None
        for b in sorted(boxes, key=lambda x: x["cy"]):
            txt = b["text"].upper()
            if b["cy"] > header_cy + 40:
                if any(k in txt for k in ["TOTAL HT", "TOTAL BRUT", "TOTALPPA", "TOTAL PPA", "NET A PAYER", "NET HT", "TOTAL TTC", "TOTALTTG", "LIGNES", "NB:"]):
                    footer_box = b
                    break

        footer_y_min = footer_box["y_min"] if footer_box else h * 0.96

        # 4. Extract Supplier & Invoice meta (Above table header)
        for b in boxes:
            if b["cy"] < header_y_max:
                txt = b["text"].upper()
                if any(sup in txt for sup in ["MILLENNIUM", "PHARMA SPOT", "SETIF MEDIC", "BIOPHARM", "CPA", "HYDRAPHARM", "PROPHARM", "SOPHAL"]):
                    if not supplier_name:
                        supplier_name = b["text"]
                if ("FACTURE" in txt or "F7298" in txt or "26/F" in txt or "26/FA" in txt) and not invoice_number:
                    m_inv = re.search(r"(\d{2}/[A-Z0-9\-]+|F\d+/\d+|\d{5,10})", b["text"], re.I)
                    if m_inv:
                        invoice_number = m_inv.group(1)
                m_date = re.search(r"(\d{1,2}[/\-\.]\d{1,2}[/\-\.]\d{4})", b["text"])
                if m_date and not invoice_date:
                    invoice_date = m_date.group(1)

        # 5. Extract Totals (Below table footer)
        is_invoice_tva_zero = False
        detected_discount = 0.0
        detected_total_ht = 0.0
        detected_tva = 0.0
        detected_timbre = 0.0
        for b in boxes:
            txt = b["text"].upper()
            if "TVA" in txt and any(z in txt for z in ["0.00", "0,00", ":0"]):
                is_invoice_tva_zero = True
            if b["cy"] >= footer_y_min:
                if any(k in txt for k in ["NET A PAYER", "TOTAL TTC", "TOTALTTG", "NET A PAYE"]):
                    val = clean_num(b["text"])
                    if val and val > 100:
                        detected_grand_total = max(detected_grand_total, val)
                elif any(k in txt for k in ["TOTAL HT", "TOTAL BRUT", "MONTANT HT"]):
                    val = clean_num(b["text"])
                    if val and val > 100:
                        detected_total_ht = max(detected_total_ht, val)
                elif any(k in txt for k in ["RISTOURNE", "REMISE", "RIST"]):
                    val = clean_num(b["text"])
                    if val and val > 0:
                        detected_discount = max(detected_discount, val)
                elif "TIMBRE" in txt:
                    val = clean_num(b["text"])
                    if val is not None and val >= 0:
                        detected_timbre = val
                elif "TVA" in txt and not any(k in txt for k in ["TAUX", "%"]):
                    val = clean_num(b["text"])
                    if val and val > 0:
                        detected_tva = val

        # 6. Table Rows Extraction
        table_boxes = [b for b in boxes if header_y_max + 8 <= b["cy"] < footer_y_min - 8]
        table_boxes.sort(key=lambda x: x["cy"])

        raw_rows = []
        cur_row = []
        cur_y = None
        for b in table_boxes:
            if cur_y is None or abs(b["cy"] - cur_y) <= 15:
                cur_row.append(b)
                cur_y = sum(x["cy"] for x in cur_row) / len(cur_row)
            else:
                cur_row.sort(key=lambda x: x["x_min"])
                raw_rows.append(cur_row)
                cur_row = [b]
                cur_y = b["cy"]
        if cur_row:
            cur_row.sort(key=lambda x: x["x_min"])
            raw_rows.append(cur_row)

        for row in raw_rows:
            row_text = " ".join(b["text"] for b in row)
            if any(k in row_text.upper() for k in ["CREE PAR", "IMPRIME PAR", "ARRETEE LA PRESENTE", "COLIS", "SERVICE EXPEDITION", "LIGNES :"]):
                continue

            row_cells = {c[1]: [] for c in cols}
            for b in row:
                if cols:
                    best_col = min(cols, key=lambda c: abs(c[0] - b["cx"]))
                    row_cells[best_col[1]].append(b["text"])

            all_numbers = []
            exp_candidate = ""
            for b in row:
                e = parse_expiry(b["text"])
                if e and not exp_candidate:
                    exp_candidate = e
                v = clean_num(b["text"])
                if v is not None:
                    all_numbers.append(v)

            # Check if row is continuation
            if not all_numbers and all_extracted_items:
                continuation = " ".join(b["text"] for b in row).strip()
                if continuation and not any(k in continuation.upper() for k in ["PAGE", "DATE", "LE:"]):
                    all_extracted_items[-1]["raw_designation"] += " " + continuation
                continue

            if "designation" in row_cells and row_cells["designation"]:
                desig = " ".join(row_cells["designation"]).strip()
            else:
                desig = " ".join(b["text"] for b in row if b["cx"] < w * 0.40).strip()
            desig = re.sub(r"^\d+[\s\.\-]+", "", desig).strip()

            lot_str = " ".join(row_cells.get("lot", [])).strip()
            exp_in_lot = parse_expiry(lot_str)
            if exp_in_lot and not exp_candidate:
                exp_candidate = exp_in_lot
                lot_str = re.sub(r"\d{1,2}/\d{2,4}", "", lot_str).strip()

            if not lot_str or len(lot_str) < 3:
                for b in row:
                    if re.match(r"^[A-Z0-9\-_]{4,14}$", b["text"]) and not any(k in b["text"].upper() for k in ["COMP", "PELL", "SOL", "INJ", "CP", "B/", "T/"]):
                        lot_str = b["text"]
                        break
            if not lot_str:
                lot_str = "LOT-01"
            lot_str = re.sub(r"^[^\w]+", "", lot_str)

            puht_val = clean_num(" ".join(row_cells.get("puht", []))) or 0.0
            ppa_val = clean_num(" ".join(row_cells.get("ppa", []))) or 0.0
            tot_val = clean_num(" ".join(row_cells.get("total", []))) or 0.0
            qty_val = clean_num(" ".join(row_cells.get("quantity", [])))
            mg_val = clean_num(" ".join(row_cells.get("mg", []))) or 0.0
            tva_raw = clean_num(" ".join(row_cells.get("tva", [])))
            if tva_raw is not None:
                # In Algerian pharmacy invoices, 20%, 25%, 33% is the margin (MG), not TVA
                if tva_raw in (20.0, 25.0, 33.0) and mg_val == 0.0:
                    mg_val = tva_raw
                    tva_val = 0.0 if is_invoice_tva_zero else 9.0
                else:
                    tva_val = tva_raw
            else:
                tva_val = 0.0 if is_invoice_tva_zero else 9.0

            if tot_val == 0.0 or puht_val == 0.0:
                if len(all_numbers) >= 2:
                    if tot_val == 0.0:
                        tot_val = all_numbers[-1]
                    if puht_val == 0.0:
                        candidates = [n for n in all_numbers[:-1] if n > 0]
                        if candidates:
                            puht_val = candidates[0]

            # Cross-verify Qté from Total and PUHT
            if tot_val > 0 and puht_val > 0:
                calc_qty = tot_val / puht_val
                if abs(calc_qty - round(calc_qty)) < 0.05 and 1 <= round(calc_qty) <= 2000:
                    qty = int(round(calc_qty))
                else:
                    qty = int(qty_val) if qty_val and qty_val >= 1 else 1
            elif qty_val and qty_val >= 1:
                qty = int(qty_val)
                if tot_val == 0.0 and puht_val > 0:
                    tot_val = round(puht_val * qty, 2)
            else:
                qty = 1

            if tot_val == 0.0 and puht_val > 0:
                tot_val = round(puht_val * qty, 2)

            if ppa_val == 0.0 and puht_val > 0:
                ppa_val = round(puht_val * 1.25, 2)

            if puht_val > 0 and ppa_val > 0 and puht_val > ppa_val:
                puht_val, ppa_val = ppa_val, puht_val

            if mg_val == 0.0 and puht_val > 0 and ppa_val > puht_val:
                base_cost = puht_val * (1 + tva_val / 100) if tva_val > 0 else puht_val
                mg_val = round(((ppa_val - base_cost) / base_cost) * 100, 1)
                if mg_val < 0:
                    mg_val = round(((ppa_val - puht_val) / puht_val) * 100, 1)

            if len(desig) >= 3:
                all_extracted_items.append({
                    "raw_designation": desig,
                    "quantity": qty,
                    "batch_number": lot_str,
                    "ppa_da": ppa_val,
                    "cost_price_da": puht_val,
                    "expiry_date": exp_candidate,
                    "tva": tva_val,
                    "mg": mg_val,
                    "total_da": tot_val,
                    "math_verified": abs(tot_val - round(qty * puht_val, 2)) <= 0.5,
                    "page_index": page_idx
                })

    items_sum = round(sum(it["total_da"] for it in all_extracted_items), 2)
    if detected_total_ht > 0 and abs(items_sum - detected_total_ht) <= 1.0:
        detected_grand_total = detected_total_ht
    elif detected_grand_total == 0.0:
        detected_grand_total = items_sum
    elif detected_total_ht > 0 and abs(items_sum - detected_grand_total) > 1.0 and abs(items_sum - detected_total_ht) <= 5.0:
        detected_grand_total = detected_total_ht

    return {
        "success": True,
        "supplier": supplier_name or "Grossiste Pharmacie",
        "invoice_number": invoice_number or "AUTO-INV",
        "invoice_date": invoice_date or "",
        "total_ht": round(detected_total_ht, 2),
        "discount": round(detected_discount, 2),
        "total_tva": round(detected_tva, 2),
        "timbre": round(detected_timbre, 2),
        "grand_total": round(detected_grand_total, 2),
        "pages_rendered": rendered_pages,
        "items": all_extracted_items,
    }


def extract_invoice_gemini(file_path: str, api_key: str, output_dir: Optional[str] = None) -> Dict[str, Any]:
    file_path = os.path.abspath(file_path)
    if not os.path.exists(file_path):
        return {"success": False, "error": f"Fichier introuvable: {file_path}"}

    rendered_pages = []
    image_parts = []

    is_pdf = file_path.lower().endswith(".pdf")
    if is_pdf:
        try:
            pdf = pdfium.PdfDocument(file_path)
            num_pages = len(pdf)
            for page_idx in range(min(num_pages, 8)):
                page = pdf[page_idx]
                bitmap = page.render(scale=2.0)
                pil_image = bitmap.to_pil()
                if pil_image.mode != "RGB":
                    pil_image = pil_image.convert("RGB")

                buf = io.BytesIO()
                pil_image.save(buf, format="JPEG", quality=88)
                b64_bytes = buf.getvalue()
                b64_str = base64.b64encode(b64_bytes).decode("ascii")
                data_url = f"data:image/jpeg;base64,{b64_str}"
                rendered_pages.append(data_url)

                image_parts.append({
                    "inline_data": {
                        "mime_type": "image/jpeg",
                        "data": b64_str
                    }
                })
        except Exception as e:
            return {"success": False, "error": f"Échec de lecture du PDF: {str(e)}"}
    else:
        try:
            from PIL import Image
            pil_image = Image.open(file_path)
            if pil_image.mode != "RGB":
                pil_image = pil_image.convert("RGB")
            buf = io.BytesIO()
            pil_image.save(buf, format="JPEG", quality=88)
            b64_bytes = buf.getvalue()
            b64_str = base64.b64encode(b64_bytes).decode("ascii")
            rendered_pages.append(f"data:image/jpeg;base64,{b64_str}")
            image_parts.append({
                "inline_data": {
                    "mime_type": "image/jpeg",
                    "data": b64_str
                }
            })
        except Exception as e:
            return {"success": False, "error": f"Échec d'ouverture de l'image: {str(e)}"}

    # Detect if whole invoice has 0.00 TVA from OCR boxes if available
    is_invoice_tva_zero = False

    prompt_text = (
        "Tu es un expert en facturation pharmaceutique en Algérie (grossistes répartiteurs: "
        "Millennium Medic, Pharma Spot, Setif Medic, Biopharm, CPA, etc.).\n"
        "Analyse attentivement cette image de facture d'achat de médicaments et extrait rigoureusement "
        "les métadonnées et toutes les lignes d'articles de la facture.\n\n"
        "Format de sortie JSON obligatoire et strict:\n"
        "{\n"
        '  "supplier": "Nom du grossiste",\n'
        '  "invoice_number": "N° Facture ou N° BL",\n'
        '  "invoice_date": "YYYY-MM-DD",\n'
        '  "total_brut": 0.0,\n'
        '  "discount": 0.0,\n'
        '  "total_tva": 0.0,\n'
        '  "timbre": 0.0,\n'
        '  "grand_total": 0.0,\n'
        '  "items": [\n'
        "    {\n"
        '      "raw_designation": "Nom complet du médicament avec dosage et forme (ex: DOLIPRANE 1000MG CPR)",\n'
        '      "quantity": 10,\n'
        '      "batch_number": "Numéro de Lot (ex: 23H091)",\n'
        '      "expiry_date": "YYYY-MM-DD",\n'
        '      "ppa_da": 250.0,\n'
        '      "cost_price_da": 180.0,\n'
        '      "tva": 0.0,\n'
        '      "mg": 20.0,\n'
        '      "total_da": 1800.0\n'
        "    }\n"
        "  ]\n"
        "}\n\n"
        "Règles impératives de lecture et validation:\n"
        "1. Extraire TOUTES les lignes de médicaments du tableau, sans en omettre aucune.\n"
        "2. raw_designation : nom complet avec dosage et forme.\n"
        "3. quantity : nombre d'unités ou boîtes facturées (QTE).\n"
        "4. batch_number : numéro de lot (NoLot / Lot).\n"
        "5. expiry_date : date d'expiration exacte au format YYYY-MM-DD. Si la facture n'indique que le mois et l'année (ex: 11/27 ou 11/2027), utiliser le premier jour du mois (ex: 2027-11-01). Ne pas confondre avec le lot.\n"
        "6. ppa_da : Prix Public Algérien (PPA / P.Vente), toujours supérieur au prix d'achat PUHT.\n"
        "7. cost_price_da : Prix Unitaire Hors Taxe (PUHT / P.U.Ht / P.Achat).\n"
        "8. tva : Taux de TVA (généralement 0.0, 9.0 ou 19.0). Si la facture indique TVA 0.00 ou exonéré dans le total, tva = 0.0. Ne pas mettre 9.0 par défaut si la facture est à 0%.\n"
        "9. mg : Marge bénéficiaire (MG / Mge / Marge, ex: 20.0, 25.0, 33.0). Ne pas confondre la marge avec la TVA. Si la colonne MG est absente ou 0, calculer la marge : ((ppa_da - cost_price_da * (1 + tva/100)) / (cost_price_da * (1 + tva/100))) * 100.\n"
        "10. total_da = quantity * cost_price_da.\n"
        "11. total_brut : Montant total brut HT des articles avant remise/ristourne (TOTAL HT / TOTAL BRUT).\n"
        "12. discount : Montant de la remise globale ou ristourne commerciale au bas de la facture (RISTOURNE / REMISE / RIST), sinon 0.0.\n"
        "13. total_tva : Montant total de la TVA de la facture (MONTANT TVA / TOTAL TVA), sinon 0.0.\n"
        "14. timbre : Montant du droit de timbre fiscal (TIMBRE), sinon 0.0.\n"
        "15. grand_total : Montant Net à Payer (NET A PAYER / TOTAL TTC / NET HT après déduction de la ristourne). Formule: total_brut - discount + total_tva + timbre.\n"
        "16. Renvoie UNIQUEMENT le JSON valide sans texte additionnel."
    )

    request_payload = {
        "contents": [
            {
                "parts": [
                    {"text": prompt_text},
                    *image_parts
                ]
            }
        ],
        "generationConfig": {
            "response_mime_type": "application/json",
            "temperature": 0.1
        }
    }

    req_data = json.dumps(request_payload).encode("utf-8")
    
    # Try available Flash models in order of speed, capability and stability
    models = [
        "gemini-3.8-flash",
        "gemini-3.7-flash",
        "gemini-3.5-flash",
        "gemini-flash-latest",
        "gemini-2.5-pro",
        "gemini-2.5-flash",
    ]
    last_error = ""

    for model_name in models:
        url = f"https://generativelanguage.googleapis.com/v1beta/models/{model_name}:generateContent?key={api_key}"
        req = urllib.request.Request(
            url,
            data=req_data,
            headers={"Content-Type": "application/json"},
            method="POST"
        )

        try:
            with urllib.request.urlopen(req, timeout=60) as resp:
                resp_data = resp.read().decode("utf-8")
                resp_json = json.loads(resp_data)
                
                # Extract text content from candidates
                candidates = resp_json.get("candidates", [])
                if not candidates:
                    return {"success": False, "error": "Aucune donnée n'a pu être extraite par l'analyse IA."}

                content_parts = candidates[0].get("content", {}).get("parts", [])
                if not content_parts:
                    return {"success": False, "error": "Réponse vide retournée par le service IA."}

                model_text = content_parts[0].get("text", "").strip()
                # Remove possible markdown fences if returned
                model_text = re.sub(r"^```(json)?\s*", "", model_text)
                model_text = re.sub(r"\s*```$", "", model_text)

                parsed_result = json.loads(model_text)

                # Normalize parsed_result if the model returned an array of items directly
                if isinstance(parsed_result, list):
                    parsed_result = {"items": parsed_result}
                elif not isinstance(parsed_result, dict):
                    parsed_result = {}

                raw_items = []
                for key_cand in ["items", "medicaments", "articles", "lignes", "data"]:
                    val = parsed_result.get(key_cand)
                    if isinstance(val, list):
                        raw_items = val
                        break

                # Post-process items
                clean_items = []
                for it in raw_items:
                    if not isinstance(it, dict):
                        continue

                    desig = str(it.get("raw_designation", "")).strip()
                    if not desig:
                        continue
                    
                    qty = int(clean_num(str(it.get("quantity", 1))) or 1)
                    lot = str(it.get("batch_number", "")).strip() or "LOT-AUTO"
                    raw_exp = str(it.get("expiry_date", "")).strip()
                    exp = parse_expiry(raw_exp) or raw_exp
                    if not re.match(r"^\d{4}-\d{2}-\d{2}$", exp):
                        exp = parse_expiry(exp)

                    puht = clean_num(str(it.get("cost_price_da", 0))) or 0.0
                    ppa = clean_num(str(it.get("ppa_da", 0))) or 0.0

                    if puht > 0 and ppa > 0 and puht > ppa:
                        puht, ppa = ppa, puht

                    tva_raw = it.get("tva")
                    tva_clean = clean_num(str(tva_raw)) if tva_raw is not None else None
                    tva = tva_clean if tva_clean is not None else 0.0

                    mg_raw = it.get("mg")
                    mg_clean = clean_num(str(mg_raw)) if mg_raw is not None else None
                    mg = mg_clean if mg_clean is not None else 0.0

                    if mg == 0.0 and puht > 0 and ppa > puht:
                        base = puht * (1 + tva / 100) if tva > 0 else puht
                        mg = round(((ppa - base) / base) * 100, 1)
                        if mg < 0:
                            mg = round(((ppa - puht) / puht) * 100, 1)

                    total = clean_num(str(it.get("total_da", round(qty * puht, 2)))) or round(qty * puht, 2)

                    clean_items.append({
                        "raw_designation": desig,
                        "quantity": qty,
                        "batch_number": lot,
                        "ppa_da": ppa,
                        "cost_price_da": puht,
                        "expiry_date": exp,
                        "tva": tva,
                        "mg": mg,
                        "total_da": total,
                        "math_verified": abs(total - round(qty * puht, 2)) <= 0.5,
                        "page_index": 0
                    })

                total_brut = clean_num(str(parsed_result.get("total_brut", 0))) or 0.0
                discount = clean_num(str(parsed_result.get("discount", 0))) or 0.0
                total_tva = clean_num(str(parsed_result.get("total_tva", 0))) or 0.0
                timbre = clean_num(str(parsed_result.get("timbre", 0))) or 0.0
                grand_total = clean_num(str(parsed_result.get("grand_total", 0))) or 0.0
                items_sum = round(sum(i["total_da"] for i in clean_items), 2)

                if grand_total == 0.0 and clean_items:
                    grand_total = round(items_sum - discount + total_tva + timbre, 2)

                return {
                    "success": True,
                    "supplier": parsed_result.get("supplier") or "Grossiste Pharmacie",
                    "invoice_number": parsed_result.get("invoice_number") or "AUTO-INV",
                    "invoice_date": parsed_result.get("invoice_date") or "",
                    "total_ht": round(total_brut, 2),
                    "discount": round(discount, 2),
                    "total_tva": round(total_tva, 2),
                    "timbre": round(timbre, 2),
                    "grand_total": round(grand_total, 2),
                    "pages_rendered": rendered_pages,
                    "items": clean_items,
                    "ai_verified": True
                }

        except urllib.error.HTTPError as e:
            err_details = ""
            try:
                raw_err = e.read().decode("utf-8", errors="replace")
                parsed_err = json.loads(raw_err)
                err_details = parsed_err.get("error", {}).get("message", "")
            except Exception:
                pass

            if e.code in (400, 403):
                last_error = "Clé API invalide ou non autorisée. Veuillez vérifier la clé dans les Paramètres."
                break
            elif e.code == 429:
                last_error = "Quota de requêtes dépassé. Veuillez patienter quelques instants avant de réessayer."
                continue
            elif e.code == 503:
                last_error = "Le service d'analyse IA est momentanément surchargé. Veuillez réessayer dans quelques instants."
                import time
                time.sleep(1.5)
                continue
            elif e.code == 404:
                last_error = "Le service d'analyse IA est momentanément indisponible."
                continue
            else:
                last_error = f"Erreur de communication avec le service IA (code {e.code}): {err_details or 'service indisponible'}."
                continue
        except urllib.error.URLError:
            last_error = "Impossible de joindre le service IA. Veuillez vérifier votre connexion Internet."
            break
        except Exception as e:
            last_error = f"Erreur lors de l'analyse intelligente : {str(e)}"
            continue

    return {"success": False, "error": last_error or "Échec de l'analyse avec le service IA."}


def test_gemini_key(api_key: str) -> Dict[str, Any]:
    api_key = api_key.strip()
    if not api_key:
        return {"success": False, "error": "Veuillez saisir une clé API."}

    test_models = [
        "gemini-3.8-flash",
        "gemini-3.7-flash",
        "gemini-3.5-flash",
        "gemini-flash-latest",
        "gemini-2.5-pro",
        "gemini-2.5-flash",
    ]
    last_err = ""
    for model_name in test_models:
        url = f"https://generativelanguage.googleapis.com/v1beta/models/{model_name}:generateContent?key={api_key}"
        payload = {
            "contents": [{"parts": [{"text": "OK"}]}],
            "generationConfig": {"maxOutputTokens": 5}
        }
        req = urllib.request.Request(
            url,
            data=json.dumps(payload).encode("utf-8"),
            headers={"Content-Type": "application/json"},
            method="POST"
        )
        try:
            with urllib.request.urlopen(req, timeout=15) as resp:
                return {"success": True, "message": "Connexion réussie ! La clé API est valide et opérationnelle."}
        except urllib.error.HTTPError as e:
            if e.code in (400, 403):
                return {"success": False, "error": "Clé API invalide ou non autorisée. Veuillez vérifier la clé saisie."}
            elif e.code == 429:
                last_err = "Quota de requêtes dépassé. Veuillez patienter un instant."
                continue
            elif e.code == 503:
                last_err = "Le service IA est temporairement saturé (erreur 503). Veuillez réessayer dans quelques instants."
                continue
            elif e.code == 404:
                last_err = "Le service d'analyse IA est temporairement indisponible."
                continue
            else:
                last_err = f"Erreur de service IA (code {e.code})."
        except urllib.error.URLError:
            return {"success": False, "error": "Connexion Internet requise pour vérifier la clé."}
        except Exception as e:
            last_err = f"Erreur de connexion : {str(e)}"

    return {"success": False, "error": last_err or "Impossible de vérifier la clé API."}


def ai_match_medicines(items_to_match: list, api_key: str) -> Dict[str, Any]:
    api_key = api_key.strip()
    if not api_key:
        return {"success": False, "error": "Clé API manquante."}

    if not items_to_match:
        return {"success": True, "decisions": []}

    prompt_text = (
        "Tu es un pharmacien expert en Algérie.\n"
        "Pour chaque ligne de médicament extraite d'une facture d'achat ('raw_designation'), examine attentivement "
        "la liste des médicaments candidats trouvés dans le catalogue de la pharmacie ('candidates').\n\n"
        "RÈGLES PHARMACEUTIQUES ABSOLUES:\n"
        "1. DOSAGE STRICTEMENT IDENTIQUE: Le dosage doit correspondre exactement au même dosage actif.\n"
        "   - Exemple crucial: 'DIAGLINIDE 0.5MG' ne doit JAMAIS être associé à 'DIAGLINIDE 1MG'. "
        "Si le dosage diffère, c'est obligatoirement un NOUVEAU médicament (is_new_drug: true, matched_drug_id: null).\n"
        "   - Équivalences autorisées: 1G = 1000MG, 500MG = 0.5G, 250MG = 0.25G.\n"
        "2. FORME PHARMACEUTIQUE: Comprimé (CP/COMP/CPR), Gélule (GEL), Sirop/Solution (BUV/FL/SIR), Injectable (INJ/AMP), etc. "
        "Si les formes sont incompatibles, marque comme nouveau médicament.\n"
        "3. DÉCISION:\n"
        "   - Si un candidat correspond exactement (même principe actif, même dosage, même forme): "
        "is_new_drug: false, matched_drug_id: <id_du_candidat_choisi>.\n"
        "   - Si AUCUN candidat ne correspond (dosage différent, forme différente ou produit absent): "
        "is_new_drug: true, matched_drug_id: null.\n\n"
        "Format de réponse JSON obligatoire (liste d'objets):\n"
        "[\n"
        "  {\n"
        '    "index": 0,\n'
        '    "is_new_drug": true,\n'
        '    "matched_drug_id": null,\n'
        '    "reason": "Explication brève (ex: Dosage différent 0.5mg vs 1mg)"\n'
        "  }\n"
        "]\n\n"
        f"Articles et candidats:\n{json.dumps(items_to_match, ensure_ascii=False)}"
    )

    request_payload = {
        "contents": [
            {
                "parts": [
                    {"text": prompt_text}
                ]
            }
        ],
        "generationConfig": {
            "response_mime_type": "application/json",
            "temperature": 0.1
        }
    }
    req_data = json.dumps(request_payload).encode("utf-8")

    models = [
        "gemini-3.8-flash",
        "gemini-3.7-flash",
        "gemini-3.5-flash",
        "gemini-flash-latest",
        "gemini-2.5-pro",
        "gemini-2.5-flash",
    ]
    last_error = ""

    for model_name in models:
        url = f"https://generativelanguage.googleapis.com/v1beta/models/{model_name}:generateContent?key={api_key}"
        req = urllib.request.Request(
            url,
            data=req_data,
            headers={"Content-Type": "application/json"},
            method="POST"
        )
        try:
            with urllib.request.urlopen(req, timeout=30) as resp:
                resp_data = resp.read().decode("utf-8")
                resp_json = json.loads(resp_data)
                candidates = resp_json.get("candidates", [])
                if not candidates:
                    return {"success": False, "error": "Aucune décision retournée par le service IA."}
                content_parts = candidates[0].get("content", {}).get("parts", [])
                if not content_parts:
                    return {"success": False, "error": "Réponse vide du service IA."}
                model_text = content_parts[0].get("text", "").strip()
                model_text = re.sub(r"^```(json)?\s*", "", model_text)
                model_text = re.sub(r"\s*```$", "", model_text)
                parsed = json.loads(model_text)
                if isinstance(parsed, dict) and "decisions" in parsed:
                    parsed = parsed["decisions"]
                elif isinstance(parsed, dict) and "items" in parsed:
                    parsed = parsed["items"]
                elif not isinstance(parsed, list):
                    parsed = []
                return {"success": True, "decisions": parsed}
        except urllib.error.HTTPError as e:
            if e.code in (400, 403):
                last_error = "Clé API invalide ou non autorisée."
                break
            elif e.code == 429:
                last_error = "Quota de requêtes dépassé."
                continue
            elif e.code in (404, 503):
                last_error = "Le service d'analyse IA est momentanément indisponible."
                continue
            else:
                last_error = f"Erreur de communication avec le service IA (code {e.code})."
                continue
        except urllib.error.URLError:
            last_error = "Impossible de joindre le service IA. Vérifiez votre connexion Internet."
            break
        except Exception as e:
            last_error = f"Erreur lors de l'analyse : {str(e)}"
            continue

    return {"success": False, "error": last_error or "Échec de l'association IA des médicaments."}


def main():
    if "--test-key" in sys.argv:
        t_idx = sys.argv.index("--test-key")
        test_key = sys.argv[t_idx + 1] if t_idx + 1 < len(sys.argv) else ""
        result = test_gemini_key(test_key)
        sys.stdout.buffer.write(json.dumps(result, ensure_ascii=False).encode("utf-8"))
        return

    if "--match-drugs" in sys.argv:
        t_idx = sys.argv.index("--match-drugs")
        payload_arg = sys.argv[t_idx + 1] if t_idx + 1 < len(sys.argv) else ""
        gemini_key = ""
        if "--gemini-key" in sys.argv:
            k_idx = sys.argv.index("--gemini-key")
            if k_idx + 1 < len(sys.argv):
                gemini_key = sys.argv[k_idx + 1]

        try:
            if os.path.exists(payload_arg):
                with open(payload_arg, "r", encoding="utf-8") as f:
                    items_payload = json.load(f)
            else:
                items_payload = json.loads(payload_arg)
        except Exception as e:
            sys.stdout.buffer.write(json.dumps({"success": False, "error": f"Payload invalide: {str(e)}"}).encode("utf-8"))
            return

        result = ai_match_medicines(items_payload, gemini_key)
        sys.stdout.buffer.write(json.dumps(result, ensure_ascii=False).encode("utf-8"))
        return

    if len(sys.argv) < 2:
        print(json.dumps({
            "success": False,
            "error": "Usage: python invoice_extractor.py <path_to_invoice_pdf_or_image> [cache_dir] [--gemini-key KEY] [--test-key KEY] [--match-drugs JSON_OR_FILE]"
        }))
        sys.exit(1)

    file_path = sys.argv[1]
    gemini_key = None
    if "--gemini-key" in sys.argv:
        try:
            k_idx = sys.argv.index("--gemini-key")
            if k_idx + 1 < len(sys.argv):
                gemini_key = sys.argv[k_idx + 1]
        except:
            pass

    if gemini_key:
        result = extract_invoice_gemini(file_path, gemini_key)
    else:
        result = extract_invoice(file_path)

    sys.stdout.buffer.write(json.dumps(result, ensure_ascii=False).encode("utf-8"))


if __name__ == "__main__":
    main()
