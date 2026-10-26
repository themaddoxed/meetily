import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

const frontendDirectory = path.join(
  path.dirname(fileURLToPath(import.meta.url)),
  '..',
  '..',
);

function read(relativePath) {
  return fs.readFileSync(path.join(frontendDirectory, relativePath), 'utf8');
}

test('manual import title provenance reaches the native import command', () => {
  const dialog = read('src/components/ImportAudio/ImportAudioDialog.tsx');
  const hook = read('src/hooks/useImportAudio.ts');

  assert.match(dialog, /setTitleModifiedByUser\(true\)/);
  assert.match(
    dialog,
    /selectedModel\?\.provider \|\| null,\s*titleModifiedByUser\s*\n\s*\);/,
  );
  assert.match(
    hook,
    /titleModifiedByUser = false[\s\S]*?titleModifiedByUser,\s*\n\s*\}\);/,
  );
});

test('the native import command keeps omitted provenance backward compatible', () => {
  const nativeImport = read('src-tauri/src/audio/import.rs');

  assert.match(nativeImport, /title_modified_by_user: Option<bool>/);
  assert.match(nativeImport, /title_modified_by_user\.unwrap_or\(false\)/);
  assert.match(nativeImport, /if title_modified_by_user \{\s*"manual"\s*\} else \{\s*"suggested"/s);
});
