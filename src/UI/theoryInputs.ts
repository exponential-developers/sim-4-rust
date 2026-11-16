/**
 * Theory-specific inputs (sim-3's "specificInputs"): extra inputs a theory defines in data.json,
 * such as MF's reset depth or NLI's lifetime α. Single-theory modes show the selected theory's
 * inputs next to its other inputs; "All" mode has a dialog with every theory's inputs.
 *
 * Values are kept per theory in localStorage, so both places show the same values and they are
 * remembered between visits.
 */
import data from "../Data/data.json" with { type: "json" };
import { ce, event, qs, removeAllChilds } from "../Utils/DOMhelpers";
import { parseLog10String } from "../Utils/helpers";

const STORAGE_KEY = "theoryInputs";

const theoryData = data.theories as TheoryDataStructure;

/** Saved values by theory, then by input id */
type StoredInputs = Partial<Record<string, TheoryInputs>>;

function loadStored(): StoredInputs {
  try {
    const stored = JSON.parse(localStorage.getItem(STORAGE_KEY) ?? "{}") as StoredInputs;
    return typeof stored === "object" && stored !== null ? stored : {};
  } catch {
    return {};
  }
}

function store(theory: string, id: string, value: string) {
  const stored = loadStored();
  stored[theory] = { ...stored[theory], [id]: value };
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(stored));
  } catch {
    // Storage full or unavailable: the value is only lost on reload
  }
}

/**
 * MF's reset depth used to be a sim setting (saved with the settings as "mfResetDepth"): keep a
 * value set there.
 */
function migrateOldSettings() {
  const stored = loadStored();
  if (stored.MF?.depth !== undefined) return;
  try {
    const old = (JSON.parse(localStorage.getItem("simState") ?? "{}") as { settings?: { mfResetDepth?: string } })
      .settings?.mfResetDepth;
    if (old !== undefined && old !== "0") store("MF", "depth", old);
  } catch {
    // Nothing to keep
  }
}
migrateOldSettings();

/** Inputs a theory defines, in data.json order */
export function theoryInputDefs(theory: string): [string, SpecificInput][] {
  return Object.entries(theoryData[theory]?.specificInputs ?? {});
}

function defaultValue(input: SpecificInput): string {
  if (input.type === "slider") return String(input.default ?? input.validation.min);
  return "";
}

/** A labelled input for one theory input; `prefix` goes before the label (e.g. "MF - ") */
function createWidget(theory: string, id: string, input: SpecificInput, prefix: string): HTMLDivElement {
  const div = ce<HTMLDivElement>("div");
  div.classList.add("theoryInput");
  div.dataset.theory = theory;
  div.dataset.input = id;

  const label = ce<HTMLLabelElement>("label");
  label.classList.add("description");
  label.textContent = prefix + input.label;
  if (input.hint) label.title = input.hint;
  div.appendChild(label);

  const value = loadStored()[theory]?.[id] ?? defaultValue(input);
  const elemId = `theoryInput-${theory}-${id}-${prefix ? "all" : "single"}`;
  label.htmlFor = elemId;

  if (input.type === "slider") {
    const row = ce<HTMLDivElement>("div");
    row.classList.add("theoryInputSlider");
    const slider = ce<HTMLInputElement>("input");
    slider.type = "range";
    slider.id = elemId;
    slider.min = String(input.validation.min);
    slider.max = String(input.validation.max);
    slider.step = String(input.step);
    slider.value = value;
    if (input.hint) slider.title = input.hint;
    const output = ce<HTMLSpanElement>("span");
    output.classList.add("sliderOtp");
    output.textContent = slider.value;
    event(slider, "input", () => {
      output.textContent = slider.value;
      store(theory, id, slider.value);
    });
    row.append(slider, output);
    div.appendChild(row);
  } else {
    const textbox = ce<HTMLInputElement>("input");
    textbox.type = "text";
    textbox.id = elemId;
    textbox.classList.add("theoryInputText");
    textbox.placeholder = input.placeholder || "<value>";
    textbox.spellcheck = false;
    textbox.value = value;
    if (input.hint) textbox.title = input.hint;
    event(textbox, "input", () => store(theory, id, textbox.value.trim()));
    div.appendChild(textbox);
  }
  return div;
}

/** Shows the selected theory's inputs (single-theory modes) */
export function renderTheoryInputs(container: HTMLElement, theory: string) {
  removeAllChilds(container);
  for (const [id, input] of theoryInputDefs(theory)) {
    container.appendChild(createWidget(theory, id, input, ""));
  }
}

/** Shows every theory's inputs ("All" mode dialog) */
export function renderAllTheoryInputs(container: HTMLElement, theories: string[]) {
  removeAllChilds(container);
  for (const theory of theories) {
    for (const [id, input] of theoryInputDefs(theory)) {
      container.appendChild(createWidget(theory, id, input, `${theory} - `));
    }
  }
}

/** sim-3's `parseExponentialValue` (for "exp" inputs) */
function parseExp(value: string): number {
  if (/^e?\d+(\.\d+)?$/.test(value)) return parseFloat(value.replace(/^e/, ""));
  if (/^\d+(\.\d+)?e\d+$/.test(value)) return parseLog10String(value);
  throw new Error("Value must be in formats <number>, <exxxx> or <xexxxx>");
}

/**
 * Checks a value like sim-3 does (`parseSpecificInputTextbox`) and returns it, or undefined
 * when the input is left empty.
 */
function validate(theory: string, input: SpecificInput, raw: string): string | undefined {
  const value = raw.trim();
  if (value === "") return undefined;
  const fail = (message: string): never => {
    throw new Error(`Bad input for ${theory} - ${input.label}: ${message}`);
  };
  const { validation } = input;
  const bounds = (parse: (s: string) => number, parsed: number) => {
    const min = typeof validation.min === "string" ? parse(validation.min) : validation.min;
    const max = typeof validation.max === "string" ? parse(validation.max) : validation.max;
    if (min !== undefined && max !== undefined && (parsed < min || parsed > max)) {
      fail(`Value should be between ${validation.min} and ${validation.max}`);
    }
  };
  switch (validation.type) {
    case "int":
      if (!/^\d+$/.test(value)) fail("Expected an integer");
      bounds(parseInt, parseInt(value));
      break;
    case "float":
      if (!/^\d+(\.\d+)?$/.test(value)) fail("Expected a number");
      bounds(parseFloat, parseFloat(value));
      break;
    case "exp": {
      let parsed = 0;
      try {
        parsed = parseExp(value);
      } catch (e) {
        fail((e as Error).message);
      }
      bounds(parseExp, parsed);
      break;
    }
  }
  return value;
}

/** The saved inputs of a theory, checked; empty inputs are left out */
export function readTheoryInputs(theory: string): TheoryInputs {
  const stored = loadStored()[theory] ?? {};
  const result: TheoryInputs = {};
  for (const [id, input] of theoryInputDefs(theory)) {
    const value = validate(theory, input, stored[id] ?? defaultValue(input));
    if (value !== undefined) result[id] = value;
  }
  return result;
}

/** Every theory's inputs ("All" mode), for the theories that have any */
export function readAllTheoryInputs(theories: string[]): Partial<Record<theoryType, TheoryInputs>> {
  const result: Partial<Record<theoryType, TheoryInputs>> = {};
  for (const theory of theories) {
    if (theoryInputDefs(theory).length > 0) result[theory as theoryType] = readTheoryInputs(theory);
  }
  return result;
}

// "All" mode dialog
const dialog = qs<HTMLDialogElement>(".theoryInputsDialog");
const dialogContent = qs(".theoryInputsDialogContent");

event(qs<HTMLButtonElement>(".theoryInputsBtn"), "pointerdown", () => {
  renderAllTheoryInputs(dialogContent, Object.keys(data.theories));
  dialog.showModal();
  document.body.style.overflow = "hidden";
});
event(qs<HTMLButtonElement>(".theoryInputsCloseBtn"), "pointerdown", () => dialog.close());
event(dialog, "close", () => document.body.style.overflow = "auto");
