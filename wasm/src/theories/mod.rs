/*! Theory implementations */

pub mod t1;
pub mod t2;
pub mod t3;
pub mod t4;
pub mod t5;
pub mod traditional;

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

    /** Every ported strat must be listed in data.json and vice versa, so "Best" selection and the
    strat dropdown can only pick strats that exist */
    #[test]
    fn ported_strats_match_data_json() {
        let t1: BTreeSet<String> = super::t1::T1Strat::ALL
            .iter()
            .map(|s| s.name().to_owned())
            .collect();
        assert_eq!(t1, data_json_strats("T1"));
        let t2: BTreeSet<String> = super::t2::T2Strat::ALL
            .iter()
            .map(|s| s.name().to_owned())
            .collect();
        assert_eq!(t2, data_json_strats("T2"));
        let t3: BTreeSet<String> = super::t3::T3Strat::ALL
            .iter()
            .map(|s| s.name().to_owned())
            .collect();
        assert_eq!(t3, data_json_strats("T3"));
        let t4: BTreeSet<String> = super::t4::T4Strat::ALL
            .iter()
            .map(|s| s.name().to_owned())
            .collect();
        assert_eq!(t4, data_json_strats("T4"));
        let t5: BTreeSet<String> = super::t5::T5Strat::ALL
            .iter()
            .map(|s| s.name().to_owned())
            .collect();
        assert_eq!(t5, data_json_strats("T5"));
    }

    #[test]
    fn strat_names_round_trip() {
        for s in super::t1::T1Strat::ALL {
            assert_eq!(super::t1::T1Strat::from_name(s.name()), Some(s));
        }
        for s in super::t2::T2Strat::ALL {
            assert_eq!(super::t2::T2Strat::from_name(s.name()), Some(s));
        }
        for s in super::t3::T3Strat::ALL {
            assert_eq!(super::t3::T3Strat::from_name(s.name()), Some(s));
        }
        for s in super::t4::T4Strat::ALL {
            assert_eq!(super::t4::T4Strat::from_name(s.name()), Some(s));
        }
        for s in super::t5::T5Strat::ALL {
            assert_eq!(super::t5::T5Strat::from_name(s.name()), Some(s));
        }
        assert_eq!(super::t1::T1Strat::from_name("T2"), None);
    }
}
