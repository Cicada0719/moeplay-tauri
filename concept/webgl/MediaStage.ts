import * as THREE from "three";
import type { ConceptMediaAsset, MediaStageContract, MotionQuality } from "../contracts";
import { recommendedRenderDpr } from "../rendering/quality";

const VERTEX_SHADER = `
  varying vec2 vUv;
  uniform float uTime;
  uniform float uVelocity;

  void main() {
    vUv = uv;
    vec3 position = position;
    float edge = sin(uv.y * 3.14159265);
    position.z += sin((uv.y * 9.0) + (uTime * 4.0)) * uVelocity * 0.055 * edge;
    position.x += sin((uv.y * 5.0) + (uTime * 2.5)) * uVelocity * 0.025 * edge;
    gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
  }
`;

const FRAGMENT_SHADER = `
  precision highp float;
  varying vec2 vUv;
  uniform sampler2D uFrom;
  uniform sampler2D uTo;
  uniform vec2 uFromScale;
  uniform vec2 uToScale;
  uniform vec2 uFocalPoint;
  uniform float uMix;
  uniform float uVelocity;

  vec2 coverUv(vec2 uv, vec2 scale, vec2 focalPoint) {
    vec2 covered = (uv - 0.5) * scale + focalPoint;
    float bend = (uv.y - 0.5) * uVelocity * 0.018;
    covered.x += bend;
    return clamp(covered, 0.001, 0.999);
  }

  void main() {
    vec2 fromUv = coverUv(vUv, uFromScale, uFocalPoint);
    vec2 toUv = coverUv(vUv, uToScale, uFocalPoint);
    vec4 fromColor = texture2D(uFrom, fromUv);
    vec4 toColor = texture2D(uTo, toUv);
    gl_FragColor = mix(fromColor, toColor, smoothstep(0.0, 1.0, uMix));
  }
`;

export interface MediaStageOptions {
  quality?: MotionQuality;
  reducedMotion?: boolean;
  transitionDuration?: number;
  onContextLost?: () => void;
  onContextRestored?: () => void;
}

interface LoadedMedia {
  asset: ConceptMediaAsset;
  texture: THREE.Texture;
  width: number;
  height: number;
}

function createFallbackTexture(color: string): THREE.DataTexture {
  const parsed = new THREE.Color(color || "#111111");
  const data = new Uint8Array([
    Math.round(parsed.r * 255),
    Math.round(parsed.g * 255),
    Math.round(parsed.b * 255),
    255,
  ]);
  const texture = new THREE.DataTexture(data, 1, 1, THREE.RGBAFormat);
  texture.colorSpace = THREE.SRGBColorSpace;
  texture.needsUpdate = true;
  return texture;
}

export class MediaStage implements MediaStageContract {
  private readonly quality: MotionQuality;
  private readonly reducedMotion: boolean;
  private readonly transitionDuration: number;
  private readonly onContextLost?: () => void;
  private readonly onContextRestored?: () => void;

  private canvas: HTMLCanvasElement | null = null;
  private renderer: THREE.WebGLRenderer | null = null;
  private scene: THREE.Scene | null = null;
  private camera: THREE.OrthographicCamera | null = null;
  private geometry: THREE.PlaneGeometry | null = null;
  private material: THREE.ShaderMaterial | null = null;
  private mesh: THREE.Mesh | null = null;
  private media: LoadedMedia[] = [];
  private activeIndex = 0;
  private fromIndex = 0;
  private velocity = 0;
  private targetVelocity = 0;
  private transitionStart = 0;
  private transitionActive = false;
  private frame = 0;
  private lastTime = 0;
  private width = 1;
  private height = 1;
  private dpr = 1;
  private generation = 0;
  private resizeObserver: ResizeObserver | null = null;
  private contextLost = false;

  constructor(options: MediaStageOptions = {}) {
    this.quality = options.quality ?? "full";
    this.reducedMotion = options.reducedMotion ?? false;
    this.transitionDuration = Math.max(0, options.transitionDuration ?? 520);
    this.onContextLost = options.onContextLost;
    this.onContextRestored = options.onContextRestored;
  }

  async mount(canvas: HTMLCanvasElement, assets: ConceptMediaAsset[]): Promise<void> {
    this.dispose();
    const generation = ++this.generation;
    this.canvas = canvas;
    canvas.addEventListener("webglcontextlost", this.handleContextLost, false);
    canvas.addEventListener("webglcontextrestored", this.handleContextRestored, false);

    const context = canvas.getContext("webgl2", {
      alpha: true,
      antialias: this.quality === "full",
      powerPreference: this.quality === "reduced" ? "low-power" : "high-performance",
      premultipliedAlpha: false,
      failIfMajorPerformanceCaveat: true,
    });
    if (!context) throw new Error("WebGL2 is unavailable");

    const renderer = new THREE.WebGLRenderer({ canvas, context, alpha: true, antialias: this.quality === "full" });
    renderer.outputColorSpace = THREE.SRGBColorSpace;
    renderer.setClearColor(0x000000, 0);
    this.renderer = renderer;
    this.scene = new THREE.Scene();
    this.camera = new THREE.OrthographicCamera(-1, 1, 1, -1, 0.01, 10);
    this.camera.position.z = 1;
    this.geometry = new THREE.PlaneGeometry(2, 2, this.quality === "full" ? 32 : 12, this.quality === "full" ? 20 : 8);

    const fallback = createFallbackTexture(assets[0]?.dominantColor ?? "#111111");
    this.material = new THREE.ShaderMaterial({
      vertexShader: VERTEX_SHADER,
      fragmentShader: FRAGMENT_SHADER,
      transparent: true,
      depthTest: false,
      depthWrite: false,
      uniforms: {
        uTime: { value: 0 },
        uVelocity: { value: 0 },
        uMix: { value: 1 },
        uFrom: { value: fallback },
        uTo: { value: fallback },
        uFromScale: { value: new THREE.Vector2(1, 1) },
        uToScale: { value: new THREE.Vector2(1, 1) },
        uFocalPoint: { value: new THREE.Vector2(0.5, 0.5) },
      },
    });
    this.mesh = new THREE.Mesh(this.geometry, this.material);
    this.scene.add(this.mesh);

    const rect = canvas.getBoundingClientRect();
    this.resize(rect.width || canvas.clientWidth || 1, rect.height || canvas.clientHeight || 1, window.devicePixelRatio || 1);
    this.resizeObserver = typeof ResizeObserver === "undefined" ? null : new ResizeObserver((entries) => {
      const entry = entries[0];
      if (!entry) return;
      this.resize(entry.contentRect.width, entry.contentRect.height, window.devicePixelRatio || 1);
    });
    this.resizeObserver?.observe(canvas);

    const loaded = await Promise.all(assets.map((asset) => this.loadAsset(asset)));
    if (generation !== this.generation || this.canvas !== canvas) {
      for (const item of loaded) item.texture.dispose();
      return;
    }
    if (loaded.length > 0) fallback.dispose();
    this.media = loaded;
    this.activeIndex = 0;
    this.fromIndex = 0;
    this.applyTextures(0, 0);
    this.startLoop();
  }

  setActive(index: number, velocity = 0): void {
    if (!this.material || this.media.length === 0) return;
    const nextIndex = Math.max(0, Math.min(this.media.length - 1, Math.trunc(index)));
    const previousIndex = this.activeIndex;
    this.fromIndex = previousIndex;
    this.activeIndex = nextIndex;
    this.targetVelocity = this.reducedMotion ? 0 : THREE.MathUtils.clamp(Math.abs(velocity), 0, 4);

    this.applyTextures(previousIndex, nextIndex);
    if (this.reducedMotion || this.quality === "reduced" || previousIndex === nextIndex) {
      this.material.uniforms.uMix.value = 1;
      this.transitionActive = false;
      this.render(performance.now());
      return;
    }

    this.material.uniforms.uMix.value = 0;
    this.transitionStart = performance.now();
    this.transitionActive = true;
    this.startLoop();
  }

  resize(width: number, height: number, dpr: number): void {
    if (!this.renderer || !this.camera) return;
    this.width = Math.max(1, width);
    this.height = Math.max(1, height);
    this.dpr = recommendedRenderDpr(this.quality, dpr);
    this.renderer.setPixelRatio(this.dpr);
    this.renderer.setSize(this.width, this.height, false);
    this.updateUvScales();
    this.render(performance.now());
  }

  dispose(): void {
    this.generation += 1;
    if (this.frame) cancelAnimationFrame(this.frame);
    this.frame = 0;
    this.resizeObserver?.disconnect();
    this.resizeObserver = null;
    this.canvas?.removeEventListener("webglcontextlost", this.handleContextLost, false);
    this.canvas?.removeEventListener("webglcontextrestored", this.handleContextRestored, false);

    const disposedTextures = new Set<unknown>();
    for (const item of this.media) {
      item.texture.dispose();
      disposedTextures.add(item.texture);
    }
    this.media = [];
    const uniforms = this.material?.uniforms;
    const uniformTextures = [uniforms?.uFrom?.value, uniforms?.uTo?.value];
    for (const texture of new Set(uniformTextures)) {
      if (texture instanceof THREE.Texture && !disposedTextures.has(texture)) texture.dispose();
    }
    this.geometry?.dispose();
    this.material?.dispose();
    this.renderer?.renderLists.dispose();
    this.renderer?.dispose();
    this.renderer?.forceContextLoss();

    this.canvas = null;
    this.renderer = null;
    this.scene = null;
    this.camera = null;
    this.geometry = null;
    this.material = null;
    this.mesh = null;
    this.contextLost = false;
    this.transitionActive = false;
    this.lastTime = 0;
  }

  private readonly handleContextLost = (event: Event): void => {
    event.preventDefault();
    this.contextLost = true;
    if (this.frame) cancelAnimationFrame(this.frame);
    this.frame = 0;
    this.onContextLost?.();
  };

  private readonly handleContextRestored = (): void => {
    this.contextLost = false;
    if (this.material) this.material.uniformsNeedUpdate = true;
    this.updateUvScales();
    this.onContextRestored?.();
    this.startLoop();
  };

  private async loadAsset(asset: ConceptMediaAsset): Promise<LoadedMedia> {
    if (asset.mediaType !== "image") {
      const texture = createFallbackTexture(asset.dominantColor);
      return { asset, texture, width: 1, height: 1 };
    }

    const loader = new THREE.TextureLoader();
    loader.setCrossOrigin("anonymous");
    try {
      const texture = await loader.loadAsync(asset.src);
      texture.colorSpace = THREE.SRGBColorSpace;
      texture.minFilter = THREE.LinearFilter;
      texture.magFilter = THREE.LinearFilter;
      texture.generateMipmaps = this.quality === "full";
      const image = texture.image as { naturalWidth?: number; naturalHeight?: number; width?: number; height?: number };
      return {
        asset,
        texture,
        width: image.naturalWidth ?? image.width ?? 1,
        height: image.naturalHeight ?? image.height ?? 1,
      };
    } catch {
      const texture = createFallbackTexture(asset.dominantColor);
      return { asset, texture, width: 1, height: 1 };
    }
  }

  private applyTextures(from: number, to: number): void {
    if (!this.material || this.media.length === 0) return;
    const fromMedia = this.media[from] ?? this.media[0];
    const toMedia = this.media[to] ?? fromMedia;
    this.material.uniforms.uFrom.value = fromMedia.texture;
    this.material.uniforms.uTo.value = toMedia.texture;
    this.material.uniforms.uFocalPoint.value.set(toMedia.asset.focalPoint.x, 1 - toMedia.asset.focalPoint.y);
    this.updateUvScales();
  }

  private updateUvScales(): void {
    if (!this.material || this.media.length === 0) return;
    const stageAspect = this.width / this.height;
    const fromMedia = this.media[this.fromIndex] ?? this.media[0];
    const toMedia = this.media[this.activeIndex] ?? fromMedia;
    this.material.uniforms.uFromScale.value.copy(this.coverScale(fromMedia.width / fromMedia.height, stageAspect));
    this.material.uniforms.uToScale.value.copy(this.coverScale(toMedia.width / toMedia.height, stageAspect));
  }

  private coverScale(imageAspect: number, stageAspect: number): THREE.Vector2 {
    if (imageAspect > stageAspect) return new THREE.Vector2(stageAspect / imageAspect, 1);
    return new THREE.Vector2(1, imageAspect / stageAspect);
  }

  private startLoop(): void {
    if (this.frame || this.contextLost || !this.renderer) return;
    this.lastTime = performance.now();
    this.frame = requestAnimationFrame(this.tick);
  }

  private readonly tick = (time: number): void => {
    this.frame = 0;
    if (!this.renderer || this.contextLost) return;
    const delta = Math.min(64, time - this.lastTime);
    this.lastTime = time;
    this.velocity = THREE.MathUtils.damp(this.velocity, this.targetVelocity, 8, delta / 1000);
    this.targetVelocity = THREE.MathUtils.damp(this.targetVelocity, 0, 5, delta / 1000);

    if (this.transitionActive && this.material) {
      const progress = this.transitionDuration === 0 ? 1 : (time - this.transitionStart) / this.transitionDuration;
      this.material.uniforms.uMix.value = THREE.MathUtils.smoothstep(progress, 0, 1);
      if (progress >= 1) {
        this.transitionActive = false;
        this.fromIndex = this.activeIndex;
        this.material.uniforms.uFrom.value = this.material.uniforms.uTo.value;
        this.updateUvScales();
      }
    }

    this.render(time);
    if (this.transitionActive || this.velocity > 0.002 || this.targetVelocity > 0.002) {
      this.frame = requestAnimationFrame(this.tick);
    }
  };

  private render(time: number): void {
    if (!this.renderer || !this.scene || !this.camera || !this.material || this.contextLost) return;
    this.material.uniforms.uTime.value = time / 1000;
    this.material.uniforms.uVelocity.value = this.reducedMotion ? 0 : this.velocity;
    this.renderer.render(this.scene, this.camera);
  }
}

export function createMediaStage(options: MediaStageOptions = {}): MediaStage {
  return new MediaStage(options);
}






