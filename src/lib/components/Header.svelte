<script lang="ts">
  import { t, langState } from "$lib/i18n/index.svelte";
  import { onMount } from "svelte";
  import lockIcon from "../../public/icons/lock.png";
  import lockOpenIcon from "../../public/icons/lock-open.png";
  import adminIcon from "../../public/icons/admin.png";
  import cashierIcon from "../../public/icons/cashier.png";
  import { invoke } from "@tauri-apps/api/core";
  import notificationsIcon from "../../public/icons/notifications.png";
  import closepIcon from "../../public/icons/closep.png";

  let { userRole, onUnlockAdmin, onLockAdmin } = $props<{
    userRole: "cashier" | "admin";
    onUnlockAdmin: () => void;
    onLockAdmin: () => void;
  }>();

  interface Notification {
    id: string;
    type_: string;
    message_fr: string;
    message_ar: string;
    severity: "danger" | "warning" | "info";
  }

  let timeString = $state("");
  let notifications = $state<Notification[]>([]);
  let dismissedIds = $state<Set<string>>(new Set());
  let showDropdown = $state(false);

  let activeNotifications = $derived(
    notifications.filter(n => !dismissedIds.has(n.id))
  );

  function updateTime() {
    const now = new Date();
    // Simple bilingual date format
    const options: Intl.DateTimeFormatOptions = {
      weekday: "long",
      year: "numeric",
      month: "long",
      day: "numeric",
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit",
    };
    timeString = now.toLocaleDateString(langState.current === "ar" ? "ar-DZ" : "fr-DZ", options);
  }

  async function fetchNotifications() {
    try {
      const list = await invoke<Notification[]>("get_notifications");
      notifications = list;
    } catch (err) {
      console.error("Failed to fetch notifications:", err);
    }
  }

  function toggleDropdown() {
    showDropdown = !showDropdown;
  }

  function dismissNotification(id: string) {
    dismissedIds.add(id);
    dismissedIds = new Set(dismissedIds);
  }

  function clearAll() {
    activeNotifications.forEach(n => dismissedIds.add(n.id));
    dismissedIds = new Set(dismissedIds);
  }

  // Click outside detector
  let headerRef: HTMLElement | null = null;
  function handleClickOutside(event: MouseEvent) {
    if (showDropdown && headerRef && !headerRef.contains(event.target as Node)) {
      showDropdown = false;
    }
  }

  onMount(() => {
    updateTime();
    const timeInterval = setInterval(updateTime, 1000);

    fetchNotifications();
    const notificationsInterval = setInterval(fetchNotifications, 15000);

    window.addEventListener("click", handleClickOutside);

    return () => {
      clearInterval(timeInterval);
      clearInterval(notificationsInterval);
      window.removeEventListener("click", handleClickOutside);
    };
  });
</script>

<header class="app-header" bind:this={headerRef}>
  <div class="header-logo-section">
    <div class="app-icon">
      <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke="currentColor">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19.428 15.428a2 2 0 00-1.022-.547l-2.387-.477a6 6 0 00-3.86.517l-.318.158a6 6 0 01-3.86.517L6.05 15.21a2 2 0 00-1.806.547M8 4h8l-1 1v5.172a2 2 0 00.586 1.414l5 5c1.26 1.26.367 3.414-1.415 3.414H4.828c-1.782 0-2.674-2.154-1.414-3.414l5-5A2 2 0 009 10.172V5L8 4z" />
      </svg>
    </div>
    <span class="app-title">{t("appName")}</span>
  </div>

  <div class="header-time-section">
    <span class="live-time">{timeString}</span>
  </div>

  <div class="header-controls-section">
    <!-- Notifications Icon and Dropdown -->
    <div class="notifications-wrapper">
      <button 
        type="button"
        class="notifications-btn" 
        onclick={toggleDropdown} 
        aria-label={t("notifications")}
        title={t("notifications")}
      >
        <img src={notificationsIcon} alt="Notifications" class="control-icon-img" />
        {#if activeNotifications.length > 0}
          <span class="notifications-badge">{activeNotifications.length}</span>
        {/if}
      </button>

      {#if showDropdown}
        <div class="notifications-dropdown">
          <div class="dropdown-header">
            <h3>{t("notifications")}</h3>
            {#if activeNotifications.length > 0}
              <button type="button" class="btn-clear-all" onclick={clearAll}>{t("clearAll")}</button>
            {/if}
          </div>
          <hr class="dropdown-divider" />
          <div class="dropdown-body">
            {#if activeNotifications.length === 0}
              <p class="empty-notifications">{t("noNotifications")}</p>
            {:else}
              <ul class="notifications-list">
                {#each activeNotifications as notif}
                  <li class="notification-item {notif.severity}">
                    <span class="severity-dot {notif.severity}"></span>
                    <span class="notification-msg">
                      {langState.current === 'ar' ? notif.message_ar : notif.message_fr}
                    </span>
                    <button type="button" class="btn-dismiss-notif" onclick={() => dismissNotification(notif.id)} aria-label="Dismiss">
                      <img src={closepIcon} alt="Dismiss" class="dismiss-icon-img" />
                    </button>
                  </li>
                {/each}
              </ul>
            {/if}
          </div>
        </div>
      {/if}
    </div>

    <!-- Language Toggle -->
    <div class="lang-selector">
      <button 
        class="lang-btn {langState.current === 'fr' ? 'active' : ''}" 
        onclick={() => langState.setLanguage('fr')}
      >
        FR
      </button>
      <button 
        class="lang-btn {langState.current === 'ar' ? 'active' : ''}" 
        onclick={() => langState.setLanguage('ar')}
      >
        عربي
      </button>
    </div>

    <!-- Admin Status / Unlock Button -->
    <div class="role-badge-section">
      {#if userRole === "admin"}
        <div class="badge badge-admin">
          <img src={adminIcon} alt="Admin" class="role-icon-img" />
          <span>{t("admin")}</span>
        </div>
        <button onclick={onLockAdmin} class="btn-lock-pos" title="Lock to Cashier">
          <img src={lockOpenIcon} alt="Unlock" class="lock-icon-img" />
        </button>
      {:else}
        <div class="badge badge-cashier">
          <img src={cashierIcon} alt="Cashier" class="role-icon-img" />
          <span>{t("cashier")}</span>
        </div>
        <button onclick={onUnlockAdmin} class="btn-unlock-pos" aria-label={t("adminMode")} title={t("adminMode")}>
          <img src={lockIcon} alt="Lock" class="lock-icon-img" />
        </button>
      {/if}
    </div>
  </div>
</header>

<style>
  .app-header {
    background-color: var(--color-bg-card);
    border-bottom: var(--border-width) solid var(--color-border);
    height: clamp(60px, 8.5vh, 80px);
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 clamp(0.75rem, 2vw, 2rem);
    box-shadow: 0 2px 4px var(--color-shadow);
    gap: clamp(0.5rem, 1.5vw, 1.5rem);
  }

  .header-logo-section {
    display: flex;
    align-items: center;
    gap: clamp(0.4rem, 1vw, 0.75rem);
    flex-shrink: 0;
  }

  .app-icon svg {
    width: clamp(26px, 3.5vw, 36px);
    height: clamp(26px, 3.5vw, 36px);
    color: var(--color-primary);
  }

  .app-title {
    font-size: clamp(1.1rem, 1.8vw, 1.6rem);
    font-weight: 850;
    color: var(--color-text-dark);
    white-space: nowrap;
  }

  .header-time-section {
    font-size: clamp(0.9rem, 1.2vw, 1.2rem);
    font-weight: 700;
    text-align: center;
    white-space: nowrap;
  }

  @media (max-width: 1024px) {
    .header-time-section {
      display: none;
    }
  }

  .header-controls-section {
    display: flex;
    align-items: center;
    gap: clamp(0.5rem, 1.2vw, 1.5rem);
    flex-shrink: 0;
  }

  .lang-selector {
    display: flex;
    border: 2px solid var(--color-border);
    border-radius: 8px;
    overflow: hidden;
  }

  .lang-btn {
    padding: 0.35rem 0.75rem;
    border: none;
    background-color: var(--color-bg-app);
    font-weight: bold;
    cursor: pointer;
  }

  .lang-btn.active {
    background-color: var(--color-primary);
    color: var(--color-text-light);
  }

  .role-badge-section {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex-shrink: 0;
  }

  .badge {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.15rem;
    padding: 0.2rem clamp(0.35rem, 0.8vw, 0.75rem);
    border-radius: 8px;
    font-weight: bold;
    font-size: clamp(0.8rem, 0.9vw, 0.95rem);
  }

  @media (max-width: 768px) {
    .badge span {
      display: none;
    }
  }

  .role-icon-img {
    width: clamp(24px, 3vw, 30px);
    height: clamp(24px, 3vw, 30px);
    object-fit: contain;
  }

  .badge-admin {
    background-color: transparent;
    color: var(--color-secondary);
    border: none;
  }

  .badge-cashier {
    background-color: transparent;
    color: var(--color-primary);
    border: none;
  }

  .lock-icon-img {
    width: clamp(32px, 3.8vw, 44px);
    height: clamp(32px, 3.8vw, 44px);
    object-fit: contain;
  }

  .btn-lock-pos, .btn-unlock-pos {
    background-color: transparent;
    border: none;
    padding: 0.25rem;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: transform 0.15s ease;
  }

  .btn-lock-pos:hover, .btn-unlock-pos:hover {
    transform: scale(1.1);
  }

  .notifications-wrapper {
    position: relative;
    display: flex;
    align-items: center;
  }

  .notifications-btn {
    background-color: transparent;
    border: none;
    padding: 0.25rem;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: transform 0.15s ease;
    position: relative;
  }

  .notifications-btn:hover {
    transform: scale(1.1);
  }

  .control-icon-img {
    width: 44px;
    height: 44px;
    object-fit: contain;
  }

  .notifications-badge {
    position: absolute;
    top: -2px;
    right: -2px;
    background-color: var(--color-danger);
    color: var(--color-text-light);
    font-size: 0.75rem;
    font-weight: bold;
    border-radius: 50%;
    min-width: 18px;
    height: 18px;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0 4px;
    box-sizing: border-box;
    box-shadow: 0 0 0 2px var(--color-bg-card);
  }

  .notifications-dropdown {
    position: absolute;
    top: 55px;
    width: 320px;
    background-color: var(--color-bg-card);
    border: var(--border-width) solid var(--color-border);
    border-radius: var(--border-radius);
    box-shadow: 0 10px 25px rgba(0, 0, 0, 0.2);
    z-index: 1000;
    padding: 1rem;
    display: flex;
    flex-direction: column;
  }

  :global([dir="ltr"]) .notifications-dropdown {
    right: 0;
    left: auto;
  }

  :global([dir="rtl"]) .notifications-dropdown {
    left: 0;
    right: auto;
  }

  .dropdown-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.5rem;
  }

  .dropdown-header h3 {
    margin: 0;
    font-size: 1.1rem;
    font-weight: 800;
    color: var(--color-text-dark);
  }

  .btn-clear-all {
    background: transparent;
    border: none;
    color: var(--color-primary);
    font-weight: bold;
    cursor: pointer;
    font-size: 0.85rem;
    padding: 0.2rem 0.5rem;
  }

  .btn-clear-all:hover {
    text-decoration: underline;
  }

  .dropdown-divider {
    border: none;
    border-top: 1px solid var(--color-border);
    margin: 0.5rem 0;
  }

  .dropdown-body {
    max-height: 250px;
    overflow-y: auto;
  }

  .empty-notifications {
    text-align: center;
    color: var(--color-text-dark);
    opacity: 0.6;
    padding: 1rem 0;
    font-style: italic;
    font-size: 0.95rem;
    margin: 0;
  }

  .notifications-list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .notification-item {
    display: flex;
    align-items: flex-start;
    gap: 0.5rem;
    padding: 0.5rem;
    border-radius: 6px;
    background-color: var(--color-bg-app);
    border-left: 4px solid transparent;
    font-size: 0.9rem;
    color: var(--color-text-dark);
  }

  :global([dir="rtl"]) .notification-item {
    border-left: none;
    border-right: 4px solid transparent;
  }

  .notification-item.danger {
    border-left-color: var(--color-danger);
  }

  :global([dir="rtl"]) .notification-item.danger {
    border-right-color: var(--color-danger);
  }

  .notification-item.warning {
    border-left-color: #f59f00;
  }

  :global([dir="rtl"]) .notification-item.warning {
    border-right-color: #f59f00;
  }

  .severity-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    margin-top: 5px;
    flex-shrink: 0;
  }

  .severity-dot.danger {
    background-color: var(--color-danger);
  }

  .severity-dot.warning {
    background-color: #f59f00;
  }

  .notification-msg {
    flex: 1;
    line-height: 1.3;
    font-weight: 550;
  }

  .btn-dismiss-notif {
    background: transparent;
    border: none;
    cursor: pointer;
    padding: 0.1rem;
    display: flex;
    align-items: center;
    justify-content: center;
    opacity: 0.6;
    transition: opacity 0.1s ease;
  }

  .btn-dismiss-notif:hover {
    opacity: 1;
  }

  .dismiss-icon-img {
    width: 16px;
    height: 16px;
    object-fit: contain;
  }
</style>
