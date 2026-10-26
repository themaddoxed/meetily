import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';
import assert from 'node:assert/strict';

const testDirectory = dirname(fileURLToPath(import.meta.url));
const source = (relativePath) => readFileSync(join(testDirectory, '../../src', relativePath), 'utf8');

test('template hook keeps CRUD operations and selection persistence in one API', () => {
  const hook = source('hooks/meeting-details/useTemplates.ts');

  for (const command of [
    'api_list_templates',
    'api_get_template_details',
    'api_validate_template',
    'api_create_custom_template',
    'api_copy_template',
    'api_update_custom_template',
    'api_delete_custom_template',
    'api_is_custom_template',
  ]) {
    assert.match(hook, new RegExp(command));
  }
  assert.match(hook, /localStorage/);
  assert.match(hook, /fallbackTemplateId/);
});

test('template editor is a structured form with validation and built-in copy flow', () => {
  const editor = source('components/MeetingDetails/TemplateEditorModal.tsx');

  assert.match(editor, /value=\{draft\.name\}/);
  assert.match(editor, /value=\{draft\.description\}/);
  assert.match(editor, /draft\.sections\.map/);
  assert.match(editor, /manager\.validateTemplate/);
  assert.match(editor, /manager\.copyTemplate/);
  assert.match(editor, /manager\.updateCustomTemplate/);
  assert.match(editor, /manager\.createCustomTemplate/);
  assert.match(editor, /value="paragraph"/);
  assert.doesNotMatch(editor, /textarea[^>]+value=\{JSON/);
});

test('meeting details wires the template manager into summary controls', () => {
  const page = source('app/meeting-details/page-content.tsx');
  const panel = source('components/MeetingDetails/SummaryPanel.tsx');
  const controls = source('components/MeetingDetails/SummaryGeneratorButtonGroup.tsx');

  assert.match(page, /templateManager=\{templates\}/);
  assert.match(panel, /templateManager: TemplateManager/);
  assert.match(controls, /TemplateEditorModal/);
});
