type SpotlightOptions = {
  disabled?: boolean;
  radius?: number;
};

function setIdleSpotlight(node: HTMLElement) {
  node.style.setProperty("--spotlight-x", "50%");
  node.style.setProperty("--spotlight-y", "0%");
  node.style.setProperty("--mx", "50%");
  node.style.setProperty("--my", "0%");
}

export function spotlight(node: HTMLElement, options: SpotlightOptions = {}) {
  let disabled = Boolean(options.disabled);
  const reduceMotion = window.matchMedia?.("(prefers-reduced-motion: reduce)");

  function isDisabled() {
    return disabled || Boolean(reduceMotion?.matches);
  }

  function move(event: PointerEvent) {
    if (isDisabled()) return;
    const rect = node.getBoundingClientRect();
    const x = ((event.clientX - rect.left) / Math.max(rect.width, 1)) * 100;
    const y = ((event.clientY - rect.top) / Math.max(rect.height, 1)) * 100;
    const mx = `${Math.min(100, Math.max(0, x)).toFixed(2)}%`;
    const my = `${Math.min(100, Math.max(0, y)).toFixed(2)}%`;
    node.style.setProperty("--spotlight-x", mx);
    node.style.setProperty("--spotlight-y", my);
    node.style.setProperty("--mx", mx);
    node.style.setProperty("--my", my);
  }

  function leave() {
    setIdleSpotlight(node);
  }

  if (typeof options.radius === "number") {
    node.style.setProperty("--spotlight-radius", `${options.radius}px`);
  }
  setIdleSpotlight(node);
  node.addEventListener("pointermove", move);
  node.addEventListener("pointerleave", leave);

  return {
    update(next: SpotlightOptions = {}) {
      disabled = Boolean(next.disabled);
      if (typeof next.radius === "number") {
        node.style.setProperty("--spotlight-radius", `${next.radius}px`);
      }
      if (isDisabled()) setIdleSpotlight(node);
    },
    destroy() {
      node.removeEventListener("pointermove", move);
      node.removeEventListener("pointerleave", leave);
    },
  };
}
