#![cfg(feature = "alloc")]

use std::error::Error as StdError;
use std::fmt::{Display, Formatter};
use trisult::{
    trisult, Acc, All, ContextStack, ContextStackMut, Contextual, Diagnosed, Diagnoses,
    Diagnosis, MapDiagnosis, Most, NoLoc, Prioritized, Severity, Trisult,
};

#[derive(Debug, Clone, PartialEq)]
pub enum HealthWarn {
    HighLatency(u32),
    OldVersion(String),
}
impl Display for HealthWarn {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Latency: {:?}", self)
    }
}
impl StdError for HealthWarn {}

impl Prioritized for HealthWarn {
    type Priority = Severity;

    fn priority(&self) -> Self::Priority {
        Severity::Warning
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum HealthErr {
    ConnectionRefused,
    Timeout,
}
impl Display for HealthErr {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Err: {:?}", self)
    }
}
impl StdError for HealthErr {}

impl Prioritized for HealthErr {
    type Priority = Severity;

    fn priority(&self) -> Self::Priority {
        Severity::Error
    }
}

#[derive(Debug, Clone, Default)]
pub struct NodeContext {
    pub node_id: String,
}
impl ContextStack for NodeContext {
    type Captured = String;
}
impl ContextStackMut for NodeContext {
    type Segment = String;
    fn capture(&self) -> String {
        self.node_id.clone()
    }
    fn push(&mut self, seg: String) {
        self.node_id = seg;
    }
    fn pop(&mut self) {}
}

pub type HealthResult<T, A> = Trisult<
    T,
    HealthWarn,
    HealthErr,
    String,
    <A as Acc>::Acc<Diagnosis<HealthWarn, HealthErr>, String>,
>;

#[trisult(segment = node_name.to_string())]
fn check_node<#[kind] A: Acc>(
    node_name: &str,
    latency: u32,
    version: &str,
    is_down: bool,
    #[context] ctx: &mut NodeContext,
) -> HealthResult<bool, A> {
    if is_down {
        error!(HealthErr::ConnectionRefused);
        error!(HealthErr::Timeout);
        return None;
    }

    if latency > 100 {
        warn!(HealthWarn::HighLatency(latency));
    }
    if version == "1.0" {
        warn!(HealthWarn::OldVersion(version.to_string()));
    }

    Some(true)
}

#[trisult]
fn parent_check<#[kind] A: Acc>(#[context] ctx: &mut NodeContext) -> HealthResult<bool, A> {
    // Generate a warning first
    warn!(HealthWarn::HighLatency(500));

    // Now call an erroring function with tri! to hit the branch that appends an error to existing diags.
    let b = tri!(check_node::<A>("fatal", 0, "1.0", true, ctx))?;
    Some(b)
}

#[test]
fn run_health_checks() {
    let mut ctx = NodeContext::default();

    // 1. Test All Allocator with OK result
    let res = check_node::<All>("node1", 150, "1.0", false, &mut ctx);

    // Test is_ok, is_err
    assert!(res.is_ok());
    assert!(!res.is_err());

    // Test expect and clone
    let Diagnosed(is_healthy, warns) = res.clone().expect("Node should be healthy");
    assert!(is_healthy);

    // Test Contextual `as_ref` and formatting
    for w in warns.iter() {
        let r = w.as_ref();
        assert!(format!("{}", r).contains("node1"));
    }

    let mut more_warns = warns.clone();
    more_warns.append_naive(warns.clone()); // Appending naively
    assert_eq!(more_warns.len(), 4);

    let mut extended_warns = warns.clone();
    extended_warns.extend(warns.clone());
    assert_eq!(extended_warns.len(), 4);

    // 2. Test Most Allocator with ERR result
    let res_err = check_node::<Most>("node2", 150, "1.0", true, &mut ctx);
    assert!(!res_err.is_ok());
    assert!(res_err.is_err());

    // Test err(), ok(), unpack, into() Result
    assert!(res_err.clone().ok().is_none());
    let errs = res_err.clone().err().unwrap();

    let (v, diags) = res_err.clone().unpack();
    assert!(v.is_none());
    assert_eq!(diags.len(), 1); // Only Most

    let res_enum: Result<_, _> = res_err.clone().into();
    assert!(res_enum.is_err());

    // Test Display of Diagnoses with ignored items
    let fmt_most = format!("{}", errs);
    assert!(fmt_most.contains("ignored"));

    // Map Iter via traits
    let iter = errs.clone().into_iter();
    let mut mapped_iter = iter.map_errors(|_| HealthErr::Timeout).map_warnings(|w| w);

    assert_eq!(mapped_iter.size_hint(), (1, Some(1)));
    let next_diag = mapped_iter.next().unwrap();

    // Diagnosis utility methods
    assert!(next_diag.value.as_error().is_some());
    assert!(next_diag.value.as_warning().is_none());
    assert!(next_diag.as_error().is_some());
    assert!(next_diag.as_warning().is_none());

    // 3. Test map and and_then in Trisult
    let ok_mapped = res.clone().map(|b| !b);
    assert!(!ok_mapped.unwrap().0);

    let res_no_warns = check_node::<All>("node1", 0, "2.0", false, &mut ctx);
    let res_err_all = check_node::<All>("node3", 150, "1.0", true, &mut ctx);

    // and_then combinations
    // ok (empty) -> ok (empty)
    let _ = res_no_warns.clone().and_then(|_| res_no_warns.clone());
    // ok (empty) -> ok (warns)
    let _ = res_no_warns.clone().and_then(|_| res.clone());
    // ok (empty) -> err
    let _ = res_no_warns.clone().and_then(|_| res_err_all.clone());
    // ok (warns) -> ok (warns)
    let _ = res.clone().and_then(|_| res.clone());
    // ok (warns) -> err
    let err_chained = res.clone().and_then(|_| res_err_all.clone());
    assert!(err_chained.is_err());
    // err -> ...
    let _ = res_err_all.clone().and_then(|_| res.clone());

    let ok_chained = res.clone().and_then(|_| {
        let mut d = Diagnoses::new(All::create_state());
        d.push(Contextual::new(
            "node1".to_string(),
            Diagnosis::<HealthWarn, HealthErr>::Warning(HealthWarn::HighLatency(200)),
        ));
        Trisult::Ok(Diagnosed(true, d.unwrap_as_warnings()))
    });
    let Diagnosed(_, chained_warns) = ok_chained.unwrap();
    assert_eq!(chained_warns.len(), 3);

    let mapped_trisult = res
        .clone()
        .map_diagnosis(|_| HealthWarn::HighLatency(0), |_| HealthErr::Timeout);
    assert!(mapped_trisult.is_ok());

    let res_mapped = res.clone().map_errors(|_| HealthErr::Timeout);
    assert!(res_mapped.is_ok());

    // 4. Test ContextStack default implementations via NoLoc
    let no_loc = NoLoc;
    assert_eq!(format!("{}", no_loc), "no-location");
    assert_eq!(no_loc.capture(), NoLoc);
    let mut loc_mut = NoLoc;
    loc_mut.push(());
    loc_mut.pop();

    fn accepts_stack<S: ContextStack>(_: S) {}
    accepts_stack(&no_loc);

    let diag_err = Diagnosis::<HealthWarn, HealthErr>::Error(HealthErr::Timeout);
    assert!(diag_err.clone().into_error().is_some());
    assert!(diag_err.into_warning().is_none());

    let diag_warn = Diagnosis::<HealthWarn, HealthErr>::Warning(HealthWarn::HighLatency(10));
    assert!(diag_warn.clone().into_warning().is_some());
    assert!(diag_warn.into_error().is_none());

    // Debug representations
    let ok_dbg = format!("{:?}", res);
    assert!(ok_dbg.contains("Ok"));
    let err_dbg = format!("{:?}", res_err);
    assert!(err_dbg.contains("Err"));

    // 5. Test Contextuals push naive vs push priority
    let mut all_acc = trisult::Contextuals::new(trisult::AllState::new());
    all_acc.push_naive(Contextual::new(NoLoc, Diagnosis::Error(HealthErr::Timeout)));
    all_acc.push(Contextual::new(
        NoLoc,
        Diagnosis::Warning(HealthWarn::HighLatency(10)),
    ));

    // Test that mapped_trisult works with mapped warnings
    let res_mapped_warns = res.clone().map_warnings(|_| HealthWarn::HighLatency(0));
    assert!(res_mapped_warns.is_ok());
}

#[test]
fn test_macro_tri_unpack_with_existing_diags() {
    let mut ctx = NodeContext::default();
    let res = parent_check::<All>(&mut ctx);
    assert!(res.is_err());
    let errs = res.err().unwrap();
    assert_eq!(errs.len(), 3); // 1 from warn!, 2 from check_node (ConnectionRefused, Timeout)
}

#[test]
#[should_panic(expected = "called `Trisult::unwrap()` on an `Err` value")]
fn test_trisult_unwrap_panics_on_err() {
    let mut ctx = NodeContext::default();
    let res = check_node::<All>("fatal_node", 0, "1.0", true, &mut ctx);
    res.unwrap();
}

#[test]
#[should_panic]
fn test_force_unwrap_warnings_panics_on_fatal() {
    let mut ctx = NodeContext::default();
    let res = check_node::<All>("fatal_node", 0, "1.0", true, &mut ctx);
    let errs = res.err().unwrap();
    let _ = errs.unwrap_as_warnings();
}
