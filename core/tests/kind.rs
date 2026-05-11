mod shared;

use crate::shared::mock::{MockErr, MockResult, MockWarn, TraceStack};
use trisult::{trisult, AccumulatorKind, Diagnosed, Diagnosis, Trisult};

#[trisult(segment = "warn_node")]
fn multiple_warnings(
    #[kind] kind: AccumulatorKind,
    #[context] stack: &mut TraceStack,
) -> MockResult<i32> {
    warn!(MockWarn::MinorIssue);
    warn!(MockWarn::MinorIssue);
    Some(42)
}

#[test]
fn test_dynamic_kind_all_warnings() {
    let mut stack = TraceStack::default();

    let res = multiple_warnings(AccumulatorKind::All, &mut stack);

    if let Trisult::Ok(Diagnosed(val, diags)) = res {
        assert_eq!(val, 42);
        let warnings: Vec<_> = diags.into_iter().collect();
        assert_eq!(warnings.len(), 2, "expected 2 warnings with AccumulatorKind::All");
        assert_eq!(warnings[0].context, "/warn_node");
        assert_eq!(warnings[1].context, "/warn_node");
    } else {
        panic!("expected Ok");
    }
}

#[test]
fn test_dynamic_kind_most_warnings() {
    let mut stack = TraceStack::default();

    let res = multiple_warnings(AccumulatorKind::Most, &mut stack);

    if let Trisult::Ok(Diagnosed(val, diags)) = res {
        assert_eq!(val, 42);
        let warnings: Vec<_> = diags.into_iter().collect();
        assert_eq!(warnings.len(), 1, "expected 1 warning with AccumulatorKind::Most");
    } else {
        panic!("expected Ok");
    }
}

#[trisult(segment = "mixed_node")]
fn warning_then_error(
    #[kind] kind: AccumulatorKind,
    #[context] stack: &mut TraceStack,
) -> MockResult<()> {
    warn!(MockWarn::MinorIssue);
    error!(MockErr::FatalIssue);
    None
}

#[test]
fn test_dynamic_kind_all_mixed() {
    let mut stack = TraceStack::default();

    let res = warning_then_error(AccumulatorKind::All, &mut stack);

    if let Trisult::Err(diags) = res {
        let trace: Vec<_> = diags.into_iter().collect();
        assert_eq!(trace.len(), 2, "expected 2 diagnostics with AccumulatorKind::All");

        assert!(matches!(trace[0].value, Diagnosis::Warning(MockWarn::MinorIssue)));
        assert!(matches!(trace[1].value, Diagnosis::Error(MockErr::FatalIssue)));
    } else {
        panic!("expected Err");
    }
}

#[test]
fn test_dynamic_kind_most_mixed_upgrades_priority() {
    let mut stack = TraceStack::default();

    let res = warning_then_error(AccumulatorKind::Most, &mut stack);

    if let Trisult::Err(diags) = res {
        let trace: Vec<_> = diags.into_iter().collect();
        assert_eq!(trace.len(), 1, "expected exactly 1 diagnostic with AccumulatorKind::Most");

        assert!(matches!(trace[0].value, Diagnosis::Error(MockErr::FatalIssue)));
        assert_eq!(trace[0].context, "/mixed_node");
    } else {
        panic!("expected Err");
    }
}

#[trisult]
fn default_kind_func(#[context] _stack: &mut TraceStack) -> MockResult<()> {
    warn!(MockWarn::MinorIssue);
    warn!(MockWarn::MinorIssue);
    Some(())
}

#[test]
fn test_default_kind_is_all() {
    let mut stack = TraceStack::default();

    let res = default_kind_func(&mut stack);

    if let Trisult::Ok(Diagnosed(_, diags)) = res {
        let warnings: Vec<_> = diags.into_iter().collect();
        assert_eq!(warnings.len(), 2, "fallback should default to AccumulatorKind::All");
    } else {
        panic!("expected Ok");
    }
}
