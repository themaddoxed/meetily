import { useCallback, useEffect, useRef, useState } from 'react';
import { invoke as invokeTauri } from '@tauri-apps/api/core';
import { toast } from 'sonner';
import { useI18n } from '@/contexts/I18nContext';

export type TemplateFormat = 'paragraph' | 'list' | 'table' | 'string';

export interface TemplateSection {
  title: string;
  instruction: string;
  format: TemplateFormat;
  item_format?: string;
  example_item_format?: string;
}

export interface TemplateInfo {
  id: string;
  name: string;
  description: string;
  is_custom: boolean;
}

export interface TemplateDetails extends TemplateInfo {
  sections: TemplateSection[];
}

export interface TemplateDraft {
  name: string;
  description: string;
  sections: TemplateSection[];
}

export interface TemplateSelectionResponse {
  templateId: string;
  replaced: boolean;
  previousTemplateId: string | null;
}

export interface TemplateManager {
  availableTemplates: TemplateInfo[];
  selectedTemplate: string;
  isSelectionReady: boolean;
  selectionError: string | null;
  handleTemplateSelection: (templateId: string, templateName: string) => Promise<void>;
  refreshTemplates: () => Promise<TemplateInfo[]>;
  getTemplateDetails: (templateId: string) => Promise<TemplateDetails>;
  validateTemplate: (draft: TemplateDraft) => Promise<string>;
  createCustomTemplate: (draft: TemplateDraft, requestedId?: string) => Promise<string>;
  copyTemplate: (templateId: string, requestedId?: string) => Promise<string>;
  updateCustomTemplate: (templateId: string, draft: TemplateDraft) => Promise<string>;
  deleteCustomTemplate: (templateId: string) => Promise<TemplateInfo[]>;
  isCustomTemplate: (templateId: string) => Promise<boolean>;
}

const SELECTED_TEMPLATE_STORAGE_KEY = 'meetily.selected-summary-template';
const DEFAULT_TEMPLATE_ID = 'standard_meeting';

function writeStoredTemplateId(templateId: string): void {
  if (typeof window === 'undefined') return;
  try {
    window.localStorage.setItem(SELECTED_TEMPLATE_STORAGE_KEY, templateId);
  } catch {
    // Local storage is a convenience. The native template file remains the source of truth.
  }
}

export function serializeTemplateDraft(draft: TemplateDraft): string {
  const sections = draft.sections.map((section) => {
    const serialized: Omit<TemplateSection, 'item_format' | 'example_item_format'> & {
      item_format?: string | null;
      example_item_format?: string | null;
    } = {
      title: section.title.trim(),
      instruction: section.instruction.trim(),
      format: section.format,
    };

    if (Object.prototype.hasOwnProperty.call(section, 'item_format')) {
      serialized.item_format = section.item_format?.trim() || null;
    }
    if (Object.prototype.hasOwnProperty.call(section, 'example_item_format')) {
      serialized.example_item_format = section.example_item_format?.trim() || null;
    }
    return serialized;
  });

  return JSON.stringify(
    {
      name: draft.name.trim(),
      description: draft.description.trim(),
      sections,
    },
    null,
    2,
  );
}

function fallbackTemplateId(templates: TemplateInfo[], currentId: string): string {
  if (templates.some((template) => template.id === currentId)) return currentId;
  return templates.find((template) => template.id === DEFAULT_TEMPLATE_ID)?.id
    || templates[0]?.id
    || DEFAULT_TEMPLATE_ID;
}

export function useTemplates(): TemplateManager {
  const { t } = useI18n();
  const tRef = useRef(t);
  tRef.current = t;
  const [availableTemplates, setAvailableTemplates] = useState<TemplateInfo[]>([]);
  const [selectedTemplate, setSelectedTemplate] = useState<string>(DEFAULT_TEMPLATE_ID);
  const [isSelectionReady, setIsSelectionReady] = useState(false);
  const [selectionError, setSelectionError] = useState<string | null>(null);
  const selectedTemplateRef = useRef(selectedTemplate);
  const selectionRequestRef = useRef(0);

  useEffect(() => {
    selectedTemplateRef.current = selectedTemplate;
  }, [selectedTemplate]);

  const applyTemplateSelection = useCallback((templateId: string) => {
    selectedTemplateRef.current = templateId;
    setSelectedTemplate(templateId);
    writeStoredTemplateId(templateId);
  }, []);

  const refreshTemplates = useCallback(async (): Promise<TemplateInfo[]> => {
    const requestId = ++selectionRequestRef.current;
    setIsSelectionReady(false);
    setSelectionError(null);

    try {
      const templates = await invokeTauri<TemplateInfo[]>('api_list_templates');
      if (requestId !== selectionRequestRef.current) return templates;

      setAvailableTemplates(templates);
      const nextSelected = fallbackTemplateId(templates, selectedTemplateRef.current);
      if (nextSelected !== selectedTemplateRef.current && templates.length > 0) {
        const persisted = await invokeTauri<TemplateSelectionResponse>(
          'api_save_summary_template_selection',
          { templateId: nextSelected },
        );
        if (requestId !== selectionRequestRef.current) return templates;
        applyTemplateSelection(persisted.templateId);
        toast.info(tRef.current('template.replaced'), {
          description: tRef.current('template.replacedDescription'),
        });
      } else if (templates.length > 0) {
        applyTemplateSelection(nextSelected);
      }

      setIsSelectionReady(true);
      return templates;
    } catch (error) {
      console.error('Failed to fetch templates:', error);
      if (requestId === selectionRequestRef.current) {
        setSelectionError(
          error instanceof Error ? error.message : tRef.current('template.loadFailed'),
        );
      }
      return [];
    }
  }, [applyTemplateSelection]);

  useEffect(() => {
    const requestId = ++selectionRequestRef.current;
    setIsSelectionReady(false);
    setSelectionError(null);

    const loadSelection = async () => {
      try {
        const selection = await invokeTauri<TemplateSelectionResponse>(
          'api_get_summary_template_selection',
        );
        if (requestId !== selectionRequestRef.current) return;
        if (selection.replaced) {
          toast.info(tRef.current('template.replaced'), {
            description: tRef.current('template.replacedDescription'),
          });
        }
        selectedTemplateRef.current = selection.templateId;
        setSelectedTemplate(selection.templateId);
        await refreshTemplates();
      } catch (error) {
        console.error('Failed to load selected summary template:', error);
        if (requestId === selectionRequestRef.current) {
          setSelectionError(
            error instanceof Error ? error.message : tRef.current('template.loadFailed'),
          );
          setIsSelectionReady(false);
        }
      }
    };

    void loadSelection();

    return () => {
      selectionRequestRef.current += 1;
    };
  }, [refreshTemplates]);

  const getTemplateDetails = useCallback(async (templateId: string): Promise<TemplateDetails> => {
    return invokeTauri<TemplateDetails>('api_get_template_details', { templateId });
  }, []);

  const validateTemplate = useCallback(async (draft: TemplateDraft): Promise<string> => {
    return invokeTauri<string>('api_validate_template', {
      templateJson: serializeTemplateDraft(draft),
    });
  }, []);

  const createCustomTemplate = useCallback(async (
    draft: TemplateDraft,
    requestedId?: string,
  ): Promise<string> => {
    const savedId = await invokeTauri<string>('api_create_custom_template', {
      templateId: requestedId || null,
      templateJson: serializeTemplateDraft(draft),
    });
    await refreshTemplates();
    return savedId;
  }, [refreshTemplates]);

  const copyTemplate = useCallback(async (
    templateId: string,
    requestedId?: string,
  ): Promise<string> => {
    const copiedId = await invokeTauri<string>('api_copy_template', {
      sourceTemplateId: templateId,
      requestedTemplateId: requestedId || null,
    });
    await refreshTemplates();
    return copiedId;
  }, [refreshTemplates]);

  const updateCustomTemplate = useCallback(async (
    templateId: string,
    draft: TemplateDraft,
  ): Promise<string> => {
    const savedId = await invokeTauri<string>('api_update_custom_template', {
      templateId,
      templateJson: serializeTemplateDraft(draft),
    });
    await refreshTemplates();
    return savedId;
  }, [refreshTemplates]);

  const deleteCustomTemplate = useCallback(async (templateId: string): Promise<TemplateInfo[]> => {
    const wasSelected = selectedTemplateRef.current === templateId;
    await invokeTauri('api_delete_custom_template', { templateId });
    const templates = await refreshTemplates();
    if (wasSelected && templates.length > 0) {
      const nextSelected = fallbackTemplateId(templates, '');
      applyTemplateSelection(nextSelected);
      toast.info(t('template.deletedToast'), {
        description: t('template.deletedDescription', {
          name: templates.find((template) => template.id === nextSelected)?.name || t('nav.meetings'),
        }),
      });
    }
    return templates;
  }, [applyTemplateSelection, refreshTemplates, t]);

  const isCustomTemplate = useCallback(async (templateId: string): Promise<boolean> => {
    try {
      return await invokeTauri<boolean>('api_is_custom_template', { templateId });
    } catch {
      return false;
    }
  }, []);

  const handleTemplateSelection = useCallback(async (templateId: string, templateName: string) => {
    try {
      const persisted = await invokeTauri<TemplateSelectionResponse>(
        'api_save_summary_template_selection',
        { templateId },
      );
      applyTemplateSelection(persisted.templateId);
      toast.success(t('template.selected'), {
        description: t('template.selectedDescription', { name: templateName }),
      });
    } catch (error) {
      console.error('Failed to save selected summary template:', error);
      toast.error(t('template.selectionFailed'), {
        description: error instanceof Error ? error.message : String(error),
      });
    }
  }, [applyTemplateSelection, t]);

  return {
    availableTemplates,
    selectedTemplate,
    isSelectionReady,
    selectionError,
    handleTemplateSelection,
    refreshTemplates,
    getTemplateDetails,
    validateTemplate,
    createCustomTemplate,
    copyTemplate,
    updateCustomTemplate,
    deleteCustomTemplate,
    isCustomTemplate,
  };
}
