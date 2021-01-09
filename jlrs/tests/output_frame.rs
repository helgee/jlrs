use jlrs::prelude::*;
use jlrs::util::JULIA;

#[test]
fn nested_value_frame() {
    JULIA.with(|j| {
        let mut jlrs = j.borrow_mut();

        let out = jlrs.frame(1, |_global, frame| {
            frame
                .value_frame(0, |output, frame| {
                    output.into_scope(frame).value_frame(0, |output, frame| {
                        let output = output.into_scope(frame);
                        Value::new(output, 1usize)
                    })
                })?
                .cast::<usize>()
        });

        assert_eq!(out.unwrap(), 1);
    });
}

#[test]
fn nested_call_frame() {
    JULIA.with(|j| {
        let mut jlrs = j.borrow_mut();

        let out = jlrs.frame(1, |global, frame| {
            frame
                .call_frame(0, |output, frame| {
                    output.into_scope(frame).call_frame(2, |output, frame| {
                        let func = Module::base(global).function("+")?;
                        let v1 = Value::new(frame.as_scope(), 1usize)?;
                        let v2 = Value::new(frame.as_scope(), 2usize)?;
                        let output = output.into_scope(frame);
                        func.call2(output, v1, v2)
                    })
                })?
                .unwrap()
                .cast::<usize>()
        });

        assert_eq!(out.unwrap(), 3);
    });
}
