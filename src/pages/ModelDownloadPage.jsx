function formatBytes(bytes) {
  if (typeof bytes !== "number" || bytes < 0) return "0 B";

  const units = ["B", "KB", "MB", "GB"];
  let value = bytes;
  let idx = 0;
  while (value >= 1024 && idx < units.length - 1) {
    value /= 1024;
    idx += 1;
  }

  return `${value.toFixed(idx === 0 ? 0 : 1)} ${units[idx]}`;
}

const MicSvg = () => (
  <svg viewBox="0 0 24 24" style={{ width: 28, height: 28, fill: '#137fec' }}>
    <path d="M12 14a3 3 0 0 0 3-3V5a3 3 0 0 0-6 0v6a3 3 0 0 0 3 3Zm5-3a5 5 0 0 1-10 0H5a7 7 0 0 0 6 6.93V21h2v-3.07A7 7 0 0 0 19 11h-2Z" />
  </svg>
);

export default function ModelDownloadPage({ progress, error, onRetry }) {
  const percent = Math.max(0, Math.min(100, progress?.progress_percent ?? 0));
  const downloaded = formatBytes(progress?.downloaded_bytes ?? 0);
  const total =
    progress?.total_bytes == null ? "unknown size" : formatBytes(progress.total_bytes);

  return (
    <main className="download-shell">
      <div className="download-brand">
        <MicSvg />
        <h1>Quan Voice</h1>
      </div>
      <p className="download-tagline">Offline Voice. Unlimited Focus.</p>

      <div className="download-card">
        <h2>Downloading Speech Model…</h2>
        <p>Preparing local whisper model for first launch.</p>
        <div className="progress-track" aria-label="Model download progress">
          <div className="progress-fill" style={{ width: `${percent}%` }} />
        </div>
        <p className="progress-meta">
          {percent.toFixed(1)}% ({downloaded} / {total})
        </p>
        {error ? (
          <>
            <div className="error-banner" style={{ marginTop: 16 }}>{error}</div>
            <button className="primary-btn" type="button" onClick={onRetry} style={{ marginTop: 16 }}>
              Retry Download
            </button>
          </>
        ) : null}
      </div>
    </main>
  );
}
