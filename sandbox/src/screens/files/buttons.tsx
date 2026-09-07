// Rust: app/crates/ui/src/popover.rs (btn_ghost, btn_primary, btn_danger),
// the three buttons files/editor.rs hands to notice().

// Ghost button: quiet text, hover wash. The native hover is a blended fade
// (motion::hover_blend over HOVER_FADE); the class pair is its rest and hover
// ends, with the fade left for the motion seat.
export function btnGhost(label: string, id: string, onPress: () => void) {
  return (
    <button key={id} type="button" data-action={id} onClick={onPress}
      className="px-3 py-1.5 rounded-lg text-ui-13 text-text-muted cursor-pointer hover:text-text hover:bg-wash/5">
      {label}
    </button>
  );
}

// Primary button: the text colour as a fill, the surface colour as the label.
export function btnPrimary(label: string, id: string, onPress: () => void) {
  return (
    <button key={id} type="button" data-action={id} onClick={onPress}
      className="px-3 py-1.5 rounded-lg bg-text text-ui-13 font-medium text-on-solid cursor-pointer hover:bg-text/88">
      {label}
    </button>
  );
}

// Destructive button: the muted red fill. The native label is gpui::white(),
// which reads at 2.8:1 on the dark theme's light red; on-accent flips with the
// appearance and clears AA in both.
export function btnDanger(label: string, id: string, onPress: () => void) {
  return (
    <button key={id} type="button" data-action={id} onClick={onPress}
      className="px-3 py-1.5 rounded-lg bg-danger-strong text-ui-13 font-medium text-on-accent cursor-pointer hover:bg-danger-strong/88">
      {label}
    </button>
  );
}
