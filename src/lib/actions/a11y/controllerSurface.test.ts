import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  controllerSurfaceFor,
  dispatchSurfaceDirection,
  dispatchSurfaceKey,
  findControllerSurface,
} from "./controllerSurface";

function place(element: HTMLElement, left = 0, top = 0, width = 100, height = 40) {
  element.getBoundingClientRect = () => ({
    x: left, y: top, left, top, width, height,
    right: left + width, bottom: top + height,
    toJSON: () => ({}),
  } as DOMRect);
}

function createSurface(): HTMLElement {
  const surface = document.createElement("section");
  surface.setAttribute("data-controller-surface", "");
  surface.tabIndex = 0;
  place(surface);
  return surface;
}

beforeEach(() => {
  document.body.innerHTML = "";
});

describe("controller surface bridge", () => {
  it("finds the surface containing an element and rejects outsiders", () => {
    const surface = createSurface();
    const child = document.createElement("button");
    surface.append(child);
    const outside = document.createElement("button");
    document.body.append(surface, outside);

    expect(controllerSurfaceFor(child)).toBe(surface);
    expect(controllerSurfaceFor(surface)).toBe(surface);
    expect(controllerSurfaceFor(outside)).toBeNull();
    expect(controllerSurfaceFor(null)).toBeNull();
  });

  it("ignores typing targets so text inputs keep their keys", () => {
    const surface = createSurface();
    const input = document.createElement("input");
    surface.append(input);
    document.body.append(surface);
    place(input);

    expect(controllerSurfaceFor(input)).toBeNull();
  });

  it("rejects surfaces that are not visible", () => {
    const surface = createSurface();
    const child = document.createElement("button");
    surface.append(child);
    document.body.append(surface);
    place(child, 0, 0, 0, 0);
    surface.getBoundingClientRect = () => ({
      x: 0, y: 0, left: 0, top: 0, width: 0, height: 0,
      right: 0, bottom: 0, toJSON: () => ({}),
    }) as DOMRect;

    expect(controllerSurfaceFor(child)).toBeNull();
    expect(findControllerSurface(document)).toBeNull();
  });

  it("finds the first visible surface under a root", () => {
    const hidden = createSurface();
    hidden.style.display = "none";
    const visible = createSurface();
    document.body.append(hidden, visible);

    expect(findControllerSurface(document)).toBe(visible);
  });

  it("focuses the surface and dispatches a bubbling keydown", () => {
    const surface = createSurface();
    document.body.append(surface);
    const keys: string[] = [];
    surface.addEventListener("keydown", (event) => keys.push(event.key));

    const result = dispatchSurfaceKey(surface, "Enter");

    expect(result).toBe(true);
    expect(document.activeElement).toBe(surface);
    expect(keys).toEqual(["Enter"]);
  });

  it("maps stick directions to arrow keys", () => {
    const surface = createSurface();
    document.body.append(surface);
    const keys: string[] = [];
    surface.addEventListener("keydown", (event) => keys.push(event.key));

    dispatchSurfaceDirection(surface, "up");
    dispatchSurfaceDirection(surface, "down");
    dispatchSurfaceDirection(surface, "left");
    dispatchSurfaceDirection(surface, "right");

    expect(keys).toEqual(["ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight"]);
  });

  it("lets a handler cancel the synthetic keydown", () => {
    const surface = createSurface();
    document.body.append(surface);
    surface.addEventListener("keydown", (event) => event.preventDefault());
    const spy = vi.fn();
    document.body.addEventListener("keydown", spy);

    const result = dispatchSurfaceKey(surface, "ArrowDown");

    expect(result).toBe(false);
    expect(spy).toHaveBeenCalledOnce();
  });
});
