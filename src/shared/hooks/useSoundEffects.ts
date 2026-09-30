import { useEffect, useRef } from "react";
import { listen } from "@tauri-apps/api/event";

interface UseSoundEffectsOptions {
  soundEnabled: boolean;
  soundVolume: number;
  pasteSoundEnabled: boolean;
}

/** Idle time after the last beep before the AudioContext is suspended. */
const SUSPEND_AFTER_IDLE_MS = 2000;

/**
 * Volume is stored on a 0..1 scale (the settings slider's range). Older installs stored
 * 0..100, so anything above 1 is read as a percentage. Non-numeric values fall back to
 * full volume; an explicit 0 stays 0.
 */
export const normalizeSoundVolume = (raw: unknown): number => {
  const value = typeof raw === "number" ? raw : parseFloat(String(raw));
  if (!Number.isFinite(value)) return 1;
  const scaled = value > 1 ? value / 100 : value;
  return Math.min(1, Math.max(0, scaled));
};

export const useSoundEffects = ({
  soundEnabled,
  soundVolume,
  pasteSoundEnabled
}: UseSoundEffectsOptions) => {
  // Volume and paste toggle are read at play time, so changing them never rebuilds the
  // AudioContext (each rebuild re-acquires the system audio route).
  const volumeRef = useRef(soundVolume);
  const pasteEnabledRef = useRef(pasteSoundEnabled);
  volumeRef.current = soundVolume;
  pasteEnabledRef.current = pasteSoundEnabled;

  useEffect(() => {
    // Sound off: no listener and no AudioContext at all. A live context holds an output
    // stream on the audio device and, on macOS, competes for the Bluetooth route even
    // while silent (upstream #174).
    if (!soundEnabled) return;

    const AudioContextCtor =
      window.AudioContext ||
      (window as Window & { webkitAudioContext?: typeof window.AudioContext }).webkitAudioContext;
    if (!AudioContextCtor) return;

    let ctx: AudioContext | null = null;
    let suspendTimer: ReturnType<typeof setTimeout> | undefined;
    // In-flight idle suspend. A beep arriving meanwhile still reads state "running", so it
    // must wait for the suspend to settle and resume, or the suspend would cut it off.
    let suspending: Promise<void> | null = null;
    let disposed = false;

    // Created lazily on the first beep, suspended again once idle so the device is released.
    const getContext = () => {
      if (!ctx) ctx = new AudioContextCtor();
      return ctx;
    };

    const scheduleSuspend = () => {
      if (suspendTimer) clearTimeout(suspendTimer);
      suspendTimer = setTimeout(() => {
        if (!ctx || ctx.state !== "running") return;
        suspending = ctx
          .suspend()
          .catch(() => {})
          .finally(() => {
            suspending = null;
          });
      }, SUSPEND_AFTER_IDLE_MS);
    };

    const playCrispBeep = (c: AudioContext, durationSec = 0.1, baseFreqHz = 1400, volume = 0.35) => {
      const t0 = c.currentTime;
      const tEnd = t0 + Math.max(0.05, durationSec);

      const osc = c.createOscillator();
      osc.type = "triangle";

      osc.frequency.setValueAtTime(baseFreqHz * 1.25, t0);
      osc.frequency.exponentialRampToValueAtTime(
        Math.max(80, baseFreqHz * 0.92),
        t0 + Math.min(0.18, durationSec * 0.25)
      );

      const filter = c.createBiquadFilter();
      filter.type = "bandpass";
      filter.frequency.setValueAtTime(Math.min(4000, baseFreqHz * 1.3), t0);
      filter.Q.setValueAtTime(6, t0);

      const gain = c.createGain();
      gain.gain.setValueAtTime(0.0001, t0);
      gain.gain.exponentialRampToValueAtTime(Math.max(0.0001, volume), t0 + 0.004);

      const mid = t0 + Math.min(0.08, durationSec * 0.2);
      gain.gain.exponentialRampToValueAtTime(Math.max(0.0001, volume * 0.35), mid);
      gain.gain.exponentialRampToValueAtTime(0.0001, tEnd);

      const noiseDur = Math.min(0.03, durationSec * 0.1);
      const sampleRate = c.sampleRate || 44100;
      const bufferSize = Math.floor(sampleRate * noiseDur);

      let noiseBuf: AudioBuffer | undefined;
      try {
        noiseBuf = c.createBuffer(1, bufferSize > 0 ? bufferSize : 1, sampleRate);
        const data = noiseBuf.getChannelData(0);
        for (let i = 0; i < data.length; i++) {
          const decay = 1 - i / data.length;
          data[i] = (Math.random() * 2 - 1) * decay;
        }
      } catch (e) {
        console.error("Audio buffer error", e);
      }

      if (noiseBuf) {
        const noiseNode = c.createBufferSource();
        noiseNode.buffer = noiseBuf;

        const noiseHP = c.createBiquadFilter();
        noiseHP.type = "highpass";
        noiseHP.frequency.setValueAtTime(1500, t0);

        const noiseGain = c.createGain();
        noiseGain.gain.setValueAtTime(Math.max(0.0001, volume * 0.25), t0);
        noiseGain.gain.exponentialRampToValueAtTime(0.0001, t0 + noiseDur);

        noiseNode.connect(noiseHP);
        noiseHP.connect(noiseGain);
        noiseGain.connect(c.destination);

        noiseNode.start(t0);
        noiseNode.stop(t0 + noiseDur);
      }

      osc.connect(filter);
      filter.connect(gain);
      gain.connect(c.destination);

      osc.start(t0);
      osc.stop(tEnd + 0.01);
    };

    const unlisten = listen<string>("play-sound", (event) => {
      const type = event.payload;
      if (type !== "copy" && type !== "paste") return;
      if (type === "paste" && !pasteEnabledRef.current) return;
      // The slider is 0..1; the old `/ 100` made full volume a gain of 0.01 (upstream #94).
      const masterVol = normalizeSoundVolume(volumeRef.current);
      if (masterVol <= 0) return;

      const c = getContext();
      const play = () => {
        if (disposed || c.state === "closed") return;
        try {
          if (type === "copy") {
            playCrispBeep(c, 0.06, 500, Math.min(1, masterVol * 0.8));
          } else {
            playCrispBeep(c, 0.09, 950, Math.min(1, masterVol * 0.9));
            setTimeout(() => {
              if (!disposed && c.state === "running") {
                playCrispBeep(c, 0.075, 1150, Math.min(1, masterVol * 0.75));
              }
            }, 110);
          }
        } catch (e) {
          console.error("Sound play error", e);
        }
        scheduleSuspend();
      };

      // Cancel a pending idle suspend: this beep keeps the context busy.
      if (suspendTimer) clearTimeout(suspendTimer);
      const settled = suspending ?? Promise.resolve();
      settled
        .then(() => (c.state === "suspended" ? c.resume() : undefined))
        .then(play)
        .catch((err) => {
          console.error("Failed to resume audio ctx", err);
          play();
        });
    });

    return () => {
      disposed = true;
      if (suspendTimer) clearTimeout(suspendTimer);
      unlisten.then((f) => f());
      if (ctx) ctx.close().catch(() => {});
    };
  }, [soundEnabled]);
};
