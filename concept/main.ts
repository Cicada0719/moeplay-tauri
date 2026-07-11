import "@fontsource/outfit/400.css";
import "@fontsource/outfit/500.css";
import "@fontsource/outfit/600.css";
import "@fontsource/outfit/700.css";
import "@fontsource/jetbrains-mono/400.css";
import "@fontsource/jetbrains-mono/500.css";
import "@fontsource/jetbrains-mono/600.css";
import { mount } from "svelte";
import App from "./App.svelte";
import "./concept.css";

const app = mount(App, {
  target: document.getElementById("concept-app")!,
});

export default app;
