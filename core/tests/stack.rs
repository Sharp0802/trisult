mod shared;

use crate::shared::mock::{MockErr, MockResult, MockWarn, TraceStack};
use trisult::{ContextStackMut, Diagnosed, Diagnosis, Trisult, trisult};

#[trisult(segment = "happy_node")]
fn happy_function(#[context] stack: &mut TraceStack) -> MockResult<i32> {
    warn!(MockWarn::MinorIssue);
    Some(42)
}

#[test]
fn test_successful_push_and_pop() {
    let mut stack = TraceStack::default();
    let res = happy_function(&mut stack);

    assert!(stack.path.is_empty(), "stack leaked out of the function!");

    if let Trisult::Ok(Diagnosed(val, diags)) = res {
        assert_eq!(val, 42);
        let warnings: Vec<_> = diags.into_iter().collect();
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].context, "/happy_node");
    } else {
        panic!("expected Ok");
    }
}

#[trisult(segment = "fatal_node")]
fn early_return_function(#[context] stack: &mut TraceStack) -> MockResult<()> {
    error!(MockErr::FatalIssue);
    return None;
}

#[test]
fn test_early_return_stack_cleanup() {
    let mut stack = TraceStack::default();
    let res = early_return_function(&mut stack);

    assert!(stack.path.is_empty(), "early return caused a stack leak!");

    if let Trisult::Err(diags) = res {
        let errors: Vec<_> = diags.into_iter().collect();
        assert_eq!(errors[0].context, "/fatal_node");
    } else {
        panic!("expected Err");
    }
}

#[trisult(segment = "sibling_1")]
fn sibling_one(#[context] stack: &mut TraceStack) -> MockResult<()> {
    warn!(MockWarn::MinorIssue);
    Some(())
}

#[trisult(segment = "sibling_2")]
fn sibling_two(#[context] stack: &mut TraceStack) -> MockResult<()> {
    error!(MockErr::FatalIssue);
    None
}

#[trisult(segment = "parent")]
fn parent_function(#[context] stack: &mut TraceStack) -> MockResult<()> {
    tri!(sibling_one(stack))?;
    tri!(sibling_two(stack))?;
    Some(())
}

#[test]
fn test_sibling_isolation() {
    let mut stack = TraceStack::default();
    let res = parent_function(&mut stack);

    assert!(stack.path.is_empty(), "parent function leaked the stack!");

    if let Trisult::Err(diags) = res {
        let trace: Vec<_> = diags.into_iter().collect();
        assert_eq!(trace.len(), 2, "expected 1 warning and 1 error");

        assert_eq!(trace[0].context, "/parent/sibling_1");
        assert!(matches!(
            trace[0].value,
            Diagnosis::Warning(MockWarn::MinorIssue)
        ));

        assert_eq!(trace[1].context, "/parent/sibling_2");
        assert!(matches!(
            trace[1].value,
            Diagnosis::Error(MockErr::FatalIssue)
        ));
    } else {
        panic!("expected Err");
    }
}

#[trisult]
fn unstacked_function(#[context] _stack: &mut TraceStack) -> MockResult<()> {
    warn!(MockWarn::MinorIssue);
    Some(())
}

#[test]
fn test_opt_out_stacking() {
    let mut stack = TraceStack::default();
    stack.push("existing_parent");

    let res = unstacked_function(&mut stack);

    assert_eq!(stack.path, vec!["existing_parent"]);

    if let Trisult::Ok(Diagnosed(_, diags)) = res {
        let trace: Vec<_> = diags.into_iter().collect();
        assert_eq!(trace[0].context, "/existing_parent");
    } else {
        panic!("expected Ok");
    }
}
