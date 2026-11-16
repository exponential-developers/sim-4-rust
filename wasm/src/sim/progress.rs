/*!
 * Progress reports during long simulations: a status text and how much of the work is done.
 *
 * Modes nest (an "All" run simulates each theory's "Best" strats, a step chain runs chains):
 * `part` gives the inner work a progress of its own, whose texts follow the outer text and whose
 * fractions fill the outer work's share. For example "Simulating MF (17/23) · MFd2Coast (12/41)".
 */

/** Receives a status text and the fraction of the whole work done (0 to 1, NaN when unknown) */
pub type ProgressSink<'a> = &'a dyn Fn(&str, f64);

#[derive(Clone)]
pub struct Progress<'a> {
    sink: ProgressSink<'a>,
    /** Text before every message (the outer work's) */
    prefix: String,
    /** Share of the whole work this progress covers */
    from: f64,
    to: f64,
}

impl<'a> Progress<'a> {
    pub fn new(sink: ProgressSink<'a>) -> Self {
        Progress {
            sink,
            prefix: String::new(),
            from: 0.,
            to: 1.,
        }
    }

    /** `fraction` of this progress's share, as a fraction of the whole work */
    fn whole(&self, fraction: f64) -> f64 {
        self.from + (self.to - self.from) * fraction.clamp(0., 1.)
    }

    /** Reports `text` with `fraction` (0 to 1, NaN when unknown) of this work done */
    pub fn report(&self, text: &str, fraction: f64) {
        (self.sink)(&format!("{}{text}", self.prefix), self.whole(fraction));
    }

    /** Inner work covering `from` to `to` of this work; its messages follow `text` */
    pub fn part(&self, text: &str, from: f64, to: f64) -> Progress<'a> {
        Progress {
            sink: self.sink,
            prefix: format!("{}{text} · ", self.prefix),
            from: self.whole(from),
            to: self.whole(to),
        }
    }

    /** The same work, with `prefix` before its messages */
    pub fn prefixed(&self, prefix: &str) -> Progress<'a> {
        Progress {
            prefix: format!("{}{prefix}", self.prefix),
            ..self.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[test]
    fn parts_fill_their_share_and_prefix_their_messages() {
        let log = RefCell::new(Vec::new());
        let sink = |text: &str, fraction: f64| log.borrow_mut().push((text.to_owned(), fraction));
        let progress = Progress::new(&sink);
        progress.report("Simulating T1 (1/2)", 0.);
        let second = progress.part("Simulating T2 (2/2)", 0.5, 1.);
        second.report("T2MS (1/4)", 0.5);
        second.part("pub 1", 0.5, 0.5).report("x", 0.3);
        progress.prefixed("Simulating ").report("T2", 2.);
        progress.report("unknown", f64::NAN);

        let log = log.borrow();
        assert_eq!(log[0], ("Simulating T1 (1/2)".to_owned(), 0.));
        assert_eq!(
            log[1],
            ("Simulating T2 (2/2) · T2MS (1/4)".to_owned(), 0.75)
        );
        assert_eq!(log[2], ("Simulating T2 (2/2) · pub 1 · x".to_owned(), 0.75));
        // Fractions are kept between 0 and 1
        assert_eq!(log[3], ("Simulating T2".to_owned(), 1.));
        assert!(log[4].1.is_nan());
    }
}
