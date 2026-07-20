export type PortalTarget = HTMLElement | string | null | undefined;

function resolvePortalTarget(target: PortalTarget): HTMLElement | null {
  if (typeof document === "undefined") return null;
  if (target instanceof HTMLElement) return target;
  if (typeof target === "string") return document.querySelector<HTMLElement>(target);
  return document.body;
}

/** Move an overlay outside transformed/virtualized layout ancestors. */
export function portal(node: HTMLElement, target: PortalTarget = undefined) {
  let currentTarget = resolvePortalTarget(target);
  currentTarget?.appendChild(node);

  return {
    update(nextTarget: PortalTarget) {
      const next = resolvePortalTarget(nextTarget);
      if (next && next !== currentTarget) {
        currentTarget = next;
        next.appendChild(node);
      }
    },
    destroy() {
      node.remove();
    },
  };
}
