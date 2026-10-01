<script lang="ts">
  import { updaterStore } from "$lib/updater.svelte";
  import { t } from "$lib/i18n/index.svelte";

  function formatBytes(bytes: number): string {
    if (bytes === 0) return "0 MB";
    const mb = bytes / (1024 * 1024);
    return `${mb.toFixed(1)} MB`;
  }
</script>

{#if updaterStore.showModal}
  <div class="modal-overlay">
    <div class="modal-content update-modal">
      {#if updaterStore.status === 'available'}
        <div class="update-header">
          <div class="icon-wrap info-icon">
            <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
              <polyline points="7 10 12 15 17 10" />
              <line x1="12" y1="15" x2="12" y2="3" />
            </svg>
          </div>
          <div>
            <h3 class="update-title">{t("updateAvailableTitle")}</h3>
            <p class="update-subtitle">{t("updateAvailableDesc")}</p>
          </div>
        </div>

        <div class="version-badges">
          <div class="version-box current">
            <span class="version-label">{t("currentVersion")}</span>
            <span class="version-value">{updaterStore.currentVersion || "0.1.0"}</span>
          </div>
          <div class="version-arrow">
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <line x1="5" y1="12" x2="19" y2="12" />
              <polyline points="12 5 19 12 12 19" />
            </svg>
          </div>
          <div class="version-box new">
            <span class="version-label">{t("newVersion")}</span>
            <span class="version-value">{updaterStore.newVersion}</span>
          </div>
        </div>

        {#if updaterStore.releaseNotes}
          <div class="release-notes-section">
            <span class="notes-heading">{t("releaseNotes")}</span>
            <div class="release-notes-body">
              {updaterStore.releaseNotes}
            </div>
          </div>
        {/if}

        <div class="modal-actions">
          <button type="button" class="btn btn-secondary" onclick={() => updaterStore.closeModal()}>
            {t("updateLater")}
          </button>
          <button type="button" class="btn btn-primary" onclick={() => updaterStore.startDownloadAndInstall()}>
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
              <polyline points="7 10 12 15 17 10" />
              <line x1="12" y1="15" x2="12" y2="3" />
            </svg>
            <span>{t("updateNow")}</span>
          </button>
        </div>

      {:else if updaterStore.status === 'downloading'}
        <div class="update-header">
          <div class="icon-wrap spin-icon">
            <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <line x1="12" y1="2" x2="12" y2="6" />
              <line x1="12" y1="18" x2="12" y2="22" />
              <line x1="4.93" y1="4.93" x2="7.76" y2="7.76" />
              <line x1="16.24" y1="16.24" x2="19.07" y2="19.07" />
              <line x1="2" y1="12" x2="6" y2="12" />
              <line x1="18" y1="12" x2="22" y2="12" />
              <line x1="4.93" y1="19.07" x2="7.76" y2="16.24" />
              <line x1="16.24" y1="7.76" x2="19.07" y2="4.93" />
            </svg>
          </div>
          <div>
            <h3 class="update-title">{t("downloadingUpdate")}</h3>
            <p class="update-subtitle">
              {#if updaterStore.totalBytes > 0}
                {formatBytes(updaterStore.downloadedBytes)} / {formatBytes(updaterStore.totalBytes)} ({updaterStore.progress}%)
              {:else}
                {updaterStore.progress}%
              {/if}
            </p>
          </div>
        </div>

        <div class="progress-bar-container">
          <div class="progress-bar-fill" style="width: {updaterStore.progress}%;"></div>
        </div>

      {:else if updaterStore.status === 'ready'}
        <div class="update-header">
          <div class="icon-wrap success-icon">
            <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <polyline points="20 6 9 17 4 12" />
            </svg>
          </div>
          <div>
            <h3 class="update-title">{t("updateReady")}</h3>
            <p class="update-subtitle">{t("updateReadyDesc")}</p>
          </div>
        </div>

        <div class="modal-actions">
          <button type="button" class="btn btn-primary w-full" onclick={() => updaterStore.relaunchApp()}>
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <polyline points="1 4 1 10 7 10" />
              <path d="M3.51 15a9 9 0 1 0 2.13-9.36L1 10" />
            </svg>
            <span>{t("restartNow")}</span>
          </button>
        </div>

      {:else if updaterStore.status === 'up-to-date'}
        <div class="update-header">
          <div class="icon-wrap success-icon">
            <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14" />
              <polyline points="22 4 12 14.01 9 11.01" />
            </svg>
          </div>
          <div>
            <h3 class="update-title">{t("appUpToDate")}</h3>
            <p class="update-subtitle">{t("appUpToDateDesc")}</p>
          </div>
        </div>

        <div class="modal-actions">
          <button type="button" class="btn btn-secondary w-full" onclick={() => updaterStore.closeModal()}>
            {t("close")}
          </button>
        </div>

      {:else if updaterStore.status === 'error'}
        <div class="update-header">
          <div class="icon-wrap error-icon">
            <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <circle cx="12" cy="12" r="10" />
              <line x1="12" y1="8" x2="12" y2="12" />
              <line x1="12" y1="16" x2="12.01" y2="16" />
            </svg>
          </div>
          <div>
            <h3 class="update-title">{t("updateError")}</h3>
            <p class="alert-clean-error">{updaterStore.errorMessage}</p>
          </div>
        </div>

        <div class="modal-actions">
          <button type="button" class="btn btn-secondary w-full" onclick={() => updaterStore.closeModal()}>
            {t("close")}
          </button>
        </div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .update-modal {
    max-width: 480px;
    width: 90%;
    padding: 1.5rem;
    border-radius: 12px;
    background: #ffffff;
    box-shadow: 0 12px 32px rgba(0, 0, 0, 0.15);
  }

  .update-header {
    display: flex;
    align-items: flex-start;
    gap: 1rem;
    margin-bottom: 1.25rem;
  }

  .icon-wrap {
    width: 44px;
    height: 44px;
    border-radius: 10px;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .info-icon {
    background: #e0f2fe;
    color: #0284c7;
  }

  .spin-icon {
    background: #f0fdf4;
    color: var(--color-primary, #059669);
    animation: spin 1.5s linear infinite;
  }

  .success-icon {
    background: #ecfdf5;
    color: #059669;
  }

  .error-icon {
    background: #fef2f2;
    color: var(--color-danger, #dc2626);
  }

  @keyframes spin {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }

  .update-title {
    margin: 0;
    font-size: 1.15rem;
    font-weight: 700;
    color: #1e293b;
  }

  .update-subtitle {
    margin: 0.25rem 0 0 0;
    font-size: 0.88rem;
    color: #64748b;
  }

  .alert-clean-error {
    margin: 0.35rem 0 0 0;
    font-size: 0.88rem;
    color: var(--color-danger, #dc2626);
    font-weight: 500;
  }

  .version-badges {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
    padding: 0.75rem 1rem;
    background: #f8fafc;
    border: 1px solid #e2e8f0;
    border-radius: 8px;
    margin-bottom: 1rem;
  }

  .version-box {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
  }

  .version-label {
    font-size: 0.75rem;
    color: #64748b;
    text-transform: uppercase;
    font-weight: 600;
  }

  .version-value {
    font-size: 0.95rem;
    font-weight: 700;
    color: #1e293b;
  }

  .version-box.new .version-value {
    color: var(--color-primary, #059669);
  }

  .version-arrow {
    color: #94a3b8;
  }

  .release-notes-section {
    margin-bottom: 1.25rem;
  }

  .notes-heading {
    display: block;
    font-size: 0.8rem;
    font-weight: 600;
    color: #475569;
    margin-bottom: 0.4rem;
    text-transform: uppercase;
  }

  .release-notes-body {
    max-height: 120px;
    overflow-y: auto;
    font-size: 0.85rem;
    color: #334155;
    background: #f8fafc;
    border: 1px solid #e2e8f0;
    border-radius: 6px;
    padding: 0.6rem 0.75rem;
    white-space: pre-wrap;
    line-height: 1.45;
  }

  .progress-bar-container {
    width: 100%;
    height: 10px;
    background: #e2e8f0;
    border-radius: 5px;
    overflow: hidden;
    margin: 1.25rem 0 0.5rem 0;
  }

  .progress-bar-fill {
    height: 100%;
    background: var(--color-primary, #059669);
    transition: width 0.25s ease-out;
    border-radius: 5px;
  }

  .modal-actions {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 0.75rem;
    margin-top: 1.25rem;
  }

  .w-full {
    width: 100%;
    justify-content: center;
  }

  .btn {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.65rem 1.15rem;
    border-radius: 8px;
    font-size: 0.9rem;
    font-weight: 600;
    cursor: pointer;
    border: none;
    transition: all 0.2s ease;
  }

  .btn-primary {
    background: var(--color-primary, #059669);
    color: #ffffff;
  }

  .btn-primary:hover {
    background: var(--color-primary-hover, #047857);
  }

  .btn-secondary {
    background: #f1f5f9;
    color: #475569;
    border: 1px solid #cbd5e1;
  }

  .btn-secondary:hover {
    background: #e2e8f0;
    color: #1e293b;
  }
</style>
