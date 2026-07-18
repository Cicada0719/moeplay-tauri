/**
 * Kinetic WebGL 舞台场景（概念站 webgl/MediaStage.ts 的生产硬化移植）。
 *
 * 与概念版的关键差异：
 *   - 零外部资源：不加载任何纹理/模型，画面全部由着色器程序化生成
 *     （连续媒体流用三层视差光带表达，配色来自主题 token palette）。
 *   - high 档追加程序化粒子层（gl_PointCoord 径向衰减，无纹理）。
 *   - 渲染循环交给 KineticMotionDriver：页面隐藏/窗口失焦自动暂停。
 *   - dispose 完整释放 geometry/material/renderer/监听器/粒子资源。
 *
 * 本文件静态 import three；它只被 KineticStage 通过动态 import 触达，
 * 因此 three 落在独立的懒加载 chunk 中。
 */

import * as THREE from "three";
import { createKineticMotionDriver, KineticMotionDriver } from "../motionDriver";
import { recommendedRenderDpr } from "../quality";
import type {
  KineticPalette,
  KineticQuality,
  KineticSceneContract,
  KineticSceneOptions,
  KineticRgb,
} from "../types";

const VERTEX_SHADER = /* glsl */ `
  varying vec2 vUv;
  uniform float uTime;
  uniform float uDrift;

  void main() {
    vUv = uv;
    vec3 position = position;
    float edge = sin(uv.y * 3.14159265);
    position.z += sin((uv.y * 7.0) + (uTime * 1.6)) * uDrift * 0.05 * edge;
    position.x += sin((uv.y * 4.0) + (uTime * 1.1)) * uDrift * 0.02 * edge;
    gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
  }
`;

const FRAGMENT_SHADER = /* glsl */ `
  precision highp float;
  varying vec2 vUv;
  uniform float uTime;
  uniform float uDrift;
  uniform float uIntensity;
  uniform vec3 uColorBg;
  uniform vec3 uColorSurface;
  uniform vec3 uColorAccent;
  uniform vec3 uColorGlow;

  float band(vec2 uv, float freq, float speed, float offset, float width) {
    float wave = sin(uv.x * freq + uTime * speed + offset);
    float center = 0.5 + wave * 0.18;
    return smoothstep(width, 0.0, abs(uv.y - center));
  }

  void main() {
    // 连续媒体流：uv 随时间缓慢横移，uDrift 呼吸式加减速。
    vec2 uv = vUv;
    uv.x += uTime * 0.008 + uDrift * 0.03;

    vec3 color = uColorBg;
    // 远景层：surface 大幕，慢速深景深。
    float far = band(uv + vec2(0.0, 0.10), 3.1, 0.21, 0.0, 0.42);
    color = mix(color, uColorSurface, far * 0.55 * uIntensity);
    // 中景层：accent 主光带。
    float mid = band(uv + vec2(0.13, -0.06), 4.7, -0.16, 1.7, 0.30);
    color = mix(color, uColorAccent, mid * 0.34 * uIntensity);
    // 近景层：高光丝带，最快，营造视差深度。
    float near = band(uv + vec2(-0.21, 0.18), 6.3, 0.11, 3.9, 0.18);
    color = mix(color, uColorGlow, near * 0.22 * uIntensity);

    // 暗角收拢视线。
    float d = distance(vUv, vec2(0.5));
    color *= 1.0 - smoothstep(0.42, 0.86, d) * 0.55;
    gl_FragColor = vec4(color, 1.0);
  }
`;

const PARTICLE_VERTEX_SHADER = /* glsl */ `
  attribute float aSeed;
  uniform float uTime;
  uniform float uPixelRatio;
  varying float vSeed;

  void main() {
    vSeed = aSeed;
    vec3 p = position;
    p.y = mod(p.y + uTime * (0.02 + aSeed * 0.05) + aSeed * 2.4, 2.4) - 1.2;
    p.x += sin(uTime * 0.3 + aSeed * 17.0) * 0.05;
    gl_Position = projectionMatrix * modelViewMatrix * vec4(p, 1.0);
    gl_PointSize = (1.5 + aSeed * 3.0) * uPixelRatio;
  }
`;

const PARTICLE_FRAGMENT_SHADER = /* glsl */ `
  precision highp float;
  uniform vec3 uColor;
  varying float vSeed;

  void main() {
    float d = distance(gl_PointCoord, vec2(0.5));
    float alpha = smoothstep(0.5, 0.05, d) * (0.25 + vSeed * 0.45);
    if (alpha < 0.01) discard;
    gl_FragColor = vec4(uColor, alpha);
  }
`;

const PLANE_SEGMENTS: Record<KineticQuality, { width: number; height: number }> = {
  high: { width: 32, height: 20 },
  medium: { width: 16, height: 10 },
  low: { width: 8, height: 6 },
};

const QUALITY_INTENSITY: Record<KineticQuality, number> = {
  high: 1,
  medium: 0.85,
  low: 0.7,
};

const PARTICLE_COUNT = 90;

function toThreeColor(rgb: KineticRgb): THREE.Color {
  return new THREE.Color(rgb.r, rgb.g, rgb.b);
}

export class KineticScene implements KineticSceneContract {
  private quality: KineticQuality;
  private palette: KineticPalette;
  private readonly onContextLost?: () => void;
  private readonly onContextRestored?: () => void;
  private readonly onFrame?: (deltaMs: number) => void;

  private canvas: HTMLCanvasElement | null = null;
  private renderer: THREE.WebGLRenderer | null = null;
  private scene: THREE.Scene | null = null;
  private camera: THREE.OrthographicCamera | null = null;
  private geometry: THREE.PlaneGeometry | null = null;
  private material: THREE.ShaderMaterial | null = null;
  private mesh: THREE.Mesh | null = null;
  private particleGeometry: THREE.BufferGeometry | null = null;
  private particleMaterial: THREE.ShaderMaterial | null = null;
  private particles: THREE.Points | null = null;
  private driver: KineticMotionDriver | null = null;
  private resizeObserver: ResizeObserver | null = null;
  private contextLost = false;
  private width = 1;
  private height = 1;
  private drift = 0;
  private mounted = false;
  private generation = 0;

  constructor(options: KineticSceneOptions) {
    this.quality = options.quality;
    this.palette = options.palette;
    this.onContextLost = options.onContextLost;
    this.onContextRestored = options.onContextRestored;
    this.onFrame = options.onFrame;
  }

  async mount(canvas: HTMLCanvasElement): Promise<void> {
    this.dispose();
    const generation = ++this.generation;
    try {
      this.mountInternal(canvas, generation);
    } catch (error) {
      // 初始化中途失败（如上下文创建失败）必须自清，不得残留监听器/半挂载状态。
      this.dispose();
      throw error;
    }
  }

  private mountInternal(canvas: HTMLCanvasElement, generation: number): void {
    this.canvas = canvas;
    canvas.addEventListener("webglcontextlost", this.handleContextLost, false);
    canvas.addEventListener("webglcontextrestored", this.handleContextRestored, false);

    const context = canvas.getContext("webgl2", {
      alpha: false,
      antialias: this.quality === "high",
      powerPreference: this.quality === "low" ? "low-power" : "high-performance",
      failIfMajorPerformanceCaveat: true,
    });
    if (!context) throw new Error("WebGL2 is unavailable");
    if (generation !== this.generation) return;

    const renderer = new THREE.WebGLRenderer({
      canvas,
      context,
      alpha: false,
      antialias: this.quality === "high",
      powerPreference: this.quality === "low" ? "low-power" : "high-performance",
    });
    renderer.outputColorSpace = THREE.SRGBColorSpace;
    renderer.setClearColor(toThreeColor(this.palette.bg), 1);
    this.renderer = renderer;

    this.scene = new THREE.Scene();
    this.camera = new THREE.OrthographicCamera(-1, 1, 1, -1, 0.01, 10);
    this.camera.position.z = 1;

    this.geometry = this.createPlaneGeometry();
    this.material = new THREE.ShaderMaterial({
      vertexShader: VERTEX_SHADER,
      fragmentShader: FRAGMENT_SHADER,
      depthTest: false,
      depthWrite: false,
      uniforms: {
        uTime: { value: 0 },
        uDrift: { value: 0 },
        uIntensity: { value: QUALITY_INTENSITY[this.quality] },
        uColorBg: { value: toThreeColor(this.palette.bg) },
        uColorSurface: { value: toThreeColor(this.palette.surface) },
        uColorAccent: { value: toThreeColor(this.palette.accent) },
        uColorGlow: { value: toThreeColor(this.palette.glow) },
      },
    });
    this.mesh = new THREE.Mesh(this.geometry, this.material);
    this.scene.add(this.mesh);
    this.rebuildParticles();

    const rect = canvas.getBoundingClientRect();
    const dpr = typeof window === "undefined" ? 1 : window.devicePixelRatio || 1;
    this.resize(rect.width || canvas.clientWidth || 1, rect.height || canvas.clientHeight || 1, dpr);
    if (typeof ResizeObserver === "function") {
      this.resizeObserver = new ResizeObserver((entries) => {
        const entry = entries[0];
        if (!entry) return;
        this.resize(
          entry.contentRect.width,
          entry.contentRect.height,
          typeof window === "undefined" ? 1 : window.devicePixelRatio || 1,
        );
      });
      this.resizeObserver.observe(canvas);
    }

    this.driver = createKineticMotionDriver();
    this.mounted = true;
    this.driver.start(this.tick);
    // 首帧立即渲染，避免循环启动前出现空画布。
    this.render(typeof performance === "undefined" ? 0 : performance.now());
  }

  setPalette(palette: KineticPalette): void {
    this.palette = palette;
    this.renderer?.setClearColor(toThreeColor(palette.bg), 1);
    if (this.material) {
      (this.material.uniforms.uColorBg.value as THREE.Color).copy(toThreeColor(palette.bg));
      (this.material.uniforms.uColorSurface.value as THREE.Color).copy(toThreeColor(palette.surface));
      (this.material.uniforms.uColorAccent.value as THREE.Color).copy(toThreeColor(palette.accent));
      (this.material.uniforms.uColorGlow.value as THREE.Color).copy(toThreeColor(palette.glow));
    }
    if (this.particleMaterial) {
      (this.particleMaterial.uniforms.uColor.value as THREE.Color).copy(toThreeColor(palette.glow));
    }
    this.render(typeof performance === "undefined" ? 0 : performance.now());
  }

  setQuality(quality: KineticQuality): void {
    if (quality === this.quality) return;
    this.quality = quality;
    if (this.material) this.material.uniforms.uIntensity.value = QUALITY_INTENSITY[quality];
    // 分段数随档位变化，重建平面几何（粒子仅 high 档）。
    if (this.mesh && this.scene) {
      const previous = this.geometry;
      this.geometry = this.createPlaneGeometry();
      this.mesh.geometry = this.geometry;
      previous?.dispose();
    }
    this.rebuildParticles();
    this.resize(this.width, this.height, typeof window === "undefined" ? 1 : window.devicePixelRatio || 1);
  }

  resize(width: number, height: number, dpr: number): void {
    if (!this.renderer || !this.camera) return;
    this.width = Math.max(1, width);
    this.height = Math.max(1, height);
    const renderDpr = recommendedRenderDpr(this.quality, dpr);
    this.renderer.setPixelRatio(renderDpr);
    this.renderer.setSize(this.width, this.height, false);
    if (this.particleMaterial) this.particleMaterial.uniforms.uPixelRatio.value = renderDpr;
    this.render(typeof performance === "undefined" ? 0 : performance.now());
  }

  pause(): void {
    this.driver?.pause();
  }

  resume(): void {
    if (this.contextLost) return;
    this.driver?.resume();
  }

  dispose(): void {
    this.generation += 1;
    this.mounted = false;
    this.driver?.dispose();
    this.driver = null;
    this.resizeObserver?.disconnect();
    this.resizeObserver = null;
    this.canvas?.removeEventListener("webglcontextlost", this.handleContextLost, false);
    this.canvas?.removeEventListener("webglcontextrestored", this.handleContextRestored, false);

    this.geometry?.dispose();
    this.material?.dispose();
    this.particleGeometry?.dispose();
    this.particleMaterial?.dispose();
    this.renderer?.renderLists.dispose();
    this.renderer?.dispose();
    if (!this.contextLost) this.renderer?.forceContextLoss();

    this.canvas = null;
    this.renderer = null;
    this.scene = null;
    this.camera = null;
    this.geometry = null;
    this.material = null;
    this.mesh = null;
    this.particleGeometry = null;
    this.particleMaterial = null;
    this.particles = null;
    this.contextLost = false;
    this.drift = 0;
  }

  private createPlaneGeometry(): THREE.PlaneGeometry {
    const segments = PLANE_SEGMENTS[this.quality];
    return new THREE.PlaneGeometry(2, 2, segments.width, segments.height);
  }

  private rebuildParticles(): void {
    if (!this.scene) return;
    if (this.particles) {
      this.scene.remove(this.particles);
      this.particles = null;
    }
    this.particleGeometry?.dispose();
    this.particleMaterial?.dispose();
    this.particleGeometry = null;
    this.particleMaterial = null;
    if (this.quality !== "high") return;

    const positions = new Float32Array(PARTICLE_COUNT * 3);
    const seeds = new Float32Array(PARTICLE_COUNT);
    for (let index = 0; index < PARTICLE_COUNT; index += 1) {
      positions[index * 3] = Math.random() * 2.4 - 1.2;
      positions[index * 3 + 1] = Math.random() * 2.4 - 1.2;
      positions[index * 3 + 2] = 0;
      seeds[index] = Math.random();
    }
    this.particleGeometry = new THREE.BufferGeometry();
    this.particleGeometry.setAttribute("position", new THREE.BufferAttribute(positions, 3));
    this.particleGeometry.setAttribute("aSeed", new THREE.BufferAttribute(seeds, 1));
    this.particleMaterial = new THREE.ShaderMaterial({
      vertexShader: PARTICLE_VERTEX_SHADER,
      fragmentShader: PARTICLE_FRAGMENT_SHADER,
      transparent: true,
      depthTest: false,
      depthWrite: false,
      blending: THREE.AdditiveBlending,
      uniforms: {
        uTime: { value: 0 },
        uPixelRatio: { value: recommendedRenderDpr(this.quality, typeof window === "undefined" ? 1 : window.devicePixelRatio || 1) },
        uColor: { value: toThreeColor(this.palette.glow) },
      },
    });
    this.particles = new THREE.Points(this.particleGeometry, this.particleMaterial);
    this.scene.add(this.particles);
  }

  private readonly handleContextLost = (event: Event): void => {
    event.preventDefault();
    this.contextLost = true;
    this.driver?.pause();
    this.onContextLost?.();
  };

  private readonly handleContextRestored = (): void => {
    this.contextLost = false;
    if (this.material) this.material.uniformsNeedUpdate = true;
    this.onContextRestored?.();
    if (this.mounted) this.driver?.resume();
  };

  private readonly tick = (time: number, delta: number): void => {
    if (!this.renderer || this.contextLost) return;
    this.onFrame?.(delta);
    // 呼吸式漂移：连续媒体流的缓动节奏，无输入也保持慢速流动。
    const target = 0.55 + 0.45 * Math.sin(time * 0.00016);
    this.drift += (target - this.drift) * Math.min(1, (delta / 1000) * 2.4);
    this.render(time);
  };

  private render(time: number): void {
    if (!this.renderer || !this.scene || !this.camera || !this.material || this.contextLost) return;
    this.material.uniforms.uTime.value = time / 1000;
    this.material.uniforms.uDrift.value = this.drift;
    if (this.particleMaterial) this.particleMaterial.uniforms.uTime.value = time / 1000;
    this.renderer.render(this.scene, this.camera);
  }
}

export function createKineticScene(options: KineticSceneOptions): KineticScene {
  return new KineticScene(options);
}
