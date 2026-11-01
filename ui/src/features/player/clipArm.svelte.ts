// V108-CC The player's Clip button asks twice. V108-C21 The first click opens the confirm
// bubble (ConfirmCallout) clear of the button; moving onto it and clicking makes the clip.
// The bubble keeps the 3 s, Escape and clicks elsewhere.

export function useClipArm(run: () => void) {
  let armed = $state(false);
  let anchor = $state<HTMLElement>();
  let pressX = $state<number>();

  return {
    get armed() {
      return armed;
    },
    get anchor() {
      return anchor;
    },
    get pressX() {
      return pressX;
    },
    click(e: MouseEvent) {
      if (armed) return;
      anchor = e.currentTarget as HTMLElement;
      // Keyboard clicks have no pointer position.
      pressX = e.detail === 0 ? undefined : e.clientX;
      armed = true;
    },
    disarm() {
      armed = false;
    },
    confirm() {
      armed = false;
      run();
    },
  };
}
