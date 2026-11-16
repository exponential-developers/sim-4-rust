import jsonData from "../Data/data.json" with { type: "json" };
import { getTheoryFromIndex, isMainTheory, parseLog10String, reverseMulti } from "../Utils/helpers";
import { qs } from "../Utils/DOMhelpers";
import { readAllTheoryInputs, readTheoryInputs } from "../UI/theoryInputs";

//Inputs
const modeSelector = qs<HTMLSelectElement>(".mode");
const theorySelector = qs<HTMLSelectElement>(".theory");
const stratSelector = qs<HTMLSelectElement>(".strat");
const sigmaInput = qs<HTMLInputElement>(".sigma");
const currencyInput = qs<HTMLInputElement>(".input");
const capInput = qs<HTMLInputElement>(".cap");
const simAllInputArea = qs<HTMLTextAreaElement>(".simAllInputArea")
const modeInput = qs<HTMLTextAreaElement>(".modeInput");
const hardCap = qs<HTMLInputElement>(".hardCap");
const semi_idle = qs<HTMLInputElement>(".semi-idle");
const hard_active = qs<HTMLInputElement>(".hard-active");

//Setting Inputs
const dtOtp = qs(".dtOtp");
const ddtOtp = qs(".ddtOtp");
const boughtVarsDeltaSlider = qs<HTMLInputElement>(".boughtVarsDelta");
const themeSelector = qs<HTMLSelectElement>(".themeSelector");
const simAllStrats = qs<HTMLSelectElement>(".simallstrats");
const completedCTs = qs<HTMLSelectElement>(".completedcts");
const showA23 = qs<HTMLInputElement>(".a23");
const showUnofficials = qs<HTMLInputElement>(".unofficials");

function parseSettings(): Settings {
    return {
        dt: parseFloat(dtOtp.textContent ?? "1.5"),
        ddt: parseFloat(ddtOtp.textContent ?? "1.0001"),
        bought_vars_delta: parseInt(boughtVarsDeltaSlider.value),
        theme: themeSelector.value,
        sim_all_strats: simAllStrats.value as SettingsSimAllStratsMode,
        completed_cts: completedCTs.value as SettingsCompletedCTsMode,
        show_a23: showA23.checked,
        show_unofficials: showUnofficials.checked
    }
}

function parseExponentialValue(str: string): number {
    if (/^e?\d+(\.\d+)?$/.test(str)) {
        if (str.charAt(0) == 'e') str = str.slice(1);
        return parseFloat(str);
    }
    else if (/^\d+(\.\d+)?e\d+$/.test(str)) {
        return parseLog10String(str);
    }
    else {
        throw new Error(`Invalid currency value ${str}. Currency value must be in formats <number>, <exxxx> or <xexxxx>.`);
    }
}

function parseCurrency(str: string, theory: theoryType, sigma: number, defaultType = "r") {
    str = str.replace(" ", "");
    // Theories without rho (NLI) take tau values (sim-3 does the same)
    const supportsRho = (jsonData.theories as TheoryDataStructure)[theory].supportsRho ?? true;
    if (!supportsRho && defaultType === "r") defaultType = "t";

    const inputType = str.match(/[rtm]$/g);
    let type = defaultType;
    if (inputType) {
        type = inputType[0];
        str = str.slice(0, str.length - 1);
    };
    if (!supportsRho && type === "r") throw new Error(`Cannot use rho values for ${theory}: enter tau (t) or a multiplier (m).`);

    const value = parseExponentialValue(str);

    if (type == 't') {
        return value / jsonData.theories[theory].tau_factor;
    }
    else if (type == 'm') {
        return reverseMulti(theory, value, sigma);
    }
    return value;
}

function parseSigma(required: boolean): number {
    const str = sigmaInput.value.replace(" ", "");
    const match = str.match(/^\d+$/g);
    if (match) {
        return parseInt(match[0]);
    }
    else {
        if (required) {
            throw new Error("Invalid sigma value. Sigma must be an integer that's >= 0");
        }
        return 0;
    }
}

function parseSingleSim(): SingleSimQuery {
    const theory = theorySelector.value as theoryType;
    const sigma = parseSigma(isMainTheory(theory));

    return {
        queryType: "single",
        theory: theory,
        strat: stratSelector.value,
        sigma: sigma,
        rho: parseCurrency(currencyInput.value, theory, sigma),
        settings: parseSettings(),
        theory_inputs: readTheoryInputs(theory)
    }
}

function parseChainSim(): ChainSimQuery {
    const theory = theorySelector.value as theoryType;
    const sigma = parseSigma(isMainTheory(theory));

    return {
        queryType: "chain",
        theory: theory,
        strat: stratSelector.value,
        sigma: sigma,
        rho: parseCurrency(currencyInput.value, theory, sigma),
        cap: parseCurrency(capInput.value, theory, sigma),
        hard_cap: hardCap.checked,
        settings: parseSettings(),
        theory_inputs: readTheoryInputs(theory)
    }
}

function parseStepSim(): StepSimQuery {
    const theory = theorySelector.value as theoryType;
    const sigma = parseSigma(isMainTheory(theory));

    return {
        queryType: "step",
        theory: theory,
        strat: stratSelector.value,
        sigma: sigma,
        rho: parseCurrency(currencyInput.value, theory, sigma),
        cap: parseCurrency(capInput.value, theory, sigma),
        step: parseExponentialValue(modeInput.value),
        settings: parseSettings(),
        theory_inputs: readTheoryInputs(theory)
    }
}

function parseStepChainSim(): StepChainQuery {
    const theory = theorySelector.value as theoryType;
    const sigma = parseSigma(isMainTheory(theory));

    return {
        queryType: "step_chain",
        theory: theory,
        strat: stratSelector.value,
        sigma: sigma,
        rho: parseCurrency(currencyInput.value, theory, sigma),
        cap: parseCurrency(capInput.value, theory, sigma),
        step: parseExponentialValue(modeInput.value),
        hard_cap: hardCap.checked,
        settings: parseSettings(),
        theory_inputs: readTheoryInputs(theory)
    }
}

function parseAmountSim(): AmountSimQuery {
    const theory = theorySelector.value as theoryType;
    const sigma = parseSigma(isMainTheory(theory));
    const amount = parseInt(modeInput.value);
    if (!(amount > 0)) throw new Error("Invalid amount of pubs. It must be a whole number greater than 0.");

    return {
        queryType: "amount",
        theory: theory,
        strat: stratSelector.value,
        sigma: sigma,
        rho: parseCurrency(currencyInput.value, theory, sigma),
        amount: amount,
        settings: parseSettings(),
        theory_inputs: readTheoryInputs(theory)
    }
}

/** Parses a duration like "1d12h30m" (y, d, h, m) to seconds, like sim-3 */
function parseDuration(str: string): number {
    const units: Record<string, number> = { y: 3600 * 24 * 365, d: 3600 * 24, h: 3600, m: 60 };
    const regex = /(\d+)([ydhm])/g;
    let time = 0;
    let component: RegExpExecArray | null;
    while ((component = regex.exec(str)) !== null) {
        time += parseInt(component[1]) * units[component[2]];
    }
    return time;
}

function parseTimeSim(): TimeSimQuery {
    const theory = theorySelector.value as theoryType;
    const sigma = parseSigma(isMainTheory(theory));
    const time = parseDuration(modeInput.value);
    if (time <= 0) throw new Error("Invalid total time. Use a duration like 12h, 3d or 1d12h30m.");

    return {
        queryType: "time",
        theory: theory,
        strat: stratSelector.value,
        sigma: sigma,
        rho: parseCurrency(currencyInput.value, theory, sigma),
        time: time,
        settings: parseSettings(),
        theory_inputs: readTheoryInputs(theory)
    }
}

function parseComparisonSim(): ComparisonSimQuery {
    const theory = theorySelector.value as theoryType;
    const sigma = parseSigma(isMainTheory(theory));

    return {
        queryType: "comparison",
        theory: theory,
        sigma: sigma,
        rho: parseCurrency(currencyInput.value, theory, sigma),
        settings: parseSettings(),
        theory_inputs: readTheoryInputs(theory)
    }
}

function parsePubTableSim(): PubTableSimQuery {
    const theory = theorySelector.value as theoryType;
    const sigma = parseSigma(isMainTheory(theory));

    return {
        queryType: "pub_table",
        theory: theory,
        strat: stratSelector.value,
        sigma: sigma,
        rho: parseCurrency(currencyInput.value, theory, sigma),
        cap: parseCurrency(capInput.value, theory, sigma),
        step: parseExponentialValue(modeInput.value),
        settings: parseSettings(),
        theory_inputs: readTheoryInputs(theory)
    }
}

function parseSimAll(): SimAllQuery {
    const settings = parseSettings();
    const str = simAllInputArea.value;
    const split = str.split(" ").map(s => s.replace("\n", "")).filter(s => s != "");
    
    const sigmaStr = split.shift() ?? "";
    if (split.length < 1) throw new Error("Student count and at least one theory value that is not 0 is required.");
    if (split.length > Object.keys(jsonData.theories).length) {
        throw new Error(`Invalid value ${split[Object.keys(jsonData.theories).length]} does not match any theory.`);
    }

    const sigmaMatch = sigmaStr.match(/^\d+$/);
    if (!sigmaMatch) throw new Error("Invalid sigma value. Sigma must be an integer that's >= 0");
    const sigma = parseInt(sigmaMatch[0]);

    let values = split.map((val, i) => parseCurrency(val, getTheoryFromIndex(i), sigma, 't'));

    values = values.map((val, i) => {
        const theory = getTheoryFromIndex(i);
        if (settings.completed_cts === "no" && i >= 8 && val * jsonData.theories[theory].tau_factor >= 600) return 0;
        if (!settings.show_unofficials && (jsonData.theories as TheoryDataStructure)[theory].UI_visible === false) return 0;
        return val;
    })

    if (values.length - values.filter(val => val <= 0).length < 1) throw new Error("Student count and at least one theory value that is not 0 is required.");

    return {
        queryType: "all",
        sigma: sigma,
        values: values,
        very_active: hard_active.checked,
        semi_idle: semi_idle.checked,
        strat_type: settings.sim_all_strats,
        settings: settings,
        theory_inputs: readAllTheoryInputs(Object.keys(jsonData.theories))
    }
}

export function parseQuery(): SimQuery {
    switch (modeSelector.value) {
        case "All": return parseSimAll();
        case "Single sim": return parseSingleSim();
        case "Chain": return parseChainSim();
        case "Steps": return parseStepSim();
        case "StepChain": return parseStepChainSim();
        case "Amount": return parseAmountSim();
        case "Time": return parseTimeSim();
        case "Comparison": return parseComparisonSim();
        case "Pub Table": return parsePubTableSim();
        default: throw new Error("This mode is not supported.");
    }
}