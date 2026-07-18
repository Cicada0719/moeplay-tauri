export type VideoEnhancementMode = "off" | "balanced" | "quality";
export type VideoEnhancementStatus = "off" | "initializing" | "ready" | "error";

export function normalizeVideoEnhancementMode(value: unknown): VideoEnhancementMode {
  return value === "balanced" || value === "quality" ? value : "off";
}

export interface EnhancementProfile {
  scale: number;
  strength: number;
  maxWidth: number;
  maxHeight: number;
}

export const ENHANCEMENT_PROFILES: Record<Exclude<VideoEnhancementMode, "off">, EnhancementProfile> = {
  balanced: { scale: 1.5, strength: 0.42, maxWidth: 1920, maxHeight: 1080 },
  quality: { scale: 2, strength: 0.72, maxWidth: 2560, maxHeight: 1440 },
};

export function resolveEnhancementSize(
  sourceWidth: number,
  sourceHeight: number,
  displayWidth: number,
  displayHeight: number,
  devicePixelRatio: number,
  mode: Exclude<VideoEnhancementMode, "off">,
): { width: number; height: number } {
  const profile = ENHANCEMENT_PROFILES[mode];
  const sourceAspect = sourceWidth > 0 && sourceHeight > 0
    ? sourceWidth / sourceHeight
    : displayWidth > 0 && displayHeight > 0 ? displayWidth / displayHeight : 16 / 9;
  const desiredWidth = Math.max(displayWidth * Math.max(1, devicePixelRatio), sourceWidth * profile.scale, 2);
  const desiredHeight = desiredWidth / sourceAspect;
  const capScale = Math.min(1, profile.maxWidth / desiredWidth, profile.maxHeight / desiredHeight);
  return {
    width: Math.max(2, Math.round(desiredWidth * capScale)),
    height: Math.max(2, Math.round(desiredHeight * capScale)),
  };
}

const VERTEX_SHADER = `#version 300 es
in vec2 aPosition;
out vec2 vUv;
void main() {
  vUv = aPosition * 0.5 + 0.5;
  gl_Position = vec4(aPosition, 0.0, 1.0);
}`;

const FRAGMENT_SHADER = `#version 300 es
precision highp float;
uniform sampler2D uFrame;
uniform vec2 uTexel;
uniform float uStrength;
in vec2 vUv;
out vec4 outColor;

float luma(vec3 value) { return dot(value, vec3(0.299, 0.587, 0.114)); }

void main() {
  vec3 c = texture(uFrame, vUv).rgb;
  vec3 n = texture(uFrame, vUv + vec2(0.0, -uTexel.y)).rgb;
  vec3 s = texture(uFrame, vUv + vec2(0.0,  uTexel.y)).rgb;
  vec3 w = texture(uFrame, vUv + vec2(-uTexel.x, 0.0)).rgb;
  vec3 e = texture(uFrame, vUv + vec2( uTexel.x, 0.0)).rgb;
  vec3 nw = texture(uFrame, vUv + vec2(-uTexel.x, -uTexel.y)).rgb;
  vec3 ne = texture(uFrame, vUv + vec2( uTexel.x, -uTexel.y)).rgb;
  vec3 sw = texture(uFrame, vUv + vec2(-uTexel.x,  uTexel.y)).rgb;
  vec3 se = texture(uFrame, vUv + vec2( uTexel.x,  uTexel.y)).rgb;

  vec3 crossBlur = (n + s + w + e) * 0.25;
  vec3 diagonalBlur = (nw + ne + sw + se) * 0.25;
  float edge = abs(luma(n) - luma(s)) + abs(luma(w) - luma(e));
  float diagonalEdge = abs(luma(nw) - luma(se)) + abs(luma(ne) - luma(sw));
  float adaptive = clamp((edge + diagonalEdge) * 1.8, 0.0, 1.0);
  vec3 localAverage = mix(crossBlur, diagonalBlur, 0.35);
  vec3 sharpened = c + (c - localAverage) * uStrength * (0.55 + adaptive);
  vec3 chromaPreserved = mix(vec3(luma(sharpened)), sharpened, 1.04);
  outColor = vec4(clamp(chromaPreserved, 0.0, 1.0), 1.0);
}`;

function compileShader(gl: WebGL2RenderingContext, type: number, source: string): WebGLShader {
  const shader = gl.createShader(type);
  if (!shader) throw new Error("无法创建画质增强着色器");
  gl.shaderSource(shader, source);
  gl.compileShader(shader);
  if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
    const message = gl.getShaderInfoLog(shader) || "着色器编译失败";
    gl.deleteShader(shader);
    throw new Error(message);
  }
  return shader;
}

export class LocalVideoEnhancer {
  private gl: WebGL2RenderingContext | null = null;
  private program: WebGLProgram | null = null;
  private texture: WebGLTexture | null = null;
  private buffer: WebGLBuffer | null = null;
  private hasRendered = false;
  private frameHandle: number | null = null;
  private videoFrameHandle: number | null = null;
  private destroyed = false;
  private readonly listeners: Array<() => void> = [];

  constructor(
    private readonly canvas: HTMLCanvasElement,
    private readonly video: HTMLVideoElement,
    private readonly mode: Exclude<VideoEnhancementMode, "off">,
    private readonly onStatus: (status: VideoEnhancementStatus, message?: string) => void = () => {},
  ) {}

  start(): void {
    this.onStatus("initializing");
    try {
      const gl = this.canvas.getContext("webgl2", {
        alpha: false,
        antialias: false,
        depth: false,
        premultipliedAlpha: false,
        preserveDrawingBuffer: false,
        powerPreference: "high-performance",
      });
      if (!gl) throw new Error("当前显卡或 WebView 不支持 WebGL2");
      this.gl = gl;
      const program = gl.createProgram();
      if (!program) throw new Error("无法创建画质增强程序");
      const vertex = compileShader(gl, gl.VERTEX_SHADER, VERTEX_SHADER);
      const fragment = compileShader(gl, gl.FRAGMENT_SHADER, FRAGMENT_SHADER);
      gl.attachShader(program, vertex);
      gl.attachShader(program, fragment);
      gl.linkProgram(program);
      gl.deleteShader(vertex);
      gl.deleteShader(fragment);
      if (!gl.getProgramParameter(program, gl.LINK_STATUS)) throw new Error(gl.getProgramInfoLog(program) || "画质增强程序链接失败");
      this.program = program;
      gl.useProgram(program);

      const buffer = gl.createBuffer();
      if (!buffer) throw new Error("无法创建画质增强顶点缓冲区");
      this.buffer = buffer;
      gl.bindBuffer(gl.ARRAY_BUFFER, buffer);
      gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 1, -1, -1, 1, -1, 1, 1, -1, 1, 1]), gl.STATIC_DRAW);
      const position = gl.getAttribLocation(program, "aPosition");
      gl.enableVertexAttribArray(position);
      gl.vertexAttribPointer(position, 2, gl.FLOAT, false, 0, 0);

      const texture = gl.createTexture();
      if (!texture) throw new Error("无法创建视频纹理");
      this.texture = texture;
      gl.bindTexture(gl.TEXTURE_2D, texture);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
      gl.uniform1i(gl.getUniformLocation(program, "uFrame"), 0);
      gl.uniform1f(gl.getUniformLocation(program, "uStrength"), ENHANCEMENT_PROFILES[this.mode].strength);

      const redraw = () => this.render();
      for (const event of ["loadeddata", "loadedmetadata", "seeked", "resize", "play"]) {
        this.video.addEventListener(event, redraw);
        this.listeners.push(() => this.video.removeEventListener(event, redraw));
      }
      this.render();
      this.schedule();
    } catch (error) {
      this.onStatus("error", error instanceof Error ? error.message : String(error));
      this.destroy();
    }
  }

  private schedule(): void {
    if (this.destroyed) return;
    const rvfcVideo = this.video as HTMLVideoElement & {
      requestVideoFrameCallback?: (callback: () => void) => number;
    };
    if (typeof rvfcVideo.requestVideoFrameCallback === "function") {
      this.videoFrameHandle = rvfcVideo.requestVideoFrameCallback(() => {
        this.videoFrameHandle = null;
        this.render();
        this.schedule();
      });
    } else {
      this.frameHandle = requestAnimationFrame(() => {
        this.frameHandle = null;
        this.render();
        this.schedule();
      });
    }
  }

  private render(): void {
    const gl = this.gl;
    const program = this.program;
    if (!gl || !program || this.destroyed || this.video.readyState < 2 || !this.video.videoWidth || !this.video.videoHeight) return;
    try {
      const target = resolveEnhancementSize(
        this.video.videoWidth,
        this.video.videoHeight,
        this.canvas.clientWidth || this.video.clientWidth,
        this.canvas.clientHeight || this.video.clientHeight,
        typeof devicePixelRatio === "number" ? devicePixelRatio : 1,
        this.mode,
      );
      if (this.canvas.width !== target.width || this.canvas.height !== target.height) {
        this.canvas.width = target.width;
        this.canvas.height = target.height;
      }
      gl.viewport(0, 0, target.width, target.height);
      gl.clearColor(0, 0, 0, 1);
      gl.clear(gl.COLOR_BUFFER_BIT);
      gl.useProgram(program);
      gl.activeTexture(gl.TEXTURE0);
      gl.bindTexture(gl.TEXTURE_2D, this.texture);
      gl.pixelStorei(gl.UNPACK_FLIP_Y_WEBGL, 1);
      gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, this.video);
      gl.uniform2f(gl.getUniformLocation(program, "uTexel"), 1 / this.video.videoWidth, 1 / this.video.videoHeight);
      gl.drawArrays(gl.TRIANGLES, 0, 6);
      if (!this.hasRendered) {
        this.hasRendered = true;
        this.onStatus("ready");
      }
    } catch (error) {
      this.onStatus("error", `本地画质增强已降级：${error instanceof Error ? error.message : String(error)}`);
      this.destroy();
    }
  }

  destroy(): void {
    if (this.destroyed) return;
    this.destroyed = true;
    if (this.frameHandle != null) cancelAnimationFrame(this.frameHandle);
    const rvfcVideo = this.video as HTMLVideoElement & { cancelVideoFrameCallback?: (handle: number) => void };
    if (this.videoFrameHandle != null) rvfcVideo.cancelVideoFrameCallback?.(this.videoFrameHandle);
    for (const release of this.listeners.splice(0)) release();
    if (this.gl && this.texture) this.gl.deleteTexture(this.texture);
    if (this.gl && this.buffer) this.gl.deleteBuffer(this.buffer);
    if (this.gl && this.program) this.gl.deleteProgram(this.program);
    this.gl = null;
    this.program = null;
    this.texture = null;
    this.buffer = null;
  }
}
