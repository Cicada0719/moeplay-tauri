import { currentMonitor } from "@tauri-apps/api/window";

export interface FullscreenWindow {
  isFullscreen(): Promise<boolean>;
  outerPosition(): Promise<{ x: number; y: number }>;
  outerSize(): Promise<{ width: number; height: number }>;
  setFullscreen(value: boolean): Promise<void>;
}

let repairInFlight: Promise<void> | null = null;
const wait = (ms: number) => new Promise<void>((resolve) => setTimeout(resolve, ms));

export function boundsMatchMonitor(
  position: { x: number; y: number },
  size: { width: number; height: number },
  monitor: { position: { x: number; y: number }; size: { width: number; height: number } },
  tolerance = 12,
) {
  return Math.abs(position.x - monitor.position.x) <= tolerance
    && Math.abs(position.y - monitor.position.y) <= tolerance
    && Math.abs(size.width - monitor.size.width) <= tolerance
    && Math.abs(size.height - monitor.size.height) <= tolerance;
}

export async function nativeFullscreenHealthy(win: FullscreenWindow) {
  if (!(await win.isFullscreen())) return false;
  const monitor = await currentMonitor();
  if (!monitor) return true;
  const [position, size] = await Promise.all([win.outerPosition(), win.outerSize()]);
  return boundsMatchMonitor(position, size, monitor);
}

export async function reassertNativeFullscreen(win: FullscreenWindow, force = false) {
  if (repairInFlight) return repairInFlight;
  repairInFlight = (async () => {
    const healthy = await nativeFullscreenHealthy(win).catch(() => false);
    if (!force && healthy) return;
    // WebView2 can restore Win32 decorations while winit still reports fullscreen=true.
    // Cycling the native state rebuilds the window style instead of trusting the stale flag.
    await win.setFullscreen(false);
    await wait(40);
    await win.setFullscreen(true);
    await wait(80);
  })().finally(() => { repairInFlight = null; });
  return repairInFlight;
}
