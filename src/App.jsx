import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import DashboardPage from "./pages/DashboardPage.jsx";
import ModelDownloadPage from "./pages/ModelDownloadPage.jsx";
import FloatingWidgetPage from "./pages/FloatingWidgetPage.jsx";

const initialProgress = {
  downloaded_bytes: 0,
  total_bytes: null,
  progress_percent: 0,
};

export default function App() {
  const [windowLabel] = useState(() => getCurrentWindow().label);
  const [phase, setPhase] = useState("booting");
  const [progress, setProgress] = useState(initialProgress);
  const [bootError, setBootError] = useState("");
  const [bootstrapTick, setBootstrapTick] = useState(0);

  useEffect(() => {
    if (windowLabel === "floating") {
      return () => {};
    }

    let unlisten;

    async function bootstrap() {
      try {
        setBootError("");
        setProgress(initialProgress);
        unlisten = await listen("model-download-progress", (event) => {
          setProgress(event.payload);
        });

        await invoke("ensure_model_ready");
        setPhase("ready");
      } catch (error) {
        setBootError(String(error));
        setPhase("error");
      }
    }

    setPhase("downloading");
    bootstrap();

    return () => {
      if (unlisten) {
        unlisten();
      }
    };
  }, [windowLabel, bootstrapTick]);

  if (windowLabel === "floating") {
    return <FloatingWidgetPage />;
  }

  if (phase !== "ready") {
    return (
      <ModelDownloadPage
        progress={progress}
        error={bootError}
        onRetry={() => setBootstrapTick((n) => n + 1)}
      />
    );
  }

  return <DashboardPage />;
}
