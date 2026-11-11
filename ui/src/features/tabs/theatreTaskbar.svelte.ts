// V110-B10 Theatre's taskbar room: at the window's bottom edge the window shrinks to the work
// area so the taskbar shows (`window_taskbar_room`), and covers the screen again after the
// pointer stays clear of the bottom. One instance per window.

import { windowsIpc } from '../../ipc/windows';
import { backIn, OFF, type Room, roomStep } from './theatreTaskbar';

export class TheatreTaskbar {
  private room: Room = OFF;
  private fromBottom: number | null = null;
  private timer = 0;

  private step(): void {
    clearTimeout(this.timer);
    const now = performance.now();
    const next = roomStep(this.room, this.fromBottom, now);
    if (next.on !== this.room.on) void windowsIpc.taskbarRoom(next.on).catch(() => {});
    this.room = next;
    const wait = backIn(next, now);
    if (wait !== null) this.timer = window.setTimeout(() => this.step(), wait);
  }

  /** Starts listening; the returned function stops and gives the screen back. */
  start(): () => void {
    const move = (e: PointerEvent) => {
      this.fromBottom = window.innerHeight - e.clientY;
      this.step();
    };
    const leave = () => {
      this.fromBottom = null;
      this.step();
    };
    window.addEventListener('pointermove', move, true);
    document.documentElement.addEventListener('pointerleave', leave);
    return () => {
      window.removeEventListener('pointermove', move, true);
      document.documentElement.removeEventListener('pointerleave', leave);
      clearTimeout(this.timer);
      if (this.room.on) void windowsIpc.taskbarRoom(false).catch(() => {});
      this.room = OFF;
      this.fromBottom = null;
    };
  }
}
