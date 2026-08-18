// 只引入 latin 子集（CJK 由系统字体回退），减少首屏字体体积。
import "@fontsource/outfit/latin-400.css";
import "@fontsource/outfit/latin-500.css";
import "@fontsource/outfit/latin-600.css";
import "@fontsource/outfit/latin-700.css";
import "@fontsource/jetbrains-mono/latin-400.css";
import "@fontsource/jetbrains-mono/latin-500.css";
import "@fontsource/jetbrains-mono/latin-600.css";
import "./app.css";
import "./lib/styles/tokens-v2.css";
import "./lib/styles/anime-themes.css";
import "./lib/styles/scheme-c.css";
import "./lib/styles/visual-system.css";
import "./lib/styles/themes/shift-editorial.css";
import "./lib/styles/themes/phantom-pop.css";
import "./lib/styles/themes/caution-industrial.css";
import "./lib/styles/themes/astral-rail.css";
import "./lib/styles/themes/borderless-lumen.css";
import { mount } from "svelte";
import App from "./App.svelte";

const app = mount(App, {
  target: document.getElementById("app")!,
});

export default app;
