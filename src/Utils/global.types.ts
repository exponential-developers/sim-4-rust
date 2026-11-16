import jsonData from "../Data/data.json" with { type: "json" };

declare global {
  type conditionFunction = () => boolean;

  type theoryType = keyof typeof jsonData.theories;
  type stratType = {
    [key in theoryType]: keyof (typeof jsonData.theories)[key]["strats"];
  };

  /** A theory's own input (data.json `specificInputs`, as in sim-3) */
  type SpecificInput = {
    label: string;
    hint?: string;
  } & (
    | {
        type: "slider";
        validation: { type: "int" | "float"; min: number; max: number };
        step: number;
        default?: number;
      }
    | {
        type: "textbox";
        validation: { type: "int" | "float" | "exp" | "string"; min?: number | string; max?: number | string };
        placeholder?: string;
      }
  );

  /** Values of a theory's own inputs by id, as entered (e.g. { depth: "2" }) */
  type TheoryInputs = Record<string, string>;

  type TheoryDataStructure = {
    [key: string]: {
      tau_factor: number;
      UI_visible?: boolean;
      /** False for theories without rho (NLI): inputs are tau values */
      supportsRho?: boolean;
      specificInputs?: Record<string, SpecificInput>;
      strats: {
        [key: string]: {
          stratFilterCondition: string;
          forcedCondition?: string;
          UI_visible?: boolean;
        }
      }
    }
  }

  type BaseSimQuery = {
    sigma: number;
    settings: Settings;
    /** The theory's own inputs (single-theory modes) */
    theory_inputs?: TheoryInputs;
  }

  type SingleSimQuery = BaseSimQuery & {
    queryType: "single";
    theory: theoryType;
    strat: string;
    rho: number;
    cap?: number;
    last_strat?: string;
  }

  type ChainSimQuery = BaseSimQuery & {
    queryType: "chain";
    theory: theoryType;
    strat: string;
    rho: number;
    cap: number;
    hard_cap: boolean;
  }

  type StepSimQuery = BaseSimQuery & {
    queryType: "step";
    theory: theoryType;
    strat: string;
    rho: number;
    cap: number;
    step: number;
  }

  type SimAllQuery = Omit<BaseSimQuery, "theory_inputs"> & {
    /** Each theory's own inputs */
    theory_inputs: Partial<Record<theoryType, TheoryInputs>>;
    queryType: "all";
    values: number[];

    very_active: boolean;
    semi_idle: boolean;
    strat_type: SettingsSimAllStratsMode;
  }

  type StepChainQuery = BaseSimQuery & {
    queryType: "step_chain"
    theory: theoryType
    strat: string
    rho: number
    cap: number
    step: number
    hard_cap: boolean
  }

  type AmountSimQuery = BaseSimQuery & {
    queryType: "amount";
    theory: theoryType;
    strat: string;
    rho: number;
    amount: number;
  }

  type TimeSimQuery = BaseSimQuery & {
    queryType: "time";
    theory: theoryType;
    strat: string;
    rho: number;
    /** Total time in seconds */
    time: number;
  }

  type ComparisonSimQuery = BaseSimQuery & {
    queryType: "comparison";
    theory: theoryType;
    rho: number;
  }

  type PubTableSimQuery = BaseSimQuery & {
    queryType: "pub_table";
    theory: theoryType;
    strat: string;
    rho: number;
    cap: number;
    step: number;
  }

  type SimQuery = SingleSimQuery | ChainSimQuery | StepSimQuery | SimAllQuery | StepChainQuery
    | AmountSimQuery | TimeSimQuery | ComparisonSimQuery | PubTableSimQuery;

  type SingleSimResponse = {
    responseType: "single";
    result: simResult;
  }

  type ChainSimResponse = {
    responseType: "chain";
    results: simResult[];
    delta_tau: number;
    average_rate: number;
    total_time: number;
  }

  type StepSimResponse = {
    responseType: "step";
    results: simResult[];
  }

  type SimAllResponse = {
    responseType: "all";
    sigma: number;
    strat_type: SettingsSimAllStratsMode;
    completed_cts: SettingsCompletedCTsMode;
    results: simAllResult[];
  }

  type PubTableResponse = {
    responseType: "pub_table";
    /** log10 rho values */
    cap: number;
    start: number;
    step: number;
    /** [time to reach cap, next pub target] for cap, cap - step, cap - 2*step, ... */
    pub_table: [number, number][];
  }

  type SimResponse = SingleSimResponse | ChainSimResponse | StepSimResponse | SimAllResponse | PubTableResponse;

  interface varBuy {
    var_name: string;
    level: number;
    cost: number;
    symbol?: string;
    timestamp: number;
  }

  interface theoryData {
    theory: theoryType;
    sigma: number;
    rho: number;
    strat: string;
    recovery: null | { value: number; time: number; recoveryTime: boolean };
    cap: null | number;
    recursionValue: null | number | number[];
    settings: Settings;
  }

  type SettingsSimAllStratsMode = "all" | "active" | "idle";
  type SettingsCompletedCTsMode = "in" | "end" | "no";
  type Settings = {
    dt: number;
    ddt: number;
    bought_vars_delta: number;
    theme: string;
    sim_all_strats: SettingsSimAllStratsMode;
    completed_cts: SettingsCompletedCTsMode;
    show_a23: boolean;
    show_unofficials: boolean;
  }

  // Deprecated in TS
  interface simResult {
    theory: theoryType;
    sigma: number;
    last_pub: number;
    pub_rho: number;
    delta_tau: number;
    pub_multi: number;
    strat: string;
    tau_h: number;
    time: number;
    bought_vars: varBuy[];
    /** Theory inputs for the next publication (NLI's lifetime α) */
    theory_inputs?: TheoryInputs;
  }

  // Deprecated in TS
  interface simAllResult {
    theory: theoryType;
    ratio: number;
    last_pub: number;
    active: simResult;
    idle: simResult;
  }

  /** Response of the wasm module's run_query (Rust `SimResponse`: `{ type, data }`) */
  type WasmSimResponse =
    | { type: "single"; data: Omit<SingleSimResponse, "responseType"> }
    | { type: "chain"; data: Omit<ChainSimResponse, "responseType"> }
    | { type: "step"; data: Omit<StepSimResponse, "responseType"> }
    | { type: "all"; data: Omit<SimAllResponse, "responseType"> }
    | { type: "pub_table"; data: Omit<PubTableResponse, "responseType"> };

  /** JSON returned by run_query (Rust `ApiResponse`) */
  type API_response =
    | { response_type: "success"; data: WasmSimResponse }
    | { response_type: "failure"; data: string };

  /** Message sent to the sim worker (src/Sim/worker.ts) */
  type WorkerRequest = { type: "query"; id: number; query: string };

  /** Messages sent by the sim worker */
  type WorkerResponse =
    | { type: "ready"; portedTheories: string[] }
    /** `fraction`: share of the work done (0 to 1), NaN when unknown */
    | { type: "progress"; id: number; message: string; fraction: number }
    /** A finished part of the response (JSON), sent as soon as it is done (src/Sim/partial.ts) */
    | { type: "finished"; id: number; json: string }
    | { type: "result"; id: number; response: string };
}
