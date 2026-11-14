/*
 * Genuine Mojang Minecraft 1.21.4 OGG Vorbis sound playback via Tauri cache + Web Audio / Blob pool.
 * Audio bytes are sourced directly from local Vanilla / Minecraft asset caches or Mojang CDN into app data.
 */

import * as ipc from "./ipc";

export type McSoundKind =
  | "click"
  | "focus"
  | "tab"
  | "toggleOn"
  | "toggleOff"
  | "launch"
  | "back"
  | "error";

const SOUND_KINDS: readonly McSoundKind[] = [
  "click",
  "focus",
  "tab",
  "toggleOn",
  "toggleOff",
  "launch",
  "back",
  "error",
];

const SOUND_GAIN: Record<McSoundKind, number> = {
  click: 0.62,
  focus: 0.42,
  tab: 0.56,
  toggleOn: 0.54,
  toggleOff: 0.56,
  launch: 0.66,
  back: 0.55,
  error: 0.62,
};

let audioCtx: AudioContext | null = null;
let soundsEnabled = false;
let initialized = false;
let lastFocusAt = 0;

const rawOggBuffers = new Map<McSoundKind, ArrayBuffer>();
const decodedAudioBuffers = new Map<McSoundKind, AudioBuffer>();
const audioPools = new Map<McSoundKind, HTMLAudioElement[]>();
const blobUrls = new Map<McSoundKind, string>();
const poolCursor = new Map<McSoundKind, number>();
let decodingStarted = false;

export function setSoundsEnabled(enabled: boolean): void {
  soundsEnabled = enabled;
}

export function isSoundsEnabled(): boolean {
  return soundsEnabled;
}

export function getLoadedMcSoundCount(): number {
  return rawOggBuffers.size;
}

function decodeBase64DataUrlToBytes(dataUrl: string): Uint8Array | null {
  const comma = dataUrl.indexOf(",");
  if (comma < 0) return null;
  const b64 = dataUrl.slice(comma + 1);
  try {
    const bin = atob(b64);
    const bytes = new Uint8Array(bin.length);
    for (let i = 0; i < bin.length; i += 1) {
      bytes[i] = bin.charCodeAt(i);
    }
    return bytes;
  } catch {
    return null;
  }
}

function getOrCreateAudioContext(): AudioContext | null {
  if (typeof window === "undefined") return null;
  const Ctor =
    window.AudioContext ||
    (window as unknown as { webkitAudioContext?: typeof AudioContext }).webkitAudioContext;
  if (!Ctor) return null;
  if (!audioCtx || audioCtx.state === "closed") {
    audioCtx = new Ctor();
    decodingStarted = false;
  }
  return audioCtx;
}

function decodeAllOggIntoWebAudio(ctx: AudioContext): void {
  if (decodingStarted) return;
  decodingStarted = true;
  for (const [kind, buf] of rawOggBuffers.entries()) {
    if (decodedAudioBuffers.has(kind)) continue;
    void ctx
      .decodeAudioData(buf.slice(0))
      .then((decoded) => {
        decodedAudioBuffers.set(kind, decoded);
      })
      .catch(() => {
        // Fallback stays on HTMLAudioElement blob pool for this sound
      });
  }
}

function ingestSoundsMap(sounds: Record<string, string>): void {
  for (const kind of SOUND_KINDS) {
    const dataUrl = sounds[kind];
    if (!dataUrl || rawOggBuffers.has(kind)) continue;
    const bytes = decodeBase64DataUrlToBytes(dataUrl);
    if (!bytes || bytes.length < 32) continue;

    const ab = new Uint8Array(bytes).buffer;
    rawOggBuffers.set(kind, ab);

    const prevUrl = blobUrls.get(kind);
    if (prevUrl) {
      URL.revokeObjectURL(prevUrl);
    }
    const blob = new Blob([ab], { type: "audio/ogg" });
    const blobUrl = URL.createObjectURL(blob);
    blobUrls.set(kind, blobUrl);

    const pool: HTMLAudioElement[] = [];
    for (let i = 0; i < 3; i += 1) {
      const el = new Audio(blobUrl);
      el.preload = "auto";
      el.volume = SOUND_GAIN[kind];
      pool.push(el);
    }
    audioPools.set(kind, pool);
    poolCursor.set(kind, 0);
  }

  if (audioCtx && audioCtx.state === "running") {
    decodingStarted = false;
    decodeAllOggIntoWebAudio(audioCtx);
  }
}

export async function refreshMinecraftSounds(): Promise<void> {
  try {
    const dto = await ipc.loadMinecraftSounds();
    ingestSoundsMap(dto.sounds);
  } catch {
    // Non-Tauri browser preview or unavailable local store
  }
}

export function initMinecraftSounds(): void {
  if (initialized) return;
  initialized = true;
  void refreshMinecraftSounds();
  void ipc.onMinecraftSoundsReady(() => {
    void refreshMinecraftSounds();
  });
}

export function unlockAudioContext(): void {
  const ctx = getOrCreateAudioContext();
  if (!ctx) return;
  if (ctx.state === "suspended") {
    void ctx.resume().then(() => {
      if (ctx.state === "running" && rawOggBuffers.size > 0) {
        decodeAllOggIntoWebAudio(ctx);
      }
    }).catch(() => {});
  } else if (ctx.state === "running" && rawOggBuffers.size > 0 && decodedAudioBuffers.size < rawOggBuffers.size) {
    decodeAllOggIntoWebAudio(ctx);
  }
}

function playViaHtmlAudioPool(kind: McSoundKind): boolean {
  const pool = audioPools.get(kind);
  if (!pool || pool.length === 0) return false;
  const cursor = poolCursor.get(kind) ?? 0;
  const el = pool[cursor % pool.length];
  poolCursor.set(kind, (cursor + 1) % pool.length);
  try {
    el.pause();
    el.currentTime = 0;
    el.volume = SOUND_GAIN[kind];
    const p = el.play();
    if (p && typeof p.catch === "function") {
      p.catch(() => {});
    }
    return true;
  } catch {
    return false;
  }
}

export function playMcSound(kind: McSoundKind, force = false): void {
  if (!force && !soundsEnabled) return;
  if (typeof window === "undefined") return;

  if (kind === "focus") {
    const now = performance.now();
    if (now - lastFocusAt < 45) return;
    lastFocusAt = now;
  }

  const ctx = getOrCreateAudioContext();
  const decoded = decodedAudioBuffers.get(kind);
  if (ctx && decoded && ctx.state === "running") {
    try {
      const src = ctx.createBufferSource();
      src.buffer = decoded;
      const gain = ctx.createGain();
      gain.gain.value = SOUND_GAIN[kind];
      src.connect(gain);
      gain.connect(ctx.destination);
      src.start(0);
      return;
    } catch {
      // Fall through to HTMLAudioElement pool
    }
  }

  if (ctx && ctx.state === "suspended") {
    void ctx.resume().catch(() => {});
  }

  playViaHtmlAudioPool(kind);
}
