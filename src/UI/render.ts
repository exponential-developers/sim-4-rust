import data from "../Data/data.json" with { type: "json" };
import { findIndex, getIndexFromTheory } from "../Utils/helpers";
import { qs, qsa, event, ce, removeAllChilds } from "../Utils/DOMhelpers";
import { getSimState, setSimState, restoreInputs, saveInputs, savedInputElements } from "./simState";
import { renderTheoryInputs } from "./theoryInputs";

//Inputs
const theorySelector = qs<HTMLSelectElement>(".theory");
const stratSelector = qs<HTMLSelectElement>(".strat");
const capInputWrapper = qs(".capWrapper");
const modeSelector = qs<HTMLSelectElement>(".mode");
const sigmaInput = qs<HTMLInputElement>(".sigma");
const currencyInput = qs<HTMLInputElement>(".input");
const capInput = qs<HTMLInputElement>(".cap");
const simAllInputArea = qs<HTMLTextAreaElement>(".simAllInputArea")
const modeInput = qs<HTMLTextAreaElement>(".modeInput");
const hardCapWrapper = qs(".hardCapWrapper");
const themeSelector = qs<HTMLSelectElement>(".themeSelector");
const showUnofficials = qs<HTMLInputElement>(".unofficials");

//Other containers/elements
const extraInputs = qs(".extraInputs");
const timeDiffWrapper = qs(".timeDiffWrapper");
const singleInput = qsa(".controls")[0];
const simAllInputs = qs(".simAllInputs");
const modeInputDescription = qs(".extraInputDescription");
const testPortedButton = qs<HTMLButtonElement>(".testPorted");
const stratSelectorWrapper = stratSelector.parentElement as HTMLElement;
const theoryInputsWrapper = qs(".theoryInputs");
const theoryInputsBtn = qs<HTMLButtonElement>(".theoryInputsBtn");


const theories = Object.keys(data.theories) as theoryType[];

/** Populates a select element with the given items */
function populateSelectElement(select: HTMLSelectElement, items: string[], clear = true) {
  if (clear) removeAllChilds(select);
  for (const item of items) {
    const option = ce<HTMLOptionElement>("option");
    option.value = item;
    option.textContent = item;
    select.appendChild(option);
  }
}
function populateTheoryList(showUnofficials: boolean) {
  populateSelectElement(theorySelector, theories.filter(theory => 
    (data.theories as TheoryDataStructure)[theory].UI_visible !== false || showUnofficials));
}

//Renders theories, strats and modes options on page load

populateSelectElement(themeSelector, data.themes);
// Show readable names ("solarized-light" -> "Solarized Light"); values stay the theme ids
for (const option of Array.from(themeSelector.options)) {
  option.textContent = option.value.split("-").map((w) => w.charAt(0).toUpperCase() + w.slice(1)).join(" ");
}
event(themeSelector, "change", themeUpdate);

getSimState();

populateSelectElement(modeSelector, data.modes);
modeUpdate();
event(modeSelector, "input", modeUpdate);

populateTheoryList(showUnofficials.checked);
theoryUpdate();
event(theorySelector, "change", theoryUpdate);

event(showUnofficials, "click", () => {
    populateTheoryList(showUnofficials.checked);
    theoryUpdate();
});

// Restore the inputs from the last visit, then save them whenever they change
restoreInputs(modeUpdate, theoryUpdate);
for (const elem of savedInputElements) {
  event(elem, "change", saveInputs);
  event(elem, "input", saveInputs);
}

function populateSingleSimFields(rewriteCurrency: boolean = false): void {
  // Sigma field
  const splits = simAllInputArea.value.replace("\n", "").split(" ").filter(s => s != "")

  if (sigmaInput.value == "" && splits.length > 0) {
    const match = splits[0].match(/^\d+$/g);
    if (match) {
      sigmaInput.value = match[0];
    }
  }

  if ((currencyInput.value == "" || rewriteCurrency) && theorySelector.value && splits.length > 1) {
    const theoryIndex = getIndexFromTheory(theorySelector.value);
    if (splits.length > theoryIndex + 1) {
      const str = splits[theoryIndex + 1];
      const match = str.match(/^e?\d+(\.\d+)?[rtm]?$/) || str.match(/^\d+(\.\d+)?e\d+[rtm]?$/);
      if (match) {
        currencyInput.value = /[rtm]/.test(str) ? str : str.concat("t");
      }
    }
    else if (rewriteCurrency) {
      currencyInput.value = "";
    }
  }
}

function modeUpdate(): void {
  const newMode = modeSelector.value;

  singleInput.style.display = "none";
  capInputWrapper.style.display = "none";
  hardCapWrapper.style.display = "none";

  extraInputs.style.display = "none";
  simAllInputs.style.display = "none";
  simAllInputArea.style.display = "none";
  modeInputDescription.style.display = "inline";
  modeInput.style.display = "none";
  timeDiffWrapper.style.display = "none";

  // Displays the single-theory inputs
  if (newMode !== "All" && newMode !== "Time diff.") singleInput.style.display = "grid";
  // Displays the strat selector (Comparison simulates every strat)
  stratSelectorWrapper.style.display = newMode === "Comparison" ? "none" : "";
  // Displays the cap input for chain/steps/pub table mode
  if (newMode === "Chain" || newMode === "Steps" || newMode === "StepChain" || newMode === "Pub Table") capInputWrapper.style.display = "inline";
  // Displays the hard cap input
  if (newMode === "Chain" || newMode == "StepChain") hardCapWrapper.style.display = "block";

  // Extra Inputs
  if (newMode !== "Single sim" && newMode !== "Comparison" && newMode !== "Time diff." && newMode !== "Chain") extraInputs.style.display = "flex";
  testPortedButton.style.display = newMode === "All" ? "block" : "none";
  // "All" mode sets every theory's own inputs in a dialog
  theoryInputsBtn.style.display = newMode === "All" ? "" : "none";
  if (newMode === "All") {
    simAllInputs.style.display = "grid";
    modeInputDescription.style.display = "none";
    simAllInputArea.style.display = "block";
    simAllInputArea.placeholder = data.modeInputPlaceholder[0];
  }
  else {
    modeInput.style.display = "block";
  }
  modeInputDescription.textContent = data.modeInputDescriptions[findIndex(data.modes, newMode)];
  modeInput.placeholder = data.modeInputPlaceholder[findIndex(data.modes, newMode)];
  
  if (newMode === "Time diff.") timeDiffWrapper.style.display = "grid";

  populateSingleSimFields();
}

function theoryUpdate() {
  const currentTheory = theorySelector.value as theoryType;
  const currentTheoryStrats = Object.keys(data.theories[currentTheory].strats).filter(
    (strat) => (data.theories as TheoryDataStructure)[currentTheory].strats[strat].UI_visible !== false
  );
  populateSelectElement(stratSelector, data.strat_categories.concat(currentTheoryStrats));
  populateSingleSimFields(true);
  renderTheoryInputs(theoryInputsWrapper, currentTheory);
  // Theories without rho (NLI) take tau values
  const placeholder = (data.theories as TheoryDataStructure)[currentTheory].supportsRho === false ? "<tau>[t/m]" : "<rho>[r/t/m]";
  currencyInput.placeholder = placeholder;
  capInput.placeholder = placeholder;
}

function themeUpdate() {
  const root = document.documentElement;
  root.setAttribute("theme", themeSelector.value);
  // The picker is no longer inside the settings dialog (which saves on close), so save now
  setSimState();
}
