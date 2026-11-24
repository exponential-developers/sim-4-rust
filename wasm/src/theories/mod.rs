/*! Theory implementations */

/**
 * Declares a strat enum with `ALL` (in the order listed, which is data.json order), `name()` and
 * `from_name()`; the names are the variant names.
 */
macro_rules! strat_enum {
    ($(#[$meta:meta])* $vis:vis enum $name:ident { $($variant:ident),+ $(,)? }) => {
        $(#[$meta])*
        // The variants are the strat names from sim-3's data.json (e.g. `WSP`, `RZd`)
        #[allow(clippy::upper_case_acronyms)]
        $vis enum $name {
            $($variant),+
        }

        impl $name {
            pub const ALL: &'static [$name] = &[$($name::$variant),+];

            pub fn name(self) -> &'static str {
                match self {
                    $($name::$variant => stringify!($variant)),+
                }
            }

            pub fn from_name(name: &str) -> Option<$name> {
                $name::ALL.iter().copied().find(|s| s.name() == name)
            }
        }
    };
}

pub mod bap;
pub mod bd;
pub mod bt;
pub mod csr2;
pub mod ef;
pub mod fi;
pub mod fp;
pub mod fs;
pub mod ilc;
pub mod mf;
pub mod nli;
pub mod pubtable;
pub mod rz;
pub mod sl;
pub mod t1;
pub mod t2;
pub mod t3;
pub mod t4;
pub mod t5;
pub mod t6;
pub mod t7;
pub mod t8;
pub mod tc;
pub mod toe;
pub mod traditional;
pub mod wsp;

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    /** Strat names of a theory in src/Data/data.json */
    fn data_json_strats(theory: &str) -> BTreeSet<String> {
        let data: serde_json::Value =
            serde_json::from_str(include_str!("../../../src/Data/data.json")).unwrap();
        data["theories"][theory]["strats"]
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect()
    }

    /**
     * "All" mode sends one value per theory in data.json order and the sim reads value i as
     * `TheoryType` i, so both orders must match
     */
    #[test]
    fn theory_types_follow_data_json_order() {
        use crate::api::config::TheoryType;
        // serde_json sorts object keys, so read the theory names from the text: they are the keys
        // indented by 4 spaces, before "strat_categories"
        let text = include_str!("../../../src/Data/data.json");
        let theories = &text[..text.find("\"strat_categories\"").unwrap()];
        let names: Vec<&str> = theories
            .lines()
            .filter_map(|l| {
                l.strip_prefix("    \"")?
                    .split_once("\": {")
                    .map(|(n, _)| n)
            })
            .collect();
        assert!(names.len() > 20, "{names:?}");
        for (i, name) in names.iter().enumerate() {
            let theory = TheoryType::try_from(i).unwrap_or_else(|_| panic!("no TheoryType {i}"));
            assert_eq!(serde_json::to_value(theory).unwrap(), *name, "theory {i}");
        }
        assert!(TheoryType::try_from(names.len()).is_err());
    }

    /**
     * Every ported strat must be listed in data.json and vice versa, so "Best" selection and the
     * strat dropdown can only pick strats that exist; and names must round trip.
     */
    fn check<S: Copy + PartialEq + std::fmt::Debug>(
        theory: &str,
        all: &[S],
        name: fn(S) -> &'static str,
        from_name: fn(&str) -> Option<S>,
    ) {
        let names: BTreeSet<String> = all.iter().map(|&s| name(s).to_owned()).collect();
        assert_eq!(names, data_json_strats(theory), "{theory}");
        for &s in all {
            assert_eq!(from_name(name(s)), Some(s), "{theory}");
        }
    }

    /** One line per ported theory: its data.json name, module and strat enum */
    macro_rules! ported_theories {
        ($($theory:literal => $module:ident :: $strat:ident),+ $(,)?) => {
            #[test]
            fn ported_strats_match_data_json() {
                $(check($theory, &super::$module::$strat::ALL, super::$module::$strat::name, super::$module::$strat::from_name);)+
                assert_eq!(super::t1::T1Strat::from_name("T2"), None);
            }
        };
    }

    ported_theories! {
        "T1" => t1::T1Strat,
        "T2" => t2::T2Strat,
        "T3" => t3::T3Strat,
        "T4" => t4::T4Strat,
        "T5" => t5::T5Strat,
        "T6" => t6::T6Strat,
        "T7" => t7::T7Strat,
        "T8" => t8::T8Strat,
        "WSP" => wsp::WspStrat,
        "SL" => sl::SlStrat,
        "EF" => ef::EfStrat,
        "CSR2" => csr2::Csr2Strat,
        "BaP" => bap::BapStrat,
        "FI" => fi::FiStrat,
        "FP" => fp::FpStrat,
        "BT" => bt::BtStrat,
        "TC" => tc::TcStrat,
        "MF" => mf::MfStrat,
        "RZ" => rz::RzStrat,
        "FS" => fs::FsStrat,
        "BD" => bd::BdStrat,
        "ILC" => ilc::IlcStrat,
        "NLI" => nli::NliStrat,
        "TOE" => toe::ToeStrat,
    }
}
