import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import FloatingMicWidget from "../components/FloatingMicWidget.jsx";

export default function FloatingWidgetPage() {
  const [status, setStatus] = useState("Ready");
  const [inputLevel, setInputLevel] = useState(0);

  useEffect(() => {
    let unlistenStatus;
    let unlistenLevel;
    let pollTimer;

    async function wireEvents() {
      unlistenStatus = await listen("hotkey-status", (event) => {
        setStatus(String(event.payload ?? "Idle"));
      });

      unlistenLevel = await listen("input-level", (event) => {
        setInputLevel(Number(event.payload ?? 0));
      });
    }

    wireEvents();
    pollTimer = setInterval(async () => {
      try {
        const state = await invoke("get_live_state");
        setStatus(String(state?.status ?? "Idle"));
        setInputLevel(Number(state?.input_level ?? 0));
      } catch {
        // ignore poll errors
      }
    }, 60);

    return () => {
      if (unlistenStatus) unlistenStatus();
      if (unlistenLevel) unlistenLevel();
      if (pollTimer) clearInterval(pollTimer);
    };
  }, []);

  async function handleDrag(event) {
    if (event.button !== 0) return;
    event.preventDefault();
    try {
      await getCurrentWindow().startDragging();
    } catch {
      // fallback: ignore if startDragging not available
    }
  }

  return (
    <main className="floating-page">
      <FloatingMicWidget status={status} level={inputLevel} onDrag={handleDrag} />
    </main>
  );
}
