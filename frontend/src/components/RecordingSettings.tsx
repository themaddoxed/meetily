import React, { useState, useEffect, useRef } from 'react';
import { Switch } from '@/components/ui/switch';
import { Button } from '@/components/ui/button';
import { FolderOpen } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { DeviceSelection, SelectedDevices } from '@/components/DeviceSelection';
import { toast } from 'sonner';
import { useConfig } from '@/contexts/ConfigContext';
import { useI18n } from '@/contexts/I18nContext';

export interface RecordingPreferences {
  save_folder: string;
  auto_save: boolean;
  file_format: string;
  preferred_mic_device: string | null;
  preferred_system_device: string | null;
}

interface RecordingSettingsProps {
  onSave?: (preferences: RecordingPreferences) => void;
}

export function RecordingSettings({ onSave }: RecordingSettingsProps) {
  const { updateRecordingSaveFolder } = useConfig();
  const { t } = useI18n();
  const [preferences, setPreferences] = useState<RecordingPreferences>({
    save_folder: '',
    auto_save: true,
    file_format: 'mp4',
    preferred_mic_device: null,
    preferred_system_device: null
  });
  const preferencesRef = useRef(preferences);
  const chooserBusyRef = useRef(false);
  const savingRef = useRef(false);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [choosingFolder, setChoosingFolder] = useState(false);
  const [showRecordingNotification, setShowRecordingNotification] = useState(true);
  const controlsBusy = saving || choosingFolder;

  const updatePreferences = (next: RecordingPreferences) => {
    preferencesRef.current = next;
    setPreferences(next);
  };

  // Load recording preferences on component mount
  useEffect(() => {
    const loadPreferences = async () => {
      try {
        const prefs = await invoke<RecordingPreferences>('get_recording_preferences');
        updatePreferences(prefs);
      } catch (error) {
        console.error('Failed to load recording preferences:', error);
        // If loading fails, get default folder path
        try {
          const defaultPath = await invoke<string>('get_default_recordings_folder_path');
          updatePreferences({
            ...preferencesRef.current,
            save_folder: defaultPath,
          });
        } catch (defaultError) {
          console.error('Failed to get default folder path:', defaultError);
        }
      } finally {
        setLoading(false);
      }
    };

    loadPreferences();
  }, []);

  // Load recording notification preference
  useEffect(() => {
    const loadNotificationPref = async () => {
      try {
        const { Store } = await import('@tauri-apps/plugin-store');
        const store = await Store.load('preferences.json');
        const show = await store.get<boolean>('show_recording_notification') ?? true;
        setShowRecordingNotification(show);
      } catch (error) {
        console.error('Failed to load notification preference:', error);
      }
    };
    loadNotificationPref();
  }, []);

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;

    import('@tauri-apps/api/event')
      .then(({ listen }) =>
        listen<RecordingPreferences>('recording-preferences-updated', (event) => {
          updatePreferences(event.payload);
        }),
      )
      .then((cleanup) => {
        if (disposed) {
          cleanup();
        } else {
          unlisten = cleanup;
        }
      })
      .catch((error) => {
        console.error('Failed to listen for recording preference updates:', error);
      });

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  const handleAutoSaveToggle = async (enabled: boolean) => {
    const newPreferences = { ...preferencesRef.current, auto_save: enabled };
    updatePreferences(newPreferences);
    await savePreferences(newPreferences);

  };

  const handleDeviceChange = async (devices: SelectedDevices) => {
    const newPreferences = {
      ...preferencesRef.current,
      preferred_mic_device: devices.micDevice,
      preferred_system_device: devices.systemDevice
    };
    updatePreferences(newPreferences);
    await savePreferences(newPreferences);

  };

  const handleOpenFolder = async () => {
    try {
      await invoke('open_recordings_folder');
    } catch (error) {
      console.error('Failed to open recordings folder:', error);
      toast.error(t('settings.openFolderFailed'), {
        description: error instanceof Error ? error.message : String(error),
      });
    }
  };

  const handleChooseFolder = async () => {
    if (chooserBusyRef.current || savingRef.current) return;
    chooserBusyRef.current = true;
    setChoosingFolder(true);
    try {
      const selectedFolder = await invoke<string | null>('select_recording_folder');
      if (!selectedFolder) return;

      const nextPreferences = {
        ...preferencesRef.current,
        save_folder: selectedFolder,
      };
      const saved = await savePreferences(nextPreferences, t('settings.recordingFolderSaved'));
      if (!saved) return;

      updatePreferences(nextPreferences);
      updateRecordingSaveFolder(selectedFolder);
    } catch (error) {
      console.error('Failed to choose recordings folder:', error);
      toast.error(t('settings.chooseFolderFailed'), {
        description: error instanceof Error ? error.message : String(error),
      });
    } finally {
      chooserBusyRef.current = false;
      setChoosingFolder(false);
    }
  };

  const handleNotificationToggle = async (enabled: boolean) => {
    try {
      setShowRecordingNotification(enabled);
      const { Store } = await import('@tauri-apps/plugin-store');
      const store = await Store.load('preferences.json');
      await store.set('show_recording_notification', enabled);
      await store.save();
      toast.success(t('settings.preferenceSaved'));
    } catch (error) {
      console.error('Failed to save notification preference:', error);
      toast.error(t('settings.preferenceSaveFailed'));
    }
  };

  const savePreferences = async (
    prefs: RecordingPreferences,
    successMessage?: string,
  ) => {
    savingRef.current = true;
    setSaving(true);
    try {
      await invoke('set_recording_preferences', { preferences: prefs });
      onSave?.(prefs);

      if (successMessage) {
        toast.success(successMessage);
      } else {
        const micDevice = prefs.preferred_mic_device || t('settings.defaultDevice');
        const systemDevice = prefs.preferred_system_device || t('settings.defaultDevice');
        toast.success(t('settings.devicePreferencesSaved'), {
          description: t('settings.devicePreferencesDescription', {
            microphone: micDevice,
            systemAudio: systemDevice,
          }),
        });
      }
      return true;
    } catch (error) {
      console.error('Failed to save recording preferences:', error);
      toast.error(successMessage ? t('settings.recordingFolderSaveFailed') : t('settings.devicePreferencesSaveFailed'), {
        description: error instanceof Error ? error.message : String(error)
      });
      return false;
    } finally {
      savingRef.current = false;
      setSaving(false);
    }
  };

  if (loading) {
    return (
      <div className="animate-pulse">
        <div className="h-4 bg-muted rounded w-1/4 mb-4"></div>
        <div className="h-8 bg-muted rounded mb-4"></div>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div>
        <h3 className="text-lg font-semibold mb-4">{t('settings.recordings')}</h3>
        <p className="text-sm text-muted-foreground mb-6">
          {t('settings.recordingSettingsDescription')}
        </p>
      </div>

      {/* Auto Save Toggle */}
      <div className="flex items-center justify-between p-4 border rounded-lg">
        <div className="flex-1">
          <div className="font-medium">{t('settings.saveAudioRecordings')}</div>
          <div className="text-sm text-muted-foreground">
            {t('settings.saveAudioRecordingsDescription')}
          </div>
        </div>
        <Switch
          checked={preferences.auto_save}
          onCheckedChange={handleAutoSaveToggle}
          disabled={controlsBusy}
        />
      </div>

      {/* Folder Location - Only shown when auto_save is enabled */}
      {preferences.auto_save && (
        <div className="space-y-4">
          <div className="p-4 border rounded-lg bg-muted/40">
            <div className="font-medium mb-2">{t('settings.saveLocation')}</div>
            <div className="text-sm text-muted-foreground mb-3 break-all">
              {preferences.save_folder || t('settings.defaultFolder')}
            </div>
            <div className="flex flex-wrap gap-2">
              <Button
                type="button"
                variant="outline"
                size="sm"
                onClick={handleOpenFolder}
                disabled={controlsBusy}
              >
                <FolderOpen className="w-4 h-4" />
                {t('settings.openFolder')}
              </Button>
              <Button
                type="button"
                variant="outline"
                size="sm"
                onClick={handleChooseFolder}
                disabled={controlsBusy}
              >
                <FolderOpen className="w-4 h-4" />
                {t('settings.chooseFolder')}
              </Button>
            </div>
          </div>

          <div className="p-4 border rounded-lg bg-info/10">
            <div className="text-sm text-info">
              <strong>{t('settings.fileFormat')}</strong> {preferences.file_format.toUpperCase()} {t('settings.files')}
            </div>
            <div className="text-xs text-info mt-1">
              {t('settings.recordingFileNameHint', { format: preferences.file_format })}
            </div>
          </div>
        </div>
      )}

      {/* Info when auto_save is disabled */}
      {!preferences.auto_save && (
        <div className="p-4 border rounded-lg bg-warning/10">
          <div className="text-sm text-warning">
            {t('settings.audioRecordingDisabled')}
          </div>
        </div>
      )}

      {/* Recording Notification Toggle */}
      <div className="flex items-center justify-between p-4 border rounded-lg">
        <div className="flex-1">
          <div className="font-medium">{t('settings.recordingStartNotification')}</div>
          <div className="text-sm text-muted-foreground">
            {t('settings.recordingStartNotificationDescription')}
          </div>
        </div>
        <Switch
          checked={showRecordingNotification}
          onCheckedChange={handleNotificationToggle}
          disabled={controlsBusy}
        />
      </div>

      {/* Device Preferences */}
      <div className="space-y-4">
        <div className="border-t pt-6">
          <h4 className="text-base font-medium text-foreground mb-4">{t('settings.defaultAudioDevices')}</h4>
          <p className="text-sm text-muted-foreground mb-4">
            {t('settings.defaultAudioDevicesDescription')}
          </p>

          <div className="border rounded-lg p-4 bg-muted/40">
            <DeviceSelection
              selectedDevices={{
                micDevice: preferences.preferred_mic_device,
                systemDevice: preferences.preferred_system_device
              }}
              onDeviceChange={handleDeviceChange}
              disabled={controlsBusy}
            />
          </div>
        </div>
      </div>
    </div>
  );
}
