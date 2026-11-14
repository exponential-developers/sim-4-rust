use num_enum::TryFromPrimitive;
use serde::{Deserialize, Deserializer, Serialize, de};
use std::collections::HashMap;
use std::fmt;

use crate::api::condition::Condition;

/** enum of all available theory types */
#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize, TryFromPrimitive)]
#[repr(usize)]
//#[serde(rename_all = "lowercase")]
pub enum TheoryType {
    T1,
    T2,
    T3,
    T4,
    T5,
    T6,
    T7,
    T8,
    WSP,
    SL,
    EF,
    CSR2,
    FI,
    FP,
    RZ,
    MF,
    BaP,
    TC,
    BT,
}

/** Conditions deciding when a strat is used by the "Best ..." strat categories */
#[derive(Debug, Clone, Deserialize)]
pub struct StratConfig {
    /** Whether the strat fits the chosen play style / progress */
    #[serde(rename = "stratFilterCondition", default)]
    pub filter: Condition,
    /** Must always hold for the strat to be simulated */
    #[serde(rename = "forcedCondition", default)]
    pub forced: Condition,
}

/** configuration for a theory */
#[derive(Debug, Deserialize)]
pub struct ConfigTheories {
    pub tau_factor: f64,
    /** Strats in data.json order (order decides ties between equally good strats) */
    #[serde(default, deserialize_with = "ordered_strats")]
    pub strats: Vec<(String, StratConfig)>,
}

/** structure for the given theory configuration as written in data.json needed
by the wasm module
*/
#[derive(Debug, Deserialize)]
pub struct Config {
    pub theories: HashMap<TheoryType, ConfigTheories>,
    pub strat_categories: Vec<String>,
}

/** Deserializes a JSON object into a list of (key, value), keeping the file order */
fn ordered_strats<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<(String, StratConfig)>, D::Error> {
    struct OrderedVisitor;

    impl<'de> de::Visitor<'de> for OrderedVisitor {
        type Value = Vec<(String, StratConfig)>;

        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("an object of strats")
        }

        fn visit_map<A: de::MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
            let mut out = Vec::new();
            while let Some((k, v)) = map.next_entry::<String, StratConfig>()? {
                out.push((k, v));
            }
            Ok(out)
        }
    }

    deserializer.deserialize_map(OrderedVisitor)
}
