import "./no-context-menu";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  clampOverlayOpacity,
  liveSnapshot,
  overlayPanelHtml,
  CHAIN_FOCUS_KEY,
  primaryChain,
  type ChainFocus,
  type ChainSnapshot,
} from "./logic";

type RaidSnapshot = {
  chain: ChainSnapshot;
  rampage: ChainSnapshot;
};

type OverlayConfig = {
  overlayOpacity: number;
  overlayClickthrough: boolean;
};

let raid: RaidSnapshot | null = null;
let config: OverlayConfig = {
  overlayOpacity: 0.85,
  overlayClickthrough: false,
};

function $(id: string): HTMLElement {
  const el = document.getElementById(id);
  if (!el) throw new Error(`missing #${id}`);
  return el;
}

function applyChrome() {
  const alpha = clampOverlayOpacity(config.overlayOpacity);
  document.body.style.setProperty("--overlay-alpha", String(alpha));
  document.body.classList.toggle("is-clickthrough", config.overlayClickthrough);
  $("overlay-hint").textContent = config.overlayClickthrough
    ? "Click-through on · toggle it off in Settings to move"
    : "Drag anywhere to move · resize from the edge";
}

function readChainFocus(): ChainFocus | null {
  try {
    const value = localStorage.getItem(CHAIN_FOCUS_KEY);
    if (value === "chain" || value === "rampage") return value;
  } catch {
    /* private mode */
  }
  return null;
}

function render() {
  const now = Date.now();
  const chain = liveSnapshot(raid?.chain ?? null, now);
  const rampage = liveSnapshot(raid?.rampage ?? null, now);
  const ch = overlayPanelHtml(chain, "CH");
  const rch = overlayPanelHtml(rampage, "Rampage");
  $("overlay-chains").innerHTML =
    primaryChain(chain, rampage, readChainFocus()).kind === "rampage" ? rch + ch : ch + rch;
}

window.addEventListener("DOMContentLoaded", () => {
  $("overlay-close").addEventListener("click", () => {
    void invoke("hide_overlay");
  });
  $("overlay-shell").addEventListener("mousedown", (event) => {
    if (event.button !== 0 || config.overlayClickthrough) return;
    const target = event.target;
    if (target instanceof Element && target.closest("#overlay-close")) return;
    void getCurrentWindow().startDragging();
  });

  void (async () => {
    try {
      const cfg = await invoke<OverlayConfig>("get_config");
      config = {
        overlayOpacity: cfg.overlayOpacity,
        overlayClickthrough: cfg.overlayClickthrough,
      };
    } catch {
      /* keep defaults */
    }
    applyChrome();
    try {
      raid = await invoke<RaidSnapshot>("get_snapshot");
    } catch {
      raid = null;
    }
    render();
    await listen<RaidSnapshot>("raid-updated", (event) => {
      raid = event.payload;
      render();
    });
    await listen<OverlayConfig>("config-updated", (event) => {
      config = {
        overlayOpacity: event.payload.overlayOpacity,
        overlayClickthrough: event.payload.overlayClickthrough,
      };
      applyChrome();
    });
    const tick = () => {
      render();
      requestAnimationFrame(tick);
    };
    requestAnimationFrame(tick);
  })();
});
