import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
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
        const nextStatus = String(state?.status ?? "Idle");
        const nextLevel = Number(state?.input_level ?? 0);
        setStatus((prev) => (prev === nextStatus ? prev : nextStatus));
        setInputLevel((prev) => (Math.abs(prev - nextLevel) < 0.001 ? prev : nextLevel));
      } catch {
        // ignore poll errors
      }
    }, 220);

    return () => {
      if (unlistenStatus) unlistenStatus();
      if (unlistenLevel) unlistenLevel();
      if (pollTimer) clearInterval(pollTimer);
    };
  }, []);

  function handleDrag(event) {
    if (typeof event.button === "number" && event.button !== 0) return;
    event.preventDefault();
    void invoke("start_floating_drag").catch(() => {
      // fallback: ignore if startDragging not available
    });
  }

  return (
    <main className="floating-page">
      <FloatingMicWidget status={status} level={inputLevel} onDrag={handleDrag} />
    </main>
  );
}
