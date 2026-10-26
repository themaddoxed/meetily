"use client"

import { useEffect, useState } from "react"
import { Switch } from "./ui/switch"
import { FolderOpen, Monitor, Moon, Sun } from "lucide-react"
import { invoke } from "@tauri-apps/api/core"
import { useConfig, NotificationSettings } from "@/contexts/ConfigContext"
import { useI18n } from "@/contexts/I18nContext"

const THEME_OPTIONS = [
  { value: 'system', labelKey: 'settings.themeSystem', icon: Monitor },
  { value: 'light', labelKey: 'settings.themeLight', icon: Sun },
  { value: 'dark', labelKey: 'settings.themeDark', icon: Moon },
] as const;

export function PreferenceSettings() {
  const { locale, setLocale, t } = useI18n();
  const {
    notificationSettings,
    storageLocations,
    isLoadingPreferences,
    loadPreferences,
    updateNotificationSettings,
    themeMode,
    setThemeMode
  } = useConfig();

  const [notificationsEnabled, setNotificationsEnabled] = useState<boolean | null>(null);
  const [isInitialLoad, setIsInitialLoad] = useState(true);
  const [previousNotificationsEnabled, setPreviousNotificationsEnabled] = useState<boolean | null>(null);

  // Lazy load preferences on mount (only loads if not already cached)
  useEffect(() => {
    loadPreferences();
  }, [loadPreferences]);

  // Update notificationsEnabled when notificationSettings are loaded from global state
  useEffect(() => {
    if (notificationSettings) {
      // Notification enabled means both started and stopped notifications are enabled
      const enabled =
        notificationSettings.notification_preferences.show_recording_started &&
        notificationSettings.notification_preferences.show_recording_stopped;
      setNotificationsEnabled(enabled);
      if (isInitialLoad) {
        setPreviousNotificationsEnabled(enabled);
        setIsInitialLoad(false);
      }
    } else if (!isLoadingPreferences) {
      // If not loading and no settings, use default
      setNotificationsEnabled(true);
      if (isInitialLoad) {
        setPreviousNotificationsEnabled(true);
        setIsInitialLoad(false);
      }
    }
  }, [notificationSettings, isLoadingPreferences, isInitialLoad])

  useEffect(() => {
    // Skip update on initial load or if value hasn't actually changed
    if (isInitialLoad || notificationsEnabled === null || notificationsEnabled === previousNotificationsEnabled) return;
    if (!notificationSettings) return;

    const handleUpdateNotificationSettings = async () => {
      console.log("Updating notification settings to:", notificationsEnabled);

      try {
        // Update the notification preferences
        const updatedSettings: NotificationSettings = {
          ...notificationSettings,
          notification_preferences: {
            ...notificationSettings.notification_preferences,
            show_recording_started: notificationsEnabled,
            show_recording_stopped: notificationsEnabled,
          }
        };

        console.log("Calling updateNotificationSettings with:", updatedSettings);
        await updateNotificationSettings(updatedSettings);
        setPreviousNotificationsEnabled(notificationsEnabled);
        console.log("Successfully updated notification settings to:", notificationsEnabled);

      } catch (error) {
        console.error('Failed to update notification settings:', error);
      }
    };

    handleUpdateNotificationSettings();
  }, [notificationsEnabled, notificationSettings, isInitialLoad, previousNotificationsEnabled, updateNotificationSettings])

  const handleOpenFolder = async (folderType: 'database' | 'models' | 'recordings') => {
    try {
      switch (folderType) {
        case 'database':
          await invoke('open_database_folder');
          break;
        case 'models':
          await invoke('open_models_folder');
          break;
        case 'recordings':
          await invoke('open_recordings_folder');
          break;
      }
    } catch (error) {
      console.error(`Failed to open ${folderType} folder:`, error);
    }
  };

  // Show loading only if we're actually loading and don't have cached data
  if (isLoadingPreferences && !notificationSettings && !storageLocations) {
    return <div className="max-w-2xl mx-auto p-6">{t('common.loading')} {t('settings.preferences').toLowerCase()}...</div>
  }

  // Show loading if notificationsEnabled hasn't been determined yet
  if (notificationsEnabled === null && !isLoadingPreferences) {
    return <div className="max-w-2xl mx-auto p-6">{t('common.loading')} {t('settings.preferences').toLowerCase()}...</div>
  }

  // Ensure we have a boolean value for the Switch component
  const notificationsEnabledValue = notificationsEnabled ?? false;

  return (
    <div className="space-y-6">
      {/* Appearance Section */}
      <div className="surface-card p-6">
        <div className="flex items-center justify-between gap-6">
          <div>
            <h3 className="mb-2 text-balance text-lg font-semibold text-foreground">{t('settings.appearance')}</h3>
            <p className="text-pretty text-sm text-muted-foreground">{t('settings.appearanceDescription')}</p>
          </div>
          <div className="inline-flex rounded-xl border border-border bg-muted p-1">
            {THEME_OPTIONS.map(({ value, labelKey, icon: Icon }) => (
              <button
                key={value}
                type="button"
                aria-pressed={themeMode === value}
                onClick={() => setThemeMode(value)}
                className={`flex min-h-10 items-center gap-2 rounded-lg px-3 py-2 text-sm font-medium transition-[color,background-color,box-shadow,transform] duration-150 motion-safe:active:scale-[0.96] motion-reduce:transform-none motion-reduce:transition-none ${themeMode === value
                  ? 'bg-background text-foreground shadow-sm'
                  : 'text-muted-foreground hover:text-foreground'
                  }`}
              >
                <Icon className="h-4 w-4" />
                <span>{t(labelKey)}</span>
              </button>
            ))}
          </div>
        </div>
      </div>

      {/* Interface language section */}
      <div className="surface-card p-6">
        <div className="flex items-center justify-between gap-6">
          <div>
            <h3 className="mb-2 text-balance text-lg font-semibold text-foreground">{t('settings.interfaceLanguage')}</h3>
            <p className="text-pretty text-sm text-muted-foreground">{t('settings.interfaceLanguageDescription')}</p>
          </div>
          <label className="sr-only" htmlFor="interface-language-selector">{t('settings.interfaceLanguage')}</label>
          <select
            id="interface-language-selector"
            data-testid="interface-language-selector"
            value={locale}
            onChange={(event) => setLocale(event.target.value === 'ru' ? 'ru' : 'en')}
            className="min-h-10 rounded-lg border border-border bg-background px-3 py-2 text-sm text-foreground focus:outline-none focus:ring-2 focus:ring-info"
          >
            <option value="en">{t('settings.english')}</option>
            <option value="ru">{t('settings.russian')}</option>
          </select>
        </div>
      </div>

      {/* Notifications Section */}
      <div className="surface-card p-6">
        <div className="flex items-center justify-between">
          <div>
            <h3 className="mb-2 text-balance text-lg font-semibold text-foreground">{t('settings.notifications')}</h3>
            <p className="text-pretty text-sm text-muted-foreground">{t('settings.notificationsDescription')}</p>
          </div>
          <Switch
            aria-label={t('settings.notifications')}
            checked={notificationsEnabledValue}
            onCheckedChange={setNotificationsEnabled}
          />
        </div>
      </div>

      {/* Data Storage Locations Section */}
      <div className="surface-card p-6">
        <h3 className="mb-4 text-balance text-lg font-semibold text-foreground">{t('settings.dataStorage')}</h3>
        <p className="mb-6 text-pretty text-sm text-muted-foreground">
          {t('settings.dataStorageDescription')}
        </p>

        <div className="space-y-4">
          {/* Database Location */}
          {/* <div className="p-4 border border-border rounded-lg bg-muted/40">
            <div className="font-medium mb-2">Database</div>
            <div className="text-sm text-muted-foreground mb-3 break-all font-mono text-xs">
              {storageLocations?.database || 'Loading...'}
            </div>
            <button
              onClick={() => handleOpenFolder('database')}
              className="flex items-center gap-2 px-3 py-2 text-sm border border-border rounded-md hover:bg-accent transition-colors"
            >
              <FolderOpen className="w-4 h-4" />
              Open Folder
            </button>
          </div> */}

          {/* Models Location */}
          {/* <div className="p-4 border border-border rounded-lg bg-muted/40">
            <div className="font-medium mb-2">Whisper Models</div>
            <div className="text-sm text-muted-foreground mb-3 break-all font-mono text-xs">
              {storageLocations?.models || 'Loading...'}
            </div>
            <button
              onClick={() => handleOpenFolder('models')}
              className="flex items-center gap-2 px-3 py-2 text-sm border border-border rounded-md hover:bg-accent transition-colors"
            >
              <FolderOpen className="w-4 h-4" />
              Open Folder
            </button>
          </div> */}

          {/* Recordings Location */}
          <div className="p-4 border border-border rounded-lg bg-muted/40">
            <div className="font-medium mb-2">{t('settings.meetingRecordings')}</div>
            <div className="text-sm text-muted-foreground mb-3 break-all font-mono text-xs">
              {storageLocations?.recordings || 'Loading...'}
            </div>
            <button
              onClick={() => handleOpenFolder('recordings')}
              className="flex min-h-10 items-center gap-2 rounded-lg border border-border px-3 py-2 text-sm transition-[color,background-color,transform] motion-safe:active:scale-[0.96] motion-reduce:transform-none motion-reduce:transition-none hover:bg-accent"
            >
              <FolderOpen className="w-4 h-4" />
              {t('settings.openFolder')}
            </button>
          </div>
        </div>

        <div className="mt-4 p-3 bg-muted/40 border border-border rounded-md">
          <p className="text-pretty text-xs text-muted-foreground">
            <strong>{t('settings.note')}</strong> {t('settings.storageNote')}
          </p>
        </div>
      </div>
    </div>
  )
}
