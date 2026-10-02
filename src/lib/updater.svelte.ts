import { check, type Update, type DownloadEvent } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';

export type UpdateStatus = 
  | 'idle' 
  | 'checking' 
  | 'available' 
  | 'up-to-date' 
  | 'downloading' 
  | 'ready' 
  | 'error';

class UpdaterStore {
  status = $state<UpdateStatus>('idle');
  currentVersion = $state<string>('');
  newVersion = $state<string>('');
  releaseDate = $state<string>('');
  releaseNotes = $state<string>('');
  progress = $state<number>(0);
  downloadedBytes = $state<number>(0);
  totalBytes = $state<number>(0);
  errorMessage = $state<string | null>(null);
  showModal = $state<boolean>(false);
  activeUpdate: Update | null = null;

  async checkForUpdates(silent: boolean = false) {
    if (this.status === 'checking' || this.status === 'downloading') return;

    this.status = 'checking';
    this.errorMessage = null;

    try {
      const update = await check();

      if (update) {
        this.activeUpdate = update;
        this.currentVersion = update.currentVersion;
        this.newVersion = update.version;
        this.releaseDate = update.date ?? '';
        this.releaseNotes = update.body ?? '';
        this.status = 'available';
        this.showModal = true;
      } else {
        this.status = 'up-to-date';
        if (!silent) {
          this.showModal = true;
        }
      }
    } catch (err: any) {
      console.warn('[Updater] Check result:', err);
      const msg = err?.message || String(err);
      // When no release has been published yet on GitHub, the endpoint returns 404
      if (msg.includes('Could not fetch a valid release JSON') || msg.includes('404')) {
        this.status = 'up-to-date';
        if (!silent) {
          this.showModal = true;
        }
      } else {
        this.status = 'error';
        this.errorMessage = msg;
        if (!silent) {
          this.showModal = true;
        }
      }
    }
  }

  async startDownloadAndInstall() {
    if (!this.activeUpdate || this.status === 'downloading') return;

    this.status = 'downloading';
    this.progress = 0;
    this.downloadedBytes = 0;
    this.totalBytes = 0;
    this.errorMessage = null;

    try {
      let downloaded = 0;
      let total = 0;

      await this.activeUpdate.downloadAndInstall((event: DownloadEvent) => {
        if (event.event === 'Started') {
          total = event.data.contentLength ?? 0;
          this.totalBytes = total;
          this.downloadedBytes = 0;
          this.progress = 0;
        } else if (event.event === 'Progress') {
          downloaded += event.data.chunkLength;
          this.downloadedBytes = downloaded;
          if (total > 0) {
            this.progress = Math.min(100, Math.round((downloaded / total) * 100));
          }
        } else if (event.event === 'Finished') {
          this.progress = 100;
        }
      });

      this.status = 'ready';
    } catch (err: any) {
      console.error('[Updater] Download & Install failed:', err);
      this.status = 'error';
      this.errorMessage = err?.message || String(err);
    }
  }

  async relaunchApp() {
    try {
      await relaunch();
    } catch (err) {
      console.error('[Updater] Relaunch failed:', err);
    }
  }

  closeModal() {
    this.showModal = false;
  }
}

export const updaterStore = new UpdaterStore();
