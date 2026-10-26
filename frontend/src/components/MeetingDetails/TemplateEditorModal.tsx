'use client';

import { useEffect, useState } from 'react';
import { Copy, Loader2, Pencil, Plus, Trash2 } from 'lucide-react';
import { toast } from 'sonner';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Textarea } from '@/components/ui/textarea';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import type {
  TemplateDraft,
  TemplateFormat,
  TemplateInfo,
  TemplateManager,
  TemplateSection,
} from '@/hooks/meeting-details/useTemplates';
import { useI18n } from '@/contexts/I18nContext';

interface TemplateEditorModalProps {
  open: boolean;
  onClose: () => void;
  manager: TemplateManager;
}

const emptySection = (): TemplateSection => ({
  title: '',
  instruction: '',
  format: 'list',
});

const emptyDraft = (): TemplateDraft => ({
  name: '',
  description: '',
  sections: [emptySection()],
});

function errorMessage(error: unknown, fallback: string): string {
  if (typeof error === 'string') return error;
  if (error instanceof Error) return error.message;
  return fallback;
}

export function TemplateEditorModal({ open, onClose, manager }: TemplateEditorModalProps) {
  const { t } = useI18n();
  const displayTemplateName = (template: Pick<TemplateInfo, 'id' | 'name'>) => {
    if (template.id === 'standard_meeting') return t('template.standardMeeting');
    if (template.id === 'tasks_only') return t('template.tasksOnly');
    return template.name;
  };
  const [draft, setDraft] = useState<TemplateDraft>(emptyDraft);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [busyId, setBusyId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (open) {
      setDraft(emptyDraft());
      setEditingId(null);
      setError(null);
    }
  }, [open]);

  const updateSection = (index: number, changes: Partial<TemplateSection>) => {
    setDraft((current) => ({
      ...current,
      sections: current.sections.map((section, sectionIndex) => (
        sectionIndex === index ? { ...section, ...changes } : section
      )),
    }));
    setError(null);
  };

  const loadTemplate = async (template: TemplateInfo) => {
    setBusyId(template.id);
    setError(null);
    try {
      const details = await manager.getTemplateDetails(template.id);
      setDraft({
        name: details.name,
        description: details.description,
        sections: details.sections.length > 0 ? details.sections : [emptySection()],
      });
      setEditingId(details.is_custom ? details.id : null);
    } catch (loadError) {
      setError(errorMessage(loadError, t('template.saveFailed')));
    } finally {
      setBusyId(null);
    }
  };

  const copyTemplate = async (template: TemplateInfo) => {
    setBusyId(template.id);
    setError(null);
    try {
      const copiedId = await manager.copyTemplate(template.id);
      const copied = await manager.getTemplateDetails(copiedId);
      setDraft({
        name: copied.name,
        description: copied.description,
        sections: copied.sections.length > 0 ? copied.sections : [emptySection()],
      });
      setEditingId(copied.id);
      toast.success(t('template.copied', { name: displayTemplateName(template) }));
    } catch (copyError) {
      setError(errorMessage(copyError, t('template.saveFailed')));
    } finally {
      setBusyId(null);
    }
  };

  const deleteTemplate = async (template: TemplateInfo) => {
    if (!template.is_custom) return;
    setBusyId(template.id);
    setError(null);
    try {
      await manager.deleteCustomTemplate(template.id);
      if (editingId === template.id) {
        setDraft(emptyDraft());
        setEditingId(null);
      }
      toast.success(t('template.deleted', { name: template.name }));
    } catch (deleteError) {
      setError(errorMessage(deleteError, t('template.saveFailed')));
    } finally {
      setBusyId(null);
    }
  };

  const saveTemplate = async () => {
    if (!draft.name.trim()) {
      setError(t('template.addName'));
      return;
    }
    if (!draft.description.trim()) {
      setError(t('template.addDescription'));
      return;
    }
    if (draft.sections.length === 0) {
      setError(t('template.addSectionError'));
      return;
    }
    if (draft.sections.some((section) => !section.title.trim() || !section.instruction.trim())) {
      setError(t('template.sectionRequired'));
      return;
    }

    setSaving(true);
    setError(null);
    try {
      await manager.validateTemplate(draft);
      const savedId = editingId
        ? await manager.updateCustomTemplate(editingId, draft)
        : await manager.createCustomTemplate(draft);
      await manager.handleTemplateSelection(savedId, draft.name.trim());
      toast.success(t('template.saved', { name: draft.name.trim() }));
      onClose();
    } catch (saveError) {
      setError(errorMessage(saveError, t('template.saveFailed')));
    } finally {
      setSaving(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={(nextOpen) => !nextOpen && onClose()}>
      <DialogContent className="max-h-[90vh] max-w-3xl overflow-y-auto">
        <DialogHeader>
          <DialogTitle>{editingId ? t('template.edit') : t('template.create')}</DialogTitle>
          <DialogDescription>
            {t('template.description')}
          </DialogDescription>
        </DialogHeader>

        <div className="grid gap-5 lg:grid-cols-[minmax(0,1fr)_18rem]">
          <div className="space-y-4">
            <div className="grid gap-3 sm:grid-cols-2">
              <label className="space-y-1.5 text-sm font-medium">
                {t('template.name')}
                <Input
                  value={draft.name}
                  onChange={(event) => setDraft((current) => ({ ...current, name: event.target.value }))}
                  placeholder={t('template.namePlaceholder')}
                  disabled={saving}
                />
              </label>
              <label className="space-y-1.5 text-sm font-medium">
                {t('template.descriptionLabel')}
                <Input
                  value={draft.description}
                  onChange={(event) => setDraft((current) => ({ ...current, description: event.target.value }))}
                  placeholder={t('template.descriptionPlaceholder')}
                  disabled={saving}
                />
              </label>
            </div>

            <div className="space-y-3">
              <div className="flex items-center justify-between gap-3">
                <div>
                  <h3 className="text-sm font-semibold">{t('template.sections')}</h3>
                  <p className="text-xs text-muted-foreground">{t('template.sectionsHint')}</p>
                </div>
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  onClick={() => setDraft((current) => ({
                    ...current,
                    sections: [...current.sections, emptySection()],
                  }))}
                  disabled={saving}
                >
                  <Plus />
                  {t('template.addSection')}
                </Button>
              </div>

              {draft.sections.map((section, index) => (
                <div key={index} className="space-y-3 rounded-xl border border-border p-3">
                  <div className="flex items-start gap-2">
                    <Input
                      value={section.title}
                      onChange={(event) => updateSection(index, { title: event.target.value })}
                      placeholder={t('template.sectionTitle')}
                      disabled={saving}
                      aria-label={`${t('template.sectionTitle')} ${index + 1}`}
                    />
                    <Select
                      value={section.format}
                      onValueChange={(value) => updateSection(index, { format: value as TemplateFormat })}
                      disabled={saving}
                    >
                      <SelectTrigger className="w-32" aria-label={t('template.sectionFormat', { index: index + 1 })}>
                        <SelectValue />
                      </SelectTrigger>
                      <SelectContent>
                        <SelectItem value="paragraph">{t('template.paragraph')}</SelectItem>
                        <SelectItem value="list">{t('template.list')}</SelectItem>
                        <SelectItem value="table">{t('template.table')}</SelectItem>
                        <SelectItem value="string">{t('template.singleLine')}</SelectItem>
                      </SelectContent>
                    </Select>
                    <Button
                      type="button"
                      variant="ghost"
                      size="icon"
                      onClick={() => setDraft((current) => ({
                        ...current,
                        sections: current.sections.filter((_, sectionIndex) => sectionIndex !== index),
                      }))}
                      disabled={saving || draft.sections.length === 1}
                      aria-label={t('template.removeSection', { index: index + 1 })}
                    >
                      <Trash2 />
                    </Button>
                  </div>
                  <Textarea
                    value={section.instruction}
                    onChange={(event) => updateSection(index, { instruction: event.target.value })}
                    placeholder={t('template.instructionPlaceholder')}
                    rows={3}
                    disabled={saving}
                    aria-label={t('template.sectionInstruction', { index: index + 1 })}
                  />
                  {(section.format === 'list' || section.format === 'table') && (
                    <Input
                      value={section.item_format || section.example_item_format || ''}
                      onChange={(event) => updateSection(index, {
                        item_format: event.target.value,
                        example_item_format: undefined,
                      })}
                      placeholder={t('template.itemFormatPlaceholder')}
                      disabled={saving}
                      aria-label={t('template.sectionFormatHint', { index: index + 1 })}
                    />
                  )}
                </div>
              ))}
            </div>

            {error && (
              <p role="alert" className="rounded-lg border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive">
                {error}
              </p>
            )}
          </div>

          <aside className="space-y-3 rounded-xl border border-border bg-muted/20 p-3">
            <div>
              <h3 className="text-sm font-semibold">{t('template.available')}</h3>
              <p className="text-xs text-muted-foreground">{t('template.copyHint')}</p>
            </div>
            <div className="space-y-2">
              {manager.availableTemplates.map((template) => {
                const isBusy = busyId === template.id;
                return (
                  <div key={template.id} className="rounded-lg border border-border bg-background p-2">
                    <div className="min-w-0">
                      <p className="truncate text-sm font-medium">{displayTemplateName(template)}</p>
                      <p className="line-clamp-2 text-xs text-muted-foreground">{template.description}</p>
                    </div>
                    <div className="mt-2 flex flex-wrap gap-1">
                      {template.is_custom ? (
                        <>
                          <Button
                            type="button"
                            variant="ghost"
                            size="sm"
                            onClick={() => void loadTemplate(template)}
                            disabled={isBusy || saving}
                          >
                            {isBusy ? <Loader2 className="animate-spin" /> : <Pencil />}
                            {t('template.editAction')}
                          </Button>
                          <Button
                            type="button"
                            variant="ghost"
                            size="sm"
                            onClick={() => void copyTemplate(template)}
                            disabled={isBusy || saving}
                          >
                            {isBusy ? <Loader2 className="animate-spin" /> : <Copy />}
                            {t('template.copy')}
                          </Button>
                        </>
                      ) : (
                        <Button
                          type="button"
                          variant="ghost"
                          size="sm"
                          onClick={() => void copyTemplate(template)}
                          disabled={isBusy || saving}
                        >
                          {isBusy ? <Loader2 className="animate-spin" /> : <Copy />}
                          {t('template.copy')}
                        </Button>
                      )}
                      {template.is_custom && (
                        <Button
                          type="button"
                          variant="ghost"
                          size="sm"
                          className="text-destructive hover:text-destructive"
                          onClick={() => void deleteTemplate(template)}
                          disabled={isBusy || saving}
                        >
                          {isBusy ? <Loader2 className="animate-spin" /> : <Trash2 />}
                          {t('template.delete')}
                        </Button>
                      )}
                    </div>
                  </div>
                );
              })}
            </div>
          </aside>
        </div>

        <DialogFooter>
          <Button type="button" variant="ghost" onClick={onClose} disabled={saving}>{t('common.cancel')}</Button>
          <Button type="button" onClick={() => void saveTemplate()} disabled={saving}>
            {saving && <Loader2 className="animate-spin" />}
            {editingId ? t('template.saveChanges') : t('template.createAction')}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

export default TemplateEditorModal;
