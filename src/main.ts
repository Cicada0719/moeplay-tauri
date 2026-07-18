import "@fontsource/outfit/400.css";
import "@fontsource/outfit/500.css";
import "@fontsource/outfit/600.css";
import "@fontsource/outfit/700.css";
import "@fontsource/jetbrains-mono/400.css";
import "@fontsource/jetbrains-mono/500.css";
import "@fontsource/jetbrains-mono/600.css";
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
