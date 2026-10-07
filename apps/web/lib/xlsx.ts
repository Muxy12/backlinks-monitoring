// Minimal, dependency-light .xlsx → CSV converter, client-side. An .xlsx file is
// a zip of XML; we unzip it (fflate) and read the first worksheet + the shared
// strings table, then emit CSV text that the existing CSV import path handles
// (same flexible header matching). Only the common case is supported — text and
// number cells of the first sheet — which is all a backlink list needs. No eval,
// so it is fine under the strict CSP.

import { unzipSync, strFromU8 } from "fflate";

// "B12" → 1 (0-based column index). Defaults to 0 on a malformed ref.
function colIndex(ref: string): number {
  const letters = /^([A-Z]+)/.exec(ref)?.[1];
  if (!letters) return 0;
  let n = 0;
  for (const ch of letters) n = n * 26 + (ch.charCodeAt(0) - 64);
  return n - 1;
}

function csvCell(s: string): string {
  return /[",\n\r]/.test(s) ? `"${s.replace(/"/g, '""')}"` : s;
}

// Concatenate the text of every <t> descendant (handles rich-text runs).
function allText(el: Element): string {
  const ts = el.getElementsByTagName("t");
  let out = "";
  for (let i = 0; i < ts.length; i++) out += ts[i].textContent ?? "";
  return out;
}

/** Convert the first worksheet of an .xlsx file to CSV text. Throws on a file
 *  that doesn't look like an .xlsx (so the caller can show a clear error). */
export function xlsxToCsv(buf: ArrayBuffer): string {
  const files = unzipSync(new Uint8Array(buf));
  const parser = new DOMParser();

  // Shared strings table (optional).
  const shared: string[] = [];
  const ssBytes = files["xl/sharedStrings.xml"];
  if (ssBytes) {
    const doc = parser.parseFromString(strFromU8(ssBytes), "application/xml");
    const sis = doc.getElementsByTagName("si");
    for (let i = 0; i < sis.length; i++) shared.push(allText(sis[i]));
  }

  // First worksheet (prefer sheet1.xml, else the first worksheet XML present).
  const names = Object.keys(files);
  const sheetKey =
    names.find((k) => k === "xl/worksheets/sheet1.xml") ??
    names.find((k) => /^xl\/worksheets\/[^/]+\.xml$/.test(k));
  if (!sheetKey) throw new Error("No worksheet found in the Excel file.");

  const doc = parser.parseFromString(strFromU8(files[sheetKey]), "application/xml");
  const rowEls = doc.getElementsByTagName("row");
  const rows: string[][] = [];

  for (let r = 0; r < rowEls.length; r++) {
    const cells: string[] = [];
    const cEls = rowEls[r].getElementsByTagName("c");
    for (let c = 0; c < cEls.length; c++) {
      const cell = cEls[c];
      const idx = colIndex(cell.getAttribute("r") ?? "");
      const type = cell.getAttribute("t");
      let val = "";
      if (type === "s") {
        const v = cell.getElementsByTagName("v")[0]?.textContent;
        val = v != null ? (shared[Number(v)] ?? "") : "";
      } else if (type === "inlineStr") {
        val = allText(cell);
      } else {
        val = cell.getElementsByTagName("v")[0]?.textContent ?? "";
      }
      cells[idx] = val;
    }
    for (let i = 0; i < cells.length; i++) if (cells[i] == null) cells[i] = "";
    rows.push(cells);
  }

  return rows.map((row) => row.map(csvCell).join(",")).join("\n");
}
