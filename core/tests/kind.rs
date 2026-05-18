mod shared;

use crate::shared::mock::{MockErr, MockWarn, TraceStack};
use trisult::{trisult, Acc, Default, Diagnosed, Diagnosis, Most, Trisult};

#[cfg(feature = "alloc")]
use trisult::All;

#[allow(type_alias_bounds)]
pub type MockResult<T, A: Acc = Default> = Trisult<T, MockWarn, MockErr, String, A>;

#[trisult(segment = "warn_node")]
fn multiple_warnings<#[kind] T: Acc>(#[context] stack: &mut TraceStack) -> MockResult<i32, T> {
    warn!(MockWarn::MinorIssue);
    warn!(MockWarn::MinorIssue);
    Some(42)
}

#[test]
#[cfg(feature = "alloc")]
fn test_dynamic_kind_all_warnings() {
    let mut stack = TraceStack::default();

    let res = multiple_warnings::<All>(&mut stack);

    if let Trisult::Ok(Diagnosed(val, diags)) = res {
        assert_eq!(val, 42);
        let warnings: Vec<_> = diags.into_iter().collect();
        assert_eq!(
            warnings.len(),
            2,
            "expected 2 warnings with All"
        );
        assert_eq!(warnings[0].context, "/warn_node");
        assert_eq!(warnings[1].context, "/warn_node");
    } else {
        panic!("expected Ok");
    }
}

#[test]
fn test_dynamic_kind_most_warnings() {
    let mut stack = TraceStack::default();

    let res = multiple_warnings::<Most>(&mut stack);

    if let Trisult::Ok(Diagnosed(val, diags)) = res {
        assert_eq!(val, 42);
        let warnings: Vec<_> = diags.into_iter().collect();
        assert_eq!(
            warnings.len(),
            1,
            "expected 1 warning with Most"
        );
    } else {
        panic!("expected Ok");
    }
}

#[trisult(segment = "mixed_node")]
fn warning_then_error<#[kind] T: Acc>(#[context] stack: &mut TraceStack) -> MockResult<(), T> {
    warn!(MockWarn::MinorIssue);
    error!(MockErr::FatalIssue);
    None
}

#[test]
#[cfg(feature = "alloc")]
fn test_dynamic_kind_all_mixed() {
    let mut stack = TraceStack::default();

    let res = warning_then_error::<All>(&mut stack);

    if let Trisult::Err(diags) = res {
        let trace: Vec<_> = diags.into_iter().collect();
        assert_eq!(
            trace.len(),
            2,
            "expected 2 diagnostics with All"
        );

        assert!(matches!(
            trace[0].value,
            Diagnosis::Warning(MockWarn::MinorIssue)
        ));
        assert!(matches!(
            trace[1].value,
            Diagnosis::Error(MockErr::FatalIssue)
        ));
    } else {
        panic!("expected Err");
    }
}

#[test]
fn test_dynamic_kind_most_mixed_upgrades_priority() {
    let mut stack = TraceStack::default();

    let res = warning_then_error::<Most>(&mut stack);

    if let Trisult::Err(diags) = res {
        let trace: Vec<_> = diags.into_iter().collect();
        assert_eq!(
            trace.len(),
            1,
            "expected exactly 1 diagnostic with Most"
        );

        assert!(matches!(
            trace[0].value,
            Diagnosis::Error(MockErr::FatalIssue)
        ));
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
        assert_eq!(
            warnings.len(),
            2,
            "fallback should default to All"
        );
    } else {
        panic!("expected Ok");
    }
}
