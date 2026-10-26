import { describe, expect, test } from 'vitest';

import { serializeTemplateDraft, type TemplateDraft } from '@/hooks/meeting-details/useTemplates';

const section = {
  title: 'Action items',
  instruction: 'List assigned work.',
  format: 'list' as const,
};

describe('serializeTemplateDraft', () => {
  test('sends explicit nulls when a populated format hint is cleared', () => {
    const draft: TemplateDraft = {
      name: 'Tasks',
      description: 'Assigned work',
      sections: [{
        ...section,
        item_format: '',
        example_item_format: undefined,
      }],
    };

    const serialized = JSON.parse(serializeTemplateDraft(draft)) as {
      sections: Array<Record<string, unknown>>;
    };

    expect(serialized.sections[0]).toMatchObject({
      item_format: null,
      example_item_format: null,
    });
  });

  test('omits optional fields when the draft never had them', () => {
    const serialized = JSON.parse(serializeTemplateDraft({
      name: 'Tasks',
      description: 'Assigned work',
      sections: [section],
    })) as {
      sections: Array<Record<string, unknown>>;
    };

    expect(serialized.sections[0]).not.toHaveProperty('item_format');
    expect(serialized.sections[0]).not.toHaveProperty('example_item_format');
  });
});
