const MicSvg = () => (
  <svg viewBox="0 0 24 24"><path d="M12 14a3 3 0 0 0 3-3V5a3 3 0 0 0-6 0v6a3 3 0 0 0 3 3Zm5-3a5 5 0 0 1-10 0H5a7 7 0 0 0 6 6.93V21h2v-3.07A7 7 0 0 0 19 11h-2Z" /></svg>
);

const BAR_COUNT = 5;

export default function FloatingMicWidget({ status, level, onDrag }) {
  const safeLevel = Math.max(0, Math.min(1, Number(level) || 0));
  const isLive = status === "Listening";

  return (
    <div
      className={`floating-mic${isLive ? " live" : ""}`}
      data-tauri-drag-region
      onPointerDown={onDrag}
      onMouseDown={onDrag}
    >
      {/* Drag handle */}
      <div className="floating-drag-handle" data-tauri-drag-region onPointerDown={onDrag}>
        <span />
        <span />
        <span />
      </div>

      {/* Mic icon */}
      <div className="floating-mic-icon" data-tauri-drag-region onPointerDown={onDrag}>
        <MicSvg />
      </div>

      {/* Waveform bars */}
      <div className="floating-mic-wave" data-tauri-drag-region onPointerDown={onDrag}>
        {Array.from({ length: BAR_COUNT }, (_, i) => {
          const weights = [0.4, 0.7, 1.0, 0.7, 0.4];
          const h = isLive ? 4 + safeLevel * 14 * weights[i] : 4;
          return <span key={i} style={{ height: `${h}px` }} />;
        })}
      </div>

      {/* Label */}
      <div className="floating-mic-label" data-tauri-drag-region onPointerDown={onDrag}>{isLive ? "Listening…" : "Idle"}</div>
    </div>
  );
}
