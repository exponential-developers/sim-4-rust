import jsonData from "../Data/data.json" with { type: "json" };
import { convertTime, formatNumber, isMainTheory, logToExp } from "../Utils/helpers";
import { qs, qsa, ce, event, removeAllChilds, downloadString, getTableHeaders, tau } from "../Utils/DOMhelpers";
import { requestBoughtVars } from "./simWorker";
import type { FinishedPart } from "./partial";

// Settings
const generateTotalPurchaseList = qs<HTMLInputElement>(".total_purchase_list");

// Outputs
const table = qs(".simTable");
const theadRow = qs<HTMLTableRowElement>(".simTable > thead > tr");
const tbody = qs(".simTable > tbody");

const varBuyDialog = qs<HTMLDialogElement>(".boughtVars");
const varBuyTable = qs<HTMLTableSectionElement>(".boughtVarsOtp");
const varBuyListCloseBtn = qs<HTMLButtonElement>(".boughtVarsCloseBtn");

const downloadIcon = '<svg xmlns="http://www.w3.org" width="24" height="24" viewBox="0 0 24 24" ' +
        'fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"' +
        'class="feather feather-download">\n' +
    '<path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"></path>' +
    '<polyline points="7 10 12 15 17 10"></polyline><line x1="12" y1="15" x2="12" y2="3"></line>\n' +
    '</svg>'

const eyeIcon = '<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" ' + 
        'viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" ' + 
        'stroke-linecap="round" stroke-linejoin="round" class="feather feather-eye">\n' + 
    ' <path d="M1 12s4-7 11-7 11 7 11 7-4 7-11 7S1 12 1 12z"></path>\n' + 
    ' <circle cx="12" cy="12" r="3"></circle>\n' + 
    '</svg>';

/** Results whose purchases make up the "total purchase list" (setting) */
let totalResults: simResult[] = [];

// Utils
function clearTable() {
    removeAllChilds(tbody);
}
function setTableClass(cl: ("big" | "small")) {
    table.classList.remove("big", "small");
    table.classList.add(cl);
}
function setTableMode(mode: string) {
    table.setAttribute("simMode", mode);
}
function getTableMode(): string {
    return table.getAttribute("simMode") ?? "";
}
function setTableHeaders(...headers: string[]) {
    removeAllChilds(theadRow);
    headers.forEach(header => {
        const cell = ce("th");
        cell.innerHTML = header;
        theadRow.appendChild(cell);
    })
}

/**
 * Adds a cell to a HTML table
 * @param row The HTML table to append the cell to
 * @param content The HTML content of the cell
 * @param rowspan The rowspan of the cell (default 1)
 */
function addTableCell(row: HTMLTableRowElement, content: string, rowspan = 1) {
    const cell = ce("td");
    cell.innerHTML = content;
    if (rowspan > 1) cell.setAttribute("rowspan", String(rowspan));
    row.appendChild(cell);
}

/**
 * Fills an HTML row with empty cells
 * @param row The HTML row to fill
 * @param count The number of empty cells to add
 */
function fillTableRow(row: HTMLTableRowElement, count: number) {
    for (let i = 0; i < count; i++) addTableCell(row, "");
}

function makeVarBuyCsv(arr: varBuy[]): string {
    let csvTotal = "variable,level,cost,timeStamp\n";
    for(const item of arr) {
        csvTotal += `"${item.var_name}",${item.level},${logToExp(item.cost, 2)}${getCurrencySymbol(item.symbol)},${convertTime(item.timestamp)}\n`;
    }
    return csvTotal;
}

/**
 * The purchases of a result. Results from the sim leave them out (a Steps run can have millions):
 * they are simulated again from the result's `replay`, which gives the same purchases. Results
 * saved by older versions (the "All" mode cache) still carry them.
 */
async function boughtVarsOf(res: simResult): Promise<varBuy[]> {
    if (res.bought_vars.length > 0 || !res.replay) return res.bought_vars;
    const response = JSON.parse(await requestBoughtVars(JSON.stringify(res.replay))) as
        { response_type: "success"; data: varBuy[] } | { response_type: "failure"; data: string };
    if (response.response_type === "failure") throw new Error(response.data);
    return response.data;
}

/** The purchases of several results, one after the other with a "---" line between them */
async function totalBoughtVars(results: simResult[]): Promise<varBuy[]> {
    const all: varBuy[] = [];
    for (const res of results) {
        all.push(...await boughtVarsOf(res));
        all.push({ var_name: "---", level: 0, cost: 0, timestamp: 0 });
    }
    return all;
}

/** "Var Buys" buttons of a result, or of the total purchase list ("total") */
function addVarBuyCell(row: HTMLTableRowElement, source: simResult | "total", rowspan = 1) {
    let load: () => Promise<varBuy[]>;
    if (source === "total") {
        const results = totalResults;
        load = () => totalBoughtVars(results);
    } else {
        if (generateTotalPurchaseList.checked) totalResults.push(source);
        load = () => boughtVarsOf(source);
    }
    const count = source === "total" ? undefined : source.bought_vars_count ?? source.bought_vars.length;

    const cell = ce("td");
    cell.classList.add("varBuyCell");

    // Real buttons, so the icons can be reached with Tab and pressed with Enter or Space
    const viewBtn = ce<HTMLButtonElement>("button");
    viewBtn.type = "button";
    viewBtn.title = count === undefined ? "View variable purchases" : `View variable purchases (${count})`;
    viewBtn.setAttribute("aria-label", "View variable purchases");
    viewBtn.innerHTML = eyeIcon;
    viewBtn.onclick = () => void openVarModal(load);

    const downloadBtn = ce<HTMLButtonElement>("button");
    downloadBtn.type = "button";
    downloadBtn.title = "Download variable purchases as CSV";
    downloadBtn.setAttribute("aria-label", "Download variable purchases as CSV");
    downloadBtn.innerHTML = downloadIcon;
    downloadBtn.onclick = () => {
        void load().then(
            (buys) => downloadString(makeVarBuyCsv(buys), "buys.csv"),
            (err: unknown) => qs(".output").textContent = err instanceof Error ? err.message : String(err),
        );
    }

    cell.appendChild(viewBtn);
    cell.appendChild(downloadBtn);
    
    if (rowspan > 1) cell.setAttribute("rowspan", String(rowspan));
    row.appendChild(cell);
}

// Var buy utils

function getCurrencySymbol(value: string | undefined): string {
    if (value === undefined || value === "rho") return "\u03C1";
    if (value === "lambda") return "\u03BB";
    if (value === "delta") return "\u03B4"
    if (/_/.test(value)) {
      value = value.replace(/{}/g, "");
      const split = value.split("_");
      return `${getCurrencySymbol(split[0])}<sub>${split[1]}</sub>`;
    }
    return value;
  }

/** Highlights MF reset cells */
function highlightResetCells() {
  const cells = qsa<HTMLTableCellElement>('.boughtVars tr td:nth-child(1)');
  cells.forEach(cell => {
    if (cell.innerText.toLowerCase().includes('reset at')) {
      cell.classList.add('highlighted');
    }
  });
}

/** A single message across the var buy list (while loading, or an error) */
function varModalMessage(text: string) {
  removeAllChilds(varBuyTable);
  const row = ce<HTMLTableRowElement>("tr");
  const cell = ce<HTMLTableCellElement>("td");
  cell.colSpan = 4;
  cell.textContent = text;
  row.appendChild(cell);
  varBuyTable.appendChild(row);
}

/** Opens the var buy list and fills it once the purchases are loaded */
async function openVarModal(load: () => Promise<varBuy[]>) {
  document.body.style.overflow = "hidden";
  varBuyDialog.showModal();
  // A running simulation finishes first: the purchases are simulated again by the same worker
  varModalMessage(qs(".simulate").textContent === "Simulate" ? "Loading..." : "Loading after the running simulation...");
  let arr: varBuy[];
  try {
    arr = await load();
  } catch (err) {
    varModalMessage(err instanceof Error ? err.message : String(err));
    return;
  }
  removeAllChilds(varBuyTable);
  for (const varBuy of arr) {
    const row = ce<HTMLTableRowElement>("tr");
    addTableCell(row, varBuy.var_name);
    addTableCell(row, varBuy.level.toString());
    addTableCell(row, `${logToExp(varBuy.cost, 2)}<span style="margin-left:.1em">${getCurrencySymbol(varBuy.symbol)}</span>`);
    addTableCell(row, convertTime(varBuy.timestamp));
    varBuyTable.appendChild(row);
  }
  highlightResetCells();
}

event(varBuyListCloseBtn, "pointerdown", () => varBuyDialog.close());

event(varBuyDialog, "close", () => document.body.style.overflow = "auto");

// Response writers

function writeSingleSimResponse(response: SingleSimResponse) {
    const res = response.result;
    const row = ce<HTMLTableRowElement>("tr");
    addTableCell(row, res.theory);
    addTableCell(row, res.sigma.toString());
    addTableCell(row, logToExp(res.last_pub, 2));
    addTableCell(row, logToExp(res.pub_rho, 2));
    addTableCell(row, logToExp(res.delta_tau, 2));
    addTableCell(row, formatNumber(res.pub_multi));
    addTableCell(row, res.strat);
    addTableCell(row, res.tau_h == 0 ? "0" : formatNumber(res.tau_h));
    addTableCell(row, convertTime(res.time));
    addVarBuyCell(row, res);
    tbody.append(row);
}

function writeChainSimResponse(response: ChainSimResponse) {
    response.results.forEach(res => writeSingleSimResponse({
        responseType: "single",
        result: res
    }));
    const labelRow = ce<HTMLTableRowElement>("tr");
    const resRow = ce<HTMLTableRowElement>("tr");

    fillTableRow(labelRow, 4);
    fillTableRow(resRow, 4);

    addTableCell(labelRow, "ΔTau Total");
    addTableCell(resRow, logToExp(response.delta_tau, 2));

    fillTableRow(labelRow, 2);
    fillTableRow(resRow, 2);

    addTableCell(labelRow, `Average ${tau}/h`);
    addTableCell(resRow, formatNumber(response.average_rate, 5));

    addTableCell(labelRow, `Total Time`);
    addTableCell(resRow, convertTime(response.total_time));

    fillTableRow(labelRow, 1);
    if(generateTotalPurchaseList.checked) {
        addVarBuyCell(resRow, "total");
    }
    else {
        fillTableRow(resRow, 1);
    }

    tbody.append(labelRow);
    tbody.append(resRow);
}

function writeStepSimResponse(response: StepSimResponse) {
    response.results.forEach(res => writeSingleSimResponse({
        responseType: "single",
        result: res
    }));
    if (generateTotalPurchaseList.checked) {
        const resRow = ce<HTMLTableRowElement>("tr");
        fillTableRow(resRow, 8);
        addTableCell(resRow, "Total");
        addVarBuyCell(resRow, "total");
        tbody.append(resRow);
    }
}

/** The cells of an active or idle result in All mode */
function completeSimAllLine(row: HTMLTableRowElement, res: simResult) {
    addTableCell(row, res.tau_h == 0 ? "0" : formatNumber(res.tau_h));
    addTableCell(row, formatNumber(res.pub_multi));
    addTableCell(row, res.strat);
    addTableCell(row, convertTime(res.time));
    addTableCell(row, logToExp(res.delta_tau, 2));
    addTableCell(row, logToExp(res.pub_rho, 2));
    addVarBuyCell(row, res);
}

/** The rows of one theory in All mode */
function writeSimAllRows(res: simAllResult, stratType: SettingsSimAllStratsMode) {
    if (stratType == "all") {
        const rowActive = ce<HTMLTableRowElement>("tr");
        const rowPassive = ce<HTMLTableRowElement>("tr");

        addTableCell(rowActive, res.theory, 2);
        addTableCell(rowActive, logToExp(res.last_pub, 2), 2);
        addTableCell(rowActive, formatNumber(res.ratio, 4), 2);

        completeSimAllLine(rowActive, res.active);
        completeSimAllLine(rowPassive, res.idle);

        tbody.appendChild(rowActive);
        tbody.appendChild(rowPassive);
    }
    else {
        const uniqueRes = stratType == "active" ? res.active : res.idle;
        const row = ce<HTMLTableRowElement>("tr");

        addTableCell(row, res.theory);
        addTableCell(row, logToExp(res.last_pub, 2));
        completeSimAllLine(row, uniqueRes);

        tbody.appendChild(row);
    }
}

function writeSimAllResponse(response: SimAllResponse) {

    let sets: simAllResult[][] = [[], [], []];
    response.results.forEach((res) => {
        if (isMainTheory(res.theory)) {
            sets[0].push(res);
        }
        else {
            if (response.completed_cts === "end" && res.last_pub * jsonData.theories[res.theory].tau_factor >= 600) {
                sets[2].push(res);
            }
            else sets[1].push(res);
        }
    });
    sets = sets.filter(set => set.length > 0);

    sets.forEach((set, i) => {
        set.forEach(res => writeSimAllRows(res, response.strat_type));

        if (i < sets.length - 1) {
            const bufferRow1 = ce<HTMLTableRowElement>("tr");
            const bufferRow2 = ce<HTMLTableRowElement>("tr");

            bufferRow1.style.display = "none";
            addTableCell(bufferRow2, "---");

            tbody.appendChild(bufferRow1);
            tbody.appendChild(bufferRow2);
        }
    })
}

// Results shown while a simulation runs

/** The table the results of a query type go in, or null for those not shown while running */
function liveTableMode(queryType: SimQuery["queryType"]): "step" | "chain" | "all" | null {
    switch (queryType) {
        case "step": case "step_chain": case "comparison": return "step";
        case "chain": case "amount": case "time": return "chain";
        case "all": return "all";
        default: return null;
    }
}

/**
 * Adds finished results to the table while a simulation runs. The first call sets the table up;
 * the final response (`writeSimResponse`) then replaces these rows.
 */
export function writeLiveResults(queryType: SimQuery["queryType"], parts: FinishedPart[], first: boolean) {
    const mode = liveTableMode(queryType);
    if (mode === null || parts.length === 0) return;
    if (first) {
        downloadPubTableButton.style.display = "none";
        setTableClass(mode == "all" ? "big" : "small");
        setTableMode(mode);
        clearTable();
        totalResults = [];
        const all = parts[0].kind === "all_result" ? parts[0] : null;
        if (all) setTableHeaders(...getTableHeaders(all.strat_type === "all" ? "all" : "all_one", "html", all.sigma), 'Var Buys');
        else setTableHeaders(...getTableHeaders("single", "html"), 'Var Buys');
    }
    for (const part of parts) {
        if (part.kind === "result" || part.kind === "pub") writeSingleSimResponse({ responseType: "single", result: part.result });
        else if (part.kind === "all_result") writeSimAllRows(part.result, part.strat_type);
    }
}

// Pub table mode: the result is a file to download (same format as sim-3)
const downloadPubTableButton = qs<HTMLButtonElement>(".downloadPubTable");
let lastPubTable: PubTableResponse | null = null;

/** JSON pub table: for each rho (log10) from cap down, the next pub target and the time to reach cap */
function makePubTableJson(response: PubTableResponse): string {
    const table: Record<string, Record<string, string | number>> = {};
    let current = response.cap;
    for (const [time, next] of response.pub_table) {
        table[current.toFixed(4)] = { next: next.toFixed(4), time };
        current -= response.step;
    }
    return JSON.stringify(table, null, 2);
}

event(downloadPubTableButton, "pointerdown", () => {
    if (lastPubTable) downloadString(makePubTableJson(lastPubTable), "table.json");
});

export function writeSimResponse(response: SimResponse) {
    if (response.responseType === "pub_table") {
        lastPubTable = response;
        downloadPubTableButton.style.display = "";
        const total = response.pub_table[response.pub_table.length - 1][0];
        // main.ts clears the status line before writing results
        qs(".output").textContent = `Pub table ready (${response.pub_table.length} rows, ${convertTime(total)} from start to cap). Use "Download Pub Table".`;
        return;
    }
    downloadPubTableButton.style.display = "none";
    const mode = response.responseType;

    if (mode != "single" || getTableMode() != mode) {
        setTableClass(mode == "all" ? "big" : "small");
        setTableMode(mode);
        clearTable();
    }
    if (mode === "all") setTableHeaders(...getTableHeaders(response.strat_type === "all" ? "all" : "all_one", "html", response.sigma), 'Var Buys')
    else setTableHeaders(...getTableHeaders("single", "html"), 'Var Buys');

    totalResults = [];

    switch (mode) {
        case "single": writeSingleSimResponse(response); break;
        case "chain": writeChainSimResponse(response); break;
        case "step": writeStepSimResponse(response); break;
        case "all": writeSimAllResponse(response); break;
    }
}
