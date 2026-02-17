export default function StatusIndicator({ state }) {
  const isListening = state === "Listening";
  const isReady = state === "Ready";

  let dotClass = "status-dot";
  if (isListening) dotClass += " listening";
  else if (isReady) dotClass += " ready";

  return (
    <div className="status-indicator">
      <span className={dotClass} />
      {state}
    </div>
  );
}
