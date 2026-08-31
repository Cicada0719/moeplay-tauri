// 迷你置顶播放器的主/小窗握手契约（localStorage 同源共享）
export const MINI_SESSION_KEY = "moeplay-mini-session-v1";
export const MINI_CLOSED_EVENT_KEY = "moeplay-mini-closed-v1";

export interface MiniSession {
  url: string;
  title: string;
  isM3u8: boolean;
  time: number;
  updatedAt: number;
}

export function readMiniSession(): MiniSession | null {
  try {
    const raw = localStorage.getItem(MINI_SESSION_KEY);
    if (!raw) return null;
    const parsed = JSON.parse(raw) as MiniSession;
    return parsed && typeof parsed.url === "string" && parsed.url ? parsed : null;
  } catch {
    return null;
  }
}

export function writeMiniSession(session: MiniSession): void {
  try {
    localStorage.setItem(MINI_SESSION_KEY, JSON.stringify(session));
  } catch {
    // 隐私模式等场景忽略持久化失败
  }
}
