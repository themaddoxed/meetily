'use client';

import { useState } from 'react';
import { Globe, Pin } from 'lucide-react';
import { Popover, PopoverTrigger, PopoverContent } from '@/components/ui/popover';
import { LanguagePickerPopover } from '@/components/LanguagePickerPopover';
import { useRecentLanguages } from '@/hooks/useRecentLanguages';
import { labelForCode } from '@/lib/summary-languages';
import { useI18n } from '@/contexts/I18nContext';

export function SummaryLanguageSettings() {
  const { t } = useI18n();
  const { recents, pinned, addRecent, removeRecent, setPinned } = useRecentLanguages();
  const [pickerOpen, setPickerOpen] = useState(false);

  const togglePin = (code: string) => {
    setPinned(pinned === code ? null : code);
  };

  return (
    <div className="bg-card rounded-lg border border-border p-6 shadow-sm relative">
      <div className="flex items-center gap-2 mb-2">
        <Globe size={18} className="text-muted-foreground" />
        <h3 className="text-lg font-semibold text-foreground">{t('summary.languageSettingsTitle')}</h3>
      </div>
      <p className="text-sm text-muted-foreground mb-4">
        {t('summary.settingsDescription')}
      </p>

      <div className="flex flex-wrap items-center gap-2">
        {recents.map((code) => {
          const isPinned = pinned === code;
          return (
            <span
              key={code}
              className={`inline-flex items-center rounded-full border text-sm overflow-hidden ${
                isPinned
                  ? 'bg-info/10 border-info/30 text-info'
                  : 'bg-muted border-border text-foreground'
              }`}
            >
              <button
                type="button"
                aria-label={isPinned ? t('summary.unpinDefault', { language: labelForCode(code) }) : t('summary.pinDefault', { language: labelForCode(code) })}
                aria-pressed={isPinned}
                title={isPinned ? t('summary.unsetDefault') : t('summary.setDefault')}
                onClick={() => togglePin(code)}
                className={`flex items-center gap-1.5 pl-3 pr-2 py-1 hover:brightness-95 active:brightness-90 ${
                  isPinned ? 'text-info' : 'text-foreground'
                }`}
              >
                <Pin
                  size={14}
                  className={isPinned ? 'text-info' : 'text-muted-foreground'}
                  fill={isPinned ? 'currentColor' : 'none'}
                />
                {labelForCode(code)}
              </button>
              <button
                type="button"
                aria-label={t('summary.removeLanguage', { language: labelForCode(code) })}
                onClick={() => removeRecent(code)}
                className={`pr-2.5 pl-0.5 py-1 leading-none ${isPinned ? 'text-info hover:text-info' : 'text-muted-foreground hover:text-foreground'}`}
              >
                ×
              </button>
            </span>
          );
        })}

        <Popover open={pickerOpen} onOpenChange={setPickerOpen}>
          <PopoverTrigger asChild>
            <button
              type="button"
              disabled={recents.length >= 5}
              className="inline-flex items-center gap-1 rounded-full border border-dashed border-border px-3 py-1 text-sm text-muted-foreground hover:border-border hover:text-foreground disabled:cursor-not-allowed disabled:opacity-50"
            >
              ＋ {t('summary.addLanguage')}
            </button>
          </PopoverTrigger>
          <PopoverContent align="start" className="w-auto p-0 border-0 shadow-none bg-transparent">
            <LanguagePickerPopover
              mode="settings"
              value={null}
              onChange={(code) => {
                if (code) addRecent(code);
                setPickerOpen(false);
              }}
              onClose={() => setPickerOpen(false)}
            />
          </PopoverContent>
        </Popover>
      </div>

      <p className="text-xs text-muted-foreground mt-3">
        {pinned
          ? t('summary.defaultLanguageHint', { language: labelForCode(pinned) })
          : t('summary.chooseDefaultHint')}
      </p>
    </div>
  );
}
