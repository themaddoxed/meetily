"use client";

import { Button } from '@/components/ui/button';
import { ButtonGroup } from '@/components/ui/button-group';
import { Copy, Save, Loader2 } from 'lucide-react';
import { useI18n } from '@/contexts/I18nContext';

interface SummaryUpdaterButtonGroupProps {
  isSaving: boolean;
  isDirty: boolean;
  onSave: () => Promise<void>;
  onCopy: () => Promise<void>;
  hasSummary: boolean;
}

export function SummaryUpdaterButtonGroup({
  isSaving,
  isDirty,
  onSave,
  onCopy,
  hasSummary
}: SummaryUpdaterButtonGroupProps) {
  const { t } = useI18n();
  return (
    <ButtonGroup className="max-w-full flex-wrap">
      {/* Save button */}
      <Button
        variant="outline"
        size="sm"
        className={`${isDirty ? 'bg-success/20' : ""}`}
        title={isSaving ? t('summary.saving') : t('summary.saveChanges')}
        onClick={() => {
          onSave();
        }}
        disabled={isSaving}
      >
        {isSaving ? (
          <>
            <Loader2 className="animate-spin" />
            <span className="hidden lg:inline">{t('summary.savingEllipsis')}</span>
          </>
        ) : (
          <>
            <Save />
            <span className="hidden lg:inline">{t('summary.save')}</span>
          </>
        )}
      </Button>

      {/* Copy button */}
      <Button
        variant="outline"
        size="sm"
        title={t('summary.copySummary')}
        onClick={() => {
          onCopy();
        }}
        disabled={!hasSummary}
        className="cursor-pointer"
      >
        <Copy />
        <span className="hidden lg:inline">{t('common.copy')}</span>
      </Button>
    </ButtonGroup>
  );
}
