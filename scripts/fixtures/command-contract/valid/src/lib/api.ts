import { invokeCmd } from "./core";

export const load = () => invokeCmd<{ ok: boolean }>("foo_bar");
