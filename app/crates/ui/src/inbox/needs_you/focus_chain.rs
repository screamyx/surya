//! E2E-KEY-01: the Needs you page has to sit in the window's focus chain.
//!
//! gpui matches a key binding by walking the focus chain - the focused
//! element and every ancestor above it - and running the first handler it
//! finds. The shell hangs every `on_action` on its root div, so a chord only
//! fires if there is an unbroken path from the focused element up to that
//! div.
//!
//! The page takes the main column and unmounts the composer, which used to be
//! the only thing the Chat route ever focused. Focus stayed on the unmounted
//! composer, the path to the root was gone, and every shortcut died until a
//! click on Home remounted it. Reproduced on 2026-09-07: three chords on the
//! page gave three byte-identical frames, then one mouse click on Home and
//! the next chord worked again.
//!
//! `NeedsYouPane::focus_handle` plus its `track_focus` are what put the page
//! on that path. Take the `track_focus` away and this test fails: the handle
//! still exists, focus still lands on it, and the chord reaches nothing.

use std::cell::Cell;
use std::rc::Rc;

use gpui::{
    AppContext, Context, Entity, InteractiveElement, IntoElement, KeyBinding, ParentElement, Render,
    Styled, TestAppContext, Window, actions, div,
};

use super::NeedsYouPane;

actions!(needs_you_focus_chain, [Chord]);

/// The shell in miniature: a root that owns the shortcut, with the page
/// mounted under it. Nothing else - the point is the path between them.
struct Root {
    page: Entity<NeedsYouPane>,
    fired: Rc<Cell<usize>>,
}

impl Render for Root {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let fired = self.fired.clone();
        div()
            .size_full()
            .on_action(move |_: &Chord, _, _| fired.set(fired.get() + 1))
            .child(self.page.clone())
    }
}

/// Build the window and return the root, the page's focus handle and the
/// counter the action bumps.
fn mount(cx: &mut TestAppContext) -> (Rc<Cell<usize>>, gpui::FocusHandle, &mut gpui::VisualTestContext)
{
    cx.update(|cx| {
        crate::theme::Theme::install(crate::theme::Appearance::Dark, cx);
        cx.bind_keys([KeyBinding::new("ctrl-shift-t", Chord, None)]);
    });
    let fired = Rc::new(Cell::new(0));
    let counter = fired.clone();
    let (root, cx) = cx.add_window_view(|_, cx| Root {
        page: cx.new(|cx| NeedsYouPane::demo(Vec::new(), Vec::new(), cx)),
        fired: counter,
    });
    let handle = root.read_with(cx, |root, cx| root.page.read(cx).focus_handle.clone());
    (fired, handle, cx)
}

#[gpui::test]
async fn a_chord_fires_while_the_needs_you_page_holds_focus(cx: &mut TestAppContext) {
    let (fired, handle, cx) = mount(cx);

    // What the shell does when the page comes up: hand the page the keyboard,
    // because the composer it replaced is no longer mounted.
    cx.update(|window, cx| window.focus(&handle, cx));
    cx.run_until_parked();
    assert_eq!(
        cx.update(|window, cx| window.focused(cx)),
        Some(handle.clone()),
        "the page must be able to hold focus at all"
    );

    cx.simulate_keystrokes("ctrl-shift-t");
    assert_eq!(
        fired.get(),
        1,
        "the chord must reach the root's handler with the page focused - \
         if this is 0 the page is not in the focus chain and every shortcut \
         is dead while it is open (E2E-KEY-01)"
    );
}

#[gpui::test]
async fn the_same_chord_is_dead_with_focus_off_the_tree(cx: &mut TestAppContext) {
    // The other half of the same fact, so the test above cannot pass by
    // accident: a handle that is not tracked by any mounted element is
    // exactly the state the bug left the window in, and the chord goes
    // nowhere from there.
    let (fired, _handle, cx) = mount(cx);
    let orphan = cx.update(|_, cx| cx.focus_handle());
    cx.update(|window, cx| window.focus(&orphan, cx));
    cx.run_until_parked();

    cx.simulate_keystrokes("ctrl-shift-t");
    assert_eq!(
        fired.get(),
        0,
        "focus on an unmounted handle has no path to the root, so nothing fires"
    );
}
