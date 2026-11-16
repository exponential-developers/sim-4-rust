import { round } from "../Utils/helpers";
import { qs } from "../Utils/DOMhelpers";

//Sim All Settings
const semiIdle = qs<HTMLInputElement>(".semi-idle");
const hardActive = qs<HTMLInputElement>(".hard-active");

//Setting Inputs
const dtSlider = qs<HTMLInputElement>(".dt");
const dtOtp = qs(".dtOtp");

const ddtSlider = qs<HTMLInputElement>(".ddt");
const ddtOtp = qs(".ddtOtp");

const boughtVarsDeltaSlider = qs<HTMLInputElement>(".boughtVarsDelta");
const boughtVarsDeltaOtp = qs(".boughtVarsDeltaOtp");

const themeSelector = qs<HTMLSelectElement>(".themeSelector");

const simAllStrats = qs<HTMLSelectElement>(".simallstrats");
const completedCTs = qs<HTMLInputElement>(".completedcts");
const showA23 = qs<HTMLInputElement>(".a23");
const showUnofficials = qs<HTMLInputElement>(".unofficials");

const defaultState = `{"settings":{"dt":"1.5","ddt":"1.0001","showA23":false,"showUnofficials":false}}`;

/** What setSimState saves under "simState" (older saves may lack fields, or have skipCompletedCTs) */
type SavedSimState = {
  settings: {
    dt: string;
    ddt: string;
    /** MF's reset depth, now a theory input (theoryInputs.ts keeps an old value) */
    mfResetDepth?: string;
    boughtVarsDelta?: string | number;
    simAllStrats?: string;
    completedCTs?: string;
    skipCompletedCTs?: boolean;
    showA23?: boolean;
    showUnofficials?: boolean;
    theme?: string;
  };
};
// Keep in sync with the inline script in index.html
const defaultTheme = window.matchMedia('(prefers-color-scheme: dark)').matches ? "dark" : "light";

// Main inputs, remembered between visits
const modeSelector = qs<HTMLSelectElement>(".mode");
const theorySelector = qs<HTMLSelectElement>(".theory");
const stratSelector = qs<HTMLSelectElement>(".strat");
const sigmaInput = qs<HTMLInputElement>(".sigma");
const currencyInput = qs<HTMLInputElement>(".input");
const capInput = qs<HTMLInputElement>(".cap");
const hardCapInput = qs<HTMLInputElement>(".hardCap");
const modeInput = qs<HTMLTextAreaElement>(".modeInput");
const simAllInputArea = qs<HTMLTextAreaElement>(".simAllInputArea");

/** Elements whose values are saved by `saveInputs` */
export const savedInputElements: HTMLElement[] = [
  modeSelector, theorySelector, stratSelector, sigmaInput, currencyInput,
  capInput, hardCapInput, modeInput, simAllInputArea
];

interface SavedInputs {
  mode?: string;
  theory?: string;
  strat?: string;
  sigma?: string;
  input?: string;
  cap?: string;
  hardCap?: boolean;
  modeInput?: string;
  simAllInput?: string;
}

/** Saves the main inputs to localStorage */
export function saveInputs() {
  const inputs: SavedInputs = {
    mode: modeSelector.value,
    theory: theorySelector.value,
    strat: stratSelector.value,
    sigma: sigmaInput.value,
    input: currencyInput.value,
    cap: capInput.value,
    hardCap: hardCapInput.checked,
    modeInput: modeInput.value,
    simAllInput: simAllInputArea.value,
  };
  localStorage.setItem("simInputs", JSON.stringify(inputs));
}

/**
 * Restores the saved main inputs.
 * `onModeChange`/`onTheoryChange` repopulate the dependent fields (render.ts modeUpdate/theoryUpdate).
 */
export function restoreInputs(onModeChange: () => void, onTheoryChange: () => void) {
  let inputs: SavedInputs;
  try {
    inputs = JSON.parse(localStorage.getItem("simInputs") ?? "{}") as SavedInputs;
  } catch {
    return;
  }
  const hasOption = (select: HTMLSelectElement, value?: string) =>
    value !== undefined && Array.from(select.options).some((o) => o.value === value);

  // The "All" text area feeds sigma/input autofill in render.ts, so restore it first
  if (inputs.simAllInput !== undefined) simAllInputArea.value = inputs.simAllInput;
  if (hasOption(modeSelector, inputs.mode)) {
    modeSelector.value = inputs.mode!;
    onModeChange();
  }
  if (hasOption(theorySelector, inputs.theory)) {
    theorySelector.value = inputs.theory!;
    onTheoryChange();
  }
  if (hasOption(stratSelector, inputs.strat)) stratSelector.value = inputs.strat!;
  if (inputs.sigma !== undefined) sigmaInput.value = inputs.sigma;
  if (inputs.input !== undefined) currencyInput.value = inputs.input;
  if (inputs.cap !== undefined) capInput.value = inputs.cap;
  if (inputs.hardCap !== undefined) hardCapInput.checked = inputs.hardCap;
  if (inputs.modeInput !== undefined) modeInput.value = inputs.modeInput;
}

export function setSimState() {
  localStorage.setItem("simAllSettings", JSON.stringify([semiIdle.checked, hardActive.checked]));
  localStorage.setItem(
    "simState",
    JSON.stringify({
      settings: {
        dt: dtOtp.textContent,
        ddt: ddtOtp.textContent,
        boughtVarsDelta: boughtVarsDeltaSlider.value,
        simAllStrats: simAllStrats.value,
        completedCTs: completedCTs.value,
        showA23: showA23.checked,
        showUnofficials: showUnofficials.checked,
        theme: themeSelector.value
      },
    })
  );
}
export function getSimState() {
  //Sim All Settings
  const simAllSettings = JSON.parse(localStorage.getItem("simAllSettings") ?? "[true, false]") as [boolean, boolean];
  semiIdle.checked = simAllSettings[0];
  hardActive.checked = simAllSettings[1];

  // Sim Settings
  const { settings } = JSON.parse(localStorage.getItem("simState") ?? defaultState) as SavedSimState;

  dtOtp.textContent = settings.dt;
  ddtOtp.textContent = settings.ddt;
  boughtVarsDeltaOtp.textContent = `e${settings.boughtVarsDelta ?? 5}ρ`;
  // Determines the slider position based on the stored value (see helpers.ts)
  dtSlider.value = String(round(Math.log2((Number(settings.dt) - 0.15) / (4.9 / (1 + 2 ** parseFloat(dtSlider.max)))), 4));
  ddtSlider.value = String(round(Math.log((Number(settings.ddt) - 1) / (0.3 / 3 ** parseFloat(ddtSlider.max))) / Math.log(3), 4));
  boughtVarsDeltaSlider.value = String(settings.boughtVarsDelta ?? 5);

  themeSelector.value = settings.theme ?? defaultTheme;
  simAllStrats.value = settings.simAllStrats ?? "all";
  completedCTs.value = settings.completedCTs ?? (settings.skipCompletedCTs === true ? "no" : "in");
  showA23.checked = settings.showA23 ?? false;
  showUnofficials.checked = settings.showUnofficials ?? false;
}
