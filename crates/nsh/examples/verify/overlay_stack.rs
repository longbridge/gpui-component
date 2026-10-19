//! Scoped sheet handles preserve the focus chain below a live host dialog.
use gpui_kit::component::WindowExt;
use gpui_kit::{
    AnyWindowHandle, Entity, HeadlessAppContext, InteractiveElement, ParentElement, Styled, div,
    test::{TestQueryExt, TestWindowExt},
};
use gpui_kit_nsh::nsh::{self, AppView, Bundle};
use std::{cell::Cell, rc::Rc};

pub(super) fn run(cx: &mut HeadlessAppContext, handle: AnyWindowHandle, view: &Entity<AppView>) {
    cx.update(|cx|view.read(cx).load(nsh::App::Js(Bundle::source(r#"import {mount} from 'nsh';import {nativeComponent} from 'nsh/native';const Button=nativeComponent('gpui-component.Button',{props:['label'],events:['press']});mount(()=>{let button;button=Button({label:'Sheet background',style:{width:180,height:36},onPress:()=>button.command('focus',null)});return button;});"#.to_owned()))));
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        cx.run_until_parked();
        let ready = cx
            .update_window(handle, |_, window, cx| {
                window.render_frame(cx);
                window.try_find_by_label("Sheet background").is_some()
            })
            .unwrap();
        if ready {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "background did not mount"
        );
        std::thread::yield_now();
    }
    crate::verification::click(cx, handle, "Sheet background");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        cx.run_until_parked();
        let focused = cx
            .update_window(handle, |_, window, cx| {
                window.render_frame(cx);
                window.find_by_label("Sheet background").focused() == Some(true)
            })
            .unwrap();
        if focused {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "newly mounted background button did not receive real focus"
        );
        std::thread::yield_now();
    }
    let closed_a = Rc::new(Cell::new(0));
    let closed_b = Rc::new(Cell::new(0));
    let (a, b, dialog, _a_subscription, _b_subscription, background, dialog_focus, b_marker) = cx
        .update_window(handle, |_, window, cx| {
            let background = window.focused(cx).expect("background button has focus");
            let a = window.open_sheet_at_with_handle(
                gpui_kit::component::Placement::Right,
                cx,
                |sheet, _, _| sheet.title("Sheet A"),
            );
            let a_count = closed_a.clone();
            let a_subscription =
                a.on_closed(window, cx, move |_, _| a_count.set(a_count.get() + 1));
            let dialog =
                window.open_dialog_with_handle(cx, |dialog, _, _| dialog.title("Host dialog"));
            let dialog_focus = window.focused(cx).expect("dialog received focus");
            let b_marker = cx.focus_handle();
            let marker = b_marker.clone();
            let b = window.open_sheet_at_with_handle(
                gpui_kit::component::Placement::Left,
                cx,
                move |sheet, _, _| {
                    sheet
                        .title("Sheet B")
                        .child(div().id("sheet-b-marker").size_1().track_focus(&marker))
                },
            );
            assert_eq!(
                window.focused(cx),
                Some(dialog_focus.clone()),
                "replacement must preserve the covering dialog's focus"
            );
            assert!(!a.is_open(window, cx));
            assert!(b.is_open(window, cx));
            assert!(dialog.is_open(window, cx));
            a.close(window, cx);
            assert!(b.is_open(window, cx), "stale sheet A must not close B");
            let b_count = closed_b.clone();
            let b_subscription =
                b.on_closed(window, cx, move |_, _| b_count.set(b_count.get() + 1));
            (
                a,
                b,
                dialog,
                a_subscription,
                b_subscription,
                background,
                dialog_focus,
                b_marker,
            )
        })
        .unwrap();
    cx.run_until_parked();
    assert_eq!(closed_a.get(), 1, "replacement emits one logical removal");
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        assert_eq!(window.focused(cx), Some(dialog_focus));
        dialog.close(window, cx);
        assert!(!dialog.is_open(window, cx));
        assert!(b.is_open(window, cx));
        assert_ne!(
            window.focused(cx),
            Some(background.clone()),
            "closing D returns to B before the background"
        );
        assert!(
            window
                .focused(cx)
                .is_some_and(|focus| focus.contains(&b_marker, window)),
            "D must restore actual B, not the removed A focus"
        );
        b.close(window, cx);
        b.close(window, cx);
        a.close(window, cx);
        assert_eq!(
            window.focused(cx),
            Some(background.clone()),
            "closing B returns to the original background owner"
        );
    })
    .unwrap();
    cx.run_until_parked();
    assert_eq!(closed_a.get(), 1);
    assert_eq!(closed_b.get(), 1, "scoped close remains idempotent");

    cx.update_window(handle, |_, window, cx| {
        window.focus(&background, cx);
        let dialog =
            window.open_dialog_with_handle(cx, |dialog, _, _| dialog.title("Dialog before sheet"));
        let dialog_focus = window.focused(cx);
        let marker = cx.focus_handle();
        let marker_builder = marker.clone();
        let sheet = window.open_sheet_at_with_handle(
            gpui_kit::component::Placement::Right,
            cx,
            move |sheet, _, _| {
                sheet.title("New sheet under dialog").child(
                    div()
                        .id("new-sheet-marker")
                        .size_1()
                        .track_focus(&marker_builder),
                )
            },
        );
        assert_eq!(
            window.focused(cx),
            dialog_focus,
            "first sheet must not steal dialog focus"
        );
        window.render_frame(cx);
        dialog.close(window, cx);
        assert!(
            window
                .focused(cx)
                .is_some_and(|focus| focus.contains(&marker, window)),
            "dialog must restore the newly inserted sheet"
        );
        assert_ne!(
            window.focused(cx),
            Some(background.clone()),
            "dialog returns to its new sheet"
        );
        sheet.close(window, cx);
        assert_eq!(
            window.focused(cx),
            Some(background.clone()),
            "new sheet must not restore the removed dialog"
        );
    })
    .unwrap();
    // Confirm follows the real native UI action path and schedules focus restoration.
    for remove_opener in [false, true] {
        let (a, b, a_focus) = cx
            .update_window(handle, |_, window, cx| {
                window.focus(&background, cx);
                let a = window
                    .open_dialog_with_handle(cx, |dialog, _, _| dialog.title("Deferred opener A"));
                let a_focus = window.focused(cx).unwrap();
                let b = window.open_dialog_with_handle(cx, |dialog, _, _| {
                    dialog.title("Deferred closing B").footer(
                        gpui_kit::component::dialog::DialogAction::new().child(
                            gpui_kit::component::button::Button::new("confirm-deferred-b")
                                .label("Confirm deferred B"),
                        ),
                    )
                });
                window.render_frame(cx);
                (a, b, a_focus)
            })
            .unwrap();
        crate::verification::click(cx, handle, "Confirm deferred B");
        let (c, c_focus) = cx
            .update_window(handle, |_, window, cx| {
                assert!(
                    !b.is_open(window, cx),
                    "real Confirm must close B before its timer"
                );
                if remove_opener {
                    a.close(window, cx);
                }
                let c = window
                    .open_dialog_with_handle(cx, |dialog, _, _| dialog.title("Replacement host C"));
                let focus = window.focused(cx).unwrap();
                window.render_frame(cx);
                (c, focus)
            })
            .unwrap();
        cx.advance_clock(std::time::Duration::from_secs(1));
        cx.run_until_parked();
        cx.update_window(handle, |_, window, cx| {
            assert!(c.is_open(window, cx));
            assert_eq!(
                window.focused(cx),
                Some(c_focus),
                "obsolete timer must not steal C focus"
            );
            c.close(window, cx);
            assert_eq!(
                window.focused(cx),
                Some(if remove_opener {
                    background.clone()
                } else {
                    a_focus
                }),
                "replacement inherits the live opener, never a removed dialog"
            );
            a.close(window, cx);
            assert_eq!(window.focused(cx), Some(background.clone()));
        })
        .unwrap();
    }
    println!(
        "verified sheet replacement below host dialogs, focus restoration, stale handles and single close observations"
    );
}
