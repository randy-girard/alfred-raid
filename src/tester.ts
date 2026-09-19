import "./no-context-menu";
import { invoke } from "@tauri-apps/api/core";

type WatchStatus = {
  character?: string | null;
};

type InjectResult = {
  line: string;
  changed: boolean;
};

type TestChannel =
  | "shout"
  | "say"
  | "ooc"
  | "group"
  | "guild"
  | "auction"
  | "raid";

function $(id: string): HTMLElement {
  const el = document.getElementById(id);
  if (!el) throw new Error(`missing #${id}`);
  return el;
}

function isChannel(value: string): value is TestChannel {
  return (
    value === "shout" ||
    value === "say" ||
    value === "ooc" ||
    value === "group" ||
    value === "guild" ||
    value === "auction" ||
    value === "raid"
  );
}

async function refreshHint() {
  const hint = $("you-hint");
  const list = $("speaker-names");
  list.replaceChildren();
  const you = document.createElement("option");
  you.value = "YOU";
  list.append(you);
  try {
    const watch = await invoke<WatchStatus>("get_watch_status");
    const name = watch.character?.trim();
    if (name) {
      const opt = document.createElement("option");
      opt.value = name;
      list.append(opt);
      hint.textContent = `YOU uses your logged-in character (${name}).`;
    } else {
      hint.textContent =
        "YOU is used until a log file sets your character.";
    }
  } catch {
    hint.textContent =
      "YOU is used until a log file sets your character.";
  }
}

async function send() {
  const status = $("status");
  const preview = $("preview");
  const speaker = ($("speaker") as HTMLInputElement).value;
  const channelValue = ($("channel") as HTMLSelectElement).value;
  const message = ($("message") as HTMLTextAreaElement).value;
  status.textContent = "";
  preview.hidden = true;
  if (!isChannel(channelValue)) {
    status.textContent = "Pick a channel.";
    return;
  }
  try {
    const result = await invoke<InjectResult>("inject_test_line", {
      speaker,
      channel: channelValue,
      message,
    });
    preview.hidden = false;
    preview.textContent = result.line;
    status.className = "save-status ok";
    status.textContent = result.changed
      ? "Applied, same as a log line."
      : "Line was ignored (not a CH, RCH, or command).";
    ($("message") as HTMLTextAreaElement).select();
  } catch (err) {
    status.className = "save-status";
    status.textContent = String(err);
  }
}

window.addEventListener("DOMContentLoaded", () => {
  void refreshHint();
  $("send").addEventListener("click", () => {
    void send();
  });
  $("message").addEventListener("keydown", (event) => {
    if (event.key === "Enter" && !event.shiftKey) {
      event.preventDefault();
      void send();
    }
  });
  $("speaker").addEventListener("keydown", (event) => {
    if (event.key === "Enter") {
      event.preventDefault();
      void send();
    }
  });
});
