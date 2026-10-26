import { readFile } from 'node:fs/promises';

import { expect, test } from '@playwright/test';

const reviewedSurfaces = [
  'src/components/shared/DownloadProgressToast.tsx',
  'src/components/ImportAudio/ImportAudioDialog.tsx',
  'src/components/onboarding/steps/WelcomeStep.tsx',
  'src/components/onboarding/steps/DownloadProgressStep.tsx',
  'src/components/AISummary/index.tsx',
];

const fixedNeutral =
  /\b(?:bg-white|bg-black|(?:bg|text|border|ring)-(?:gray|slate|zinc|neutral|stone)-\d{2,3})\b/g;

for (const file of reviewedSurfaces) {
  test(`${file} uses semantic neutral theme tokens`, async () => {
    const source = await readFile(file, 'utf8');

    expect(source.match(fixedNeutral) ?? []).toEqual([]);
  });
}
