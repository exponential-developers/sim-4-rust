/*! Theory implementations */

/**
 * Declares a strat enum with `ALL` (in the order listed, which is data.json order), `name()` and
 * `from_name()`; the names are the variant names.
 */
macro_rules! strat_enum {
    ($(#[$meta:meta])* $vis:vis enum $name:ident { $($variant:ident),+ $(,)? }) => {
        $(#[$meta])*
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

pub mod t1;
pub mod t2;
pub mod t3;
pub mod t4;
pub mod t5;
pub mod t6;
pub mod t7;
pub mod t8;
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
        let names = |all: &[&str]| {
            all.iter()
                .map(|s| s.to_string())
                .collect::<BTreeSet<String>>()
        };
        assert_eq!(
            names(
                &super::t6::T6Strat::ALL
                    .iter()
                    .map(|s| s.name())
                    .collect::<Vec<_>>()
            ),
            data_json_strats("T6")
        );
        assert_eq!(
            names(
                &super::t7::T7Strat::ALL
                    .iter()
                    .map(|s| s.name())
                    .collect::<Vec<_>>()
            ),
            data_json_strats("T7")
        );
        assert_eq!(
            names(
                &super::t8::T8Strat::ALL
                    .iter()
                    .map(|s| s.name())
                    .collect::<Vec<_>>()
            ),
            data_json_strats("T8")
        );
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
        for &s in super::t6::T6Strat::ALL {
            assert_eq!(super::t6::T6Strat::from_name(s.name()), Some(s));
        }
        for &s in super::t7::T7Strat::ALL {
            assert_eq!(super::t7::T7Strat::from_name(s.name()), Some(s));
        }
        for &s in super::t8::T8Strat::ALL {
            assert_eq!(super::t8::T8Strat::from_name(s.name()), Some(s));
        }
        assert_eq!(super::t1::T1Strat::from_name("T2"), None);
    }
}
