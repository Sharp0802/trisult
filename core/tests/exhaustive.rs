#![cfg(feature = "alloc")]

use std::error::Error;
use std::fmt::{Display, Formatter};
use trisult::{
    Acc, All, Contextual, ContextualDiagnosis, Contextuals, Diagnosed, Diagnoses, Diagnosis,
    MapDiagnosis, Most, NoLoc, Prioritized, Trisult,
};

#[derive(Debug, Clone, PartialEq)]
pub struct W(i32);

impl Display for W {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "W({})", self.0)
    }
}

impl Error for W {}

#[derive(Debug, Clone, PartialEq)]
pub struct E(i32);

impl Display for E {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "E({})", self.0)
    }
}

impl Error for E {}

#[derive(Debug, Clone, PartialEq)]
pub struct Val(i32);

impl Display for Val {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Val({})", self.0)
    }
}

impl Error for Val {}

impl Prioritized for Val {
    type Priority = i32;
    fn priority(&self) -> i32 {
        self.0
    }
}

#[test]
fn test_accumulator_most() {
    use trisult::AccState;

    let mut most = Most::create_state::<Val, NoLoc>();

    AccState::reserve(&mut most, 5);

    // push
    {
        assert!(most.push_naive(Contextual::new(NoLoc, Val(0))));
        assert!(!most.push_naive(Contextual::new(NoLoc, Val(1)))); // ignored
        assert!(!most.push(Contextual::new(NoLoc, Val(10)))); // replaced (old one ignored)
        assert!(!most.push(Contextual::new(NoLoc, Val(1)))); // ignored
    }

    // append
    {
        let mut lhs = Most::create_state::<Val, NoLoc>();
        lhs.push(Contextual::new(NoLoc, Val(20)));
        lhs.push(Contextual::new(NoLoc, Val(30))); // replaced

        assert_eq!(most.append_naive(lhs.clone()), 1); // ignored
        assert!(most.iter().map(|d| d.value.0).eq([10]));

        assert_eq!(most.append(lhs.clone()), 1); // replaced
        assert!(most.iter().map(|d| d.value.0).eq([30]));
    }

    // append with empty
    {
        let mut lhs = Most::create_state::<Val, NoLoc>();
        assert_eq!(lhs.append_naive(most.clone()), 0); // replaced
        assert!(lhs.iter().map(|d| d.value.0).eq([30]));

        let lhs = Most::create_state::<Val, NoLoc>();
        assert_eq!(most.append(lhs.clone()), 0); // there is no item to append
        assert_eq!(most.append_naive(lhs), 0); // there is no item to append
    }

    // map
    {
        let mapped = AccState::map(most, |v| Val(v.0 + 1));
        assert_eq!(mapped.unwrap().value.0, 31);
    }

    // map empty
    {
        let lhs = Most::create_state::<Val, NoLoc>();
        let mapped = AccState::map(lhs, |v| Val(v.0 + 1));
        assert!(mapped.is_empty());
    }
}

#[test]
fn test_accumulator_all() {
    use trisult::AccState;

    let mut all = All::create_state::<Val, NoLoc>();

    AccState::reserve(&mut all, 5);

    // push
    {
        assert!(all.push_naive(Contextual::new(NoLoc, Val(0))));
        assert!(all.push_naive(Contextual::new(NoLoc, Val(1))));
        assert!(AccState::push(&mut all, Contextual::new(NoLoc, Val(10))));
        assert!(AccState::push(&mut all, Contextual::new(NoLoc, Val(1))));
    }

    // append
    {
        let mut lhs = All::create_state::<Val, NoLoc>();
        lhs.push(Contextual::new(NoLoc, Val(20)));
        lhs.push(Contextual::new(NoLoc, Val(30)));

        assert_eq!(all.append_naive(lhs.clone()), 0);
        assert!(all.iter().map(|d| d.value.0).eq([0, 1, 10, 1, 20, 30]));

        assert_eq!(AccState::append(&mut all, lhs.clone()), 0);
        assert!(
            all.iter()
                .map(|d| d.value.0)
                .eq([0, 1, 10, 1, 20, 30, 20, 30])
        );
    }

    // append with empty
    {
        let mut lhs = All::create_state::<Val, NoLoc>();
        assert_eq!(lhs.append_naive(all.clone()), 0);
        assert!(lhs.iter().eq(all.iter()));

        let lhs = All::create_state::<Val, NoLoc>();
        assert_eq!(AccState::append(&mut all, lhs.clone()), 0);
        assert_eq!(all.append_naive(lhs), 0);
        assert_eq!(all.len(), 8); // there is no item to append
    }

    // map
    {
        let mapped = AccState::map(all, |v| Val(v.0 + 1));
        assert!(
            mapped
                .iter()
                .map(|v| v.value.0)
                .eq([1, 2, 11, 2, 21, 31, 21, 31])
        );
    }

    // map empty
    {
        let lhs = All::create_state::<Val, NoLoc>();
        let mapped = AccState::map(lhs, |v| Val(v.0 + 1));
        assert!(mapped.is_empty());
    }
}

#[test]
fn test_contextuals_most() {
    let mut ctxs = Contextuals::new(trisult::Most::create_state::<Val, NoLoc>());

    // display empty
    {
        assert!(ctxs.to_string().is_empty());
    }

    // push/append
    {
        ctxs.push_naive(Contextual::new(NoLoc, Val(1)));
        assert_eq!(ctxs.ignored(), 0);

        ctxs.push_naive(Contextual::new(NoLoc, Val(2))); // ignored
        assert_eq!(ctxs.ignored(), 1);
        assert!(ctxs.iter().map(|d| d.value.0).eq([1]));

        ctxs.push(Contextual::new(NoLoc, Val(0))); // ignored priority
        assert_eq!(ctxs.ignored(), 2);
        assert!(ctxs.iter().map(|d| d.value.0).eq([1]));

        ctxs.append_naive(ctxs.clone()); // ignored
        assert_eq!(ctxs.ignored(), 5);
        assert!(ctxs.iter().map(|d| d.value.0).eq([1]));

        ctxs.append(ctxs.clone()); // ignored
        assert_eq!(ctxs.ignored(), 11);
        assert!(ctxs.iter().map(|d| d.value.0).eq([1]));

        ctxs.push(Contextual::new(NoLoc, Val(10))); // replaced
        assert_eq!(ctxs.ignored(), 12);
        assert!(ctxs.iter().map(|d| d.value.0).eq([10]));
    }

    // display
    {
        // TODO: maybe there will be better way to test display
        assert!(ctxs.to_string().contains("ignored"));
    }

    // map
    {
        let mapped = ctxs.clone().map(|v| Val(v.0 + 1));
        assert!(mapped.iter().map(|d| d.value.0).eq([11]));
    }

    // extend
    {
        ctxs.extend(vec![Contextual::new(NoLoc, Val(20))]); // replaced
        assert!(ctxs.iter().map(|d| d.value.0).eq([20]));
    }

    // into_iter
    {
        assert!((&ctxs).into_iter().map(|d| d.value.0).eq([20]));
        assert!(ctxs.into_iter().map(|d| d.value.0).eq([20]));
    }
}

#[test]
fn test_contextuals_all() {
    let mut ctxs = Contextuals::new(trisult::All::create_state::<Val, NoLoc>());

    // display empty
    {
        assert!(ctxs.to_string().is_empty());
    }

    // push/append
    {
        ctxs.reserve(5);
        ctxs.push_naive(Contextual::new(NoLoc, Val(1)));
        ctxs.push(Contextual::new(NoLoc, Val(2)));

        assert_eq!(ctxs.len(), 2);
        assert_eq!(ctxs.ignored(), 0);

        let mut ctxs2 = Contextuals::new(trisult::AllState::new());
        ctxs2.push_naive(Contextual::new(NoLoc, Val(3)));
        ctxs.append_naive(ctxs2.clone());
        ctxs.append(ctxs2.clone());

        assert_eq!(ctxs.len(), 4);
        assert_eq!(ctxs.ignored(), 0);
    }

    // display
    {
        // TODO: maybe there will be better way to test display
        assert!(!ctxs.to_string().is_empty());
    }

    // map
    {
        let mapped: Vec<_> = ctxs
            .clone()
            .map(|v| Val(v.0 + 1))
            .into_iter()
            .map(|d| d.value.0)
            .collect();
        assert_eq!(mapped, &[2, 3, 4, 4]);
    }

    // extend
    {
        ctxs.extend(vec![Contextual::new(NoLoc, Val(10))]);
        assert!(ctxs.iter().map(|d| d.value.0).eq([1, 2, 3, 3, 10]));
    }

    // into_iter
    {
        assert!((&ctxs).into_iter().map(|d| d.value.0).eq([1, 2, 3, 3, 10]));
        assert!(ctxs.into_iter().map(|d| d.value.0).eq([1, 2, 3, 3, 10]));
    }
}

#[test]
fn test_diagnosis_warning() {
    let dw = Diagnosis::<W, E>::Warning(W(1));

    // conversions
    {
        assert_eq!(dw.as_error(), None);
        assert_eq!(dw.as_warning(), Some(&W(1)));
        assert_eq!(dw.clone().into_error(), None);
        assert_eq!(dw.clone().into_warning(), Some(W(1)));

        assert_eq!(dw.to_string(), "warning: W(1)");
        assert_eq!(dw.source().unwrap().to_string(), "W(1)");
    }

    // map
    {
        let mapped = dw.clone().map_diagnosis(|w| W(w.0 + 1), |e| E(e.0 + 2));
        assert_eq!(mapped.as_warning(), Some(&W(2)));
    }

    let cw = ContextualDiagnosis::new(NoLoc, dw.clone());

    // conversions
    {
        assert_eq!(cw.as_error(), None);
        assert_eq!(cw.as_warning(), Some(Contextual::new(&NoLoc, &W(1))));
        assert_eq!(cw.source().unwrap().to_string(), "W(1)");
    }

    // map
    {
        let mapped = cw.clone().map_diagnosis(|w| W(w.0 + 1), |e| E(e.0 + 2));
        assert_eq!(mapped.as_warning(), Some(Contextual::new(&NoLoc, &W(2))));
    }

    // append warnings
    {
        let mut diags = Diagnoses::new(All::create_state());
        diags.push(cw.clone());

        let mut diags_other = Diagnoses::new(All::create_state());
        diags_other.push(cw.clone());

        diags.append_warnings(diags_other.unwrap_as_warnings());
        assert_eq!(diags.len(), 2);
    }

    // append with empty
    {
        let mut diags = Diagnoses::<W, E>::new(All::create_state());
        let empty_other = Contextuals::new(All::create_state());
        diags.append_warnings(empty_other);

        let mapped_diags = diags.map_diagnosis(|w| W(w.0 + 1), |e| E(e.0 + 1));
        assert!(mapped_diags.is_empty());
    }
}

#[test]
fn test_diagnosis_error() {
    let de = Diagnosis::<W, E>::Error(E(2));

    // conversions
    {
        assert_eq!(de.as_error(), Some(&E(2)));
        assert_eq!(de.as_warning(), None);
        assert_eq!(de.clone().into_error(), Some(E(2)));
        assert_eq!(de.clone().into_warning(), None);

        assert_eq!(de.to_string(), "error: E(2)");
        assert_eq!(de.source().unwrap().to_string(), "E(2)");
    }

    // map
    {
        let mapped = de.clone().map_diagnosis(|w| W(w.0 + 1), |e| E(e.0 + 2));
        assert_eq!(mapped.as_error(), Some(&E(4)));
    }

    let cw = ContextualDiagnosis::new(NoLoc, de.clone());

    // conversions
    {
        assert_eq!(cw.as_error(), Some(Contextual::new(&NoLoc, &E(2))));
        assert_eq!(cw.as_warning(), None);

        assert_eq!(cw.source().unwrap().to_string(), "E(2)");
    }

    // map
    {
        let mapped = cw.clone().map_diagnosis(|w| W(w.0 + 1), |e| E(e.0 + 2));
        assert_eq!(mapped.as_error(), Some(Contextual::new(&NoLoc, &E(4))));
    }
}

#[test]
fn test_trisult_exhaustive() {
    let mut ok_diags = Diagnoses::new(trisult::AllState::new());
    ok_diags.push(Contextual::new(NoLoc, Diagnosis::<W, E>::Warning(W(1))));
    let ok: Trisult<i32, W, E, NoLoc, trisult::AllState<Diagnosis<W, E>, NoLoc>> =
        Trisult::Ok(Diagnosed(10, ok_diags.unwrap_as_warnings()));

    let mut err_diags = Diagnoses::new(trisult::AllState::new());
    err_diags.push(Contextual::new(NoLoc, Diagnosis::<W, E>::Error(E(2))));
    let err: Trisult<i32, W, E, NoLoc, trisult::AllState<Diagnosis<W, E>, NoLoc>> =
        Trisult::Err(err_diags);

    // is_ok, is_err
    assert!(ok.is_ok());
    assert!(err.is_err());

    // Debug
    assert!(format!("{:?}", ok).contains("Ok"));
    assert!(format!("{:?}", err).contains("Err"));

    // Clone
    let ok_cloned = ok.clone();
    let err_cloned = err.clone();

    // unpack
    let (v, _) = ok_cloned.unpack();
    assert_eq!(v, Some(10));
    let (v, _) = err_cloned.unpack();
    assert_eq!(v, None);

    // ok, err
    assert!(ok.clone().ok().is_some());
    assert!(ok.clone().err().is_none());
    assert!(err.clone().ok().is_none());
    assert!(err.clone().err().is_some());

    // expect & unwrap on ok
    assert_eq!(ok.clone().expect("Should be ok").0, 10);
    assert_eq!(ok.clone().unwrap().0, 10);

    // and_then combinations
    let ok_to_ok = ok
        .clone()
        .and_then(|x| Trisult::Ok(Diagnosed(x + 1, Contextuals::new(trisult::AllState::new()))));
    assert!(ok_to_ok.is_ok());

    let ok_empty = Trisult::<i32, W, E, NoLoc, trisult::AllState<Diagnosis<W, E>, NoLoc>>::Ok(
        Diagnosed(5, Contextuals::new(trisult::AllState::new())),
    );
    let ok_empty_to_ok_empty = ok_empty
        .clone()
        .and_then(|x| Trisult::Ok(Diagnosed(x + 1, Contextuals::new(trisult::AllState::new()))));
    assert!(ok_empty_to_ok_empty.is_ok());

    let ok_empty_to_ok_warns = ok_empty.clone().and_then(|x| {
        Trisult::Ok(Diagnosed(x + 1, {
            let mut d = Diagnoses::new(trisult::AllState::new());
            d.push(Contextual::new(NoLoc, Diagnosis::<W, E>::Warning(W(1))));
            d.unwrap_as_warnings()
        }))
    });
    assert!(ok_empty_to_ok_warns.is_ok());

    let ok_empty_to_err = ok_empty.clone().and_then(|_| err.clone());
    assert!(ok_empty_to_err.is_err());

    let err_to_ok = err
        .clone()
        .and_then(|x| Trisult::Ok(Diagnosed(x + 1, Contextuals::new(trisult::AllState::new()))));
    assert!(err_to_ok.is_err());

    // map combinations
    let mapped_ok = ok.clone().map(|x| x + 1);
    assert!(mapped_ok.is_ok());
    let mapped_err = err.clone().map(|x| x + 1);
    assert!(mapped_err.is_err());

    let map_diag_ok = ok.clone().map_diagnosis(|w| W(w.0 + 1), |e| E(e.0 + 1));
    assert!(map_diag_ok.is_ok());
    let map_diag_err = err.clone().map_diagnosis(|w| W(w.0 + 1), |e| E(e.0 + 1));
    assert!(map_diag_err.is_err());

    // into Result
    let res_ok: Result<_, _> = ok.clone().into();
    assert!(res_ok.is_ok());
    let res_err: Result<_, _> = err.clone().into();
    assert!(res_err.is_err());
}

#[test]
#[should_panic]
fn test_trisult_expect_panic() {
    let mut err_diags = Diagnoses::new(trisult::AllState::new());
    err_diags.push(Contextual::new(NoLoc, Diagnosis::<W, E>::Error(E(2))));
    let err: Trisult<i32, W, E, NoLoc, trisult::AllState<Diagnosis<W, E>, NoLoc>> =
        Trisult::Err(err_diags);
    err.expect("Expected panic");
}

#[test]
#[should_panic]
fn test_trisult_unwrap_panic() {
    let mut err_diags = Diagnoses::new(trisult::AllState::new());
    err_diags.push(Contextual::new(NoLoc, Diagnosis::<W, E>::Error(E(2))));
    let err: Trisult<i32, W, E, NoLoc, trisult::AllState<Diagnosis<W, E>, NoLoc>> =
        Trisult::Err(err_diags);
    err.unwrap();
}

#[test]
#[should_panic]
fn test_unwrap_as_warnings_panic() {
    let mut err_diags = Diagnoses::new(trisult::AllState::new());
    err_diags.push(Contextual::new(NoLoc, Diagnosis::<W, E>::Error(E(2))));
    err_diags.unwrap_as_warnings();
}

#[test]
fn test_iterators() {
    let mut err_diags = Diagnoses::new(trisult::AllState::new());
    err_diags.push(Contextual::new(NoLoc, Diagnosis::<W, E>::Warning(W(1))));

    let diag_iter = err_diags.clone().into_iter();
    assert_eq!(diag_iter.len(), 1);
    let mut mapped_iter = diag_iter
        .map_errors(|e: E| E(e.0))
        .map_warnings(|w: W| W(w.0));
    assert_eq!(mapped_iter.len(), 1);
    assert_eq!(mapped_iter.size_hint(), (1, Some(1)));

    assert!(mapped_iter.next().is_some());
    assert!(mapped_iter.next().is_none());

    let mut ref_iter = err_diags.iter();
    assert_eq!(ref_iter.len(), 1);
    assert_eq!(ref_iter.size_hint(), (1, Some(1)));
    assert!(ref_iter.next().is_some());
    assert!(ref_iter.next().is_none());
}

#[test]
fn test_tri_unpack_with_existing_diags_append() {
    use trisult::Acc;
    let mut diags = Diagnoses::new(All::create_state());
    let mut has_errors = true; // Simulating we already have an error

    let err: Trisult<i32, W, E, NoLoc, trisult::AllState<Diagnosis<W, E>, NoLoc>> = Trisult::Err({
        let mut d = Diagnoses::new(All::create_state());
        d.push(Contextual::new(NoLoc, Diagnosis::<W, E>::Error(E(2))));
        d
    });

    // Test the `if diags.is_empty() { *diags = err; } else { diags.append(err); }` branch
    diags.push(Contextual::new(NoLoc, Diagnosis::<W, E>::Error(E(1))));
    err.__macro_tri_unpack(&mut diags, &mut has_errors);
    assert_eq!(diags.into_iter().count(), 2);
}
