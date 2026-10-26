import { expect, test, type Page } from '@playwright/test';

type MotionSample = {
  filter: string;
  flowCount?: number;
  height?: number;
  opacity: number;
  state?: string;
  transform: string;
  width?: number;
  x?: number;
  y?: number;
};

async function emitTauriEvent(
  page: Page,
  event: string,
  payload: Record<string, unknown>,
) {
  await page.evaluate(
    async ({ eventName, eventPayload }) => {
      const emit = (
        window as typeof window & {
          __emitTauriEvent?: (
            event: string,
            payload: Record<string, unknown>,
          ) => Promise<void>;
        }
      ).__emitTauriEvent;

      if (!emit) throw new Error('E2E Tauri event bridge is unavailable');
      await emit(eventName, eventPayload);
    },
    { eventName: event, eventPayload: payload },
  );
}

async function collectStepTransition(
  page: Page,
  buttonName: string,
  destinationHeading: string,
) {
  return page.evaluate(
    ({ buttonName, destinationHeading }) =>
      new Promise<MotionSample[]>((resolve, reject) => {
        const button = [...document.querySelectorAll('button')].find(
          (candidate) => candidate.textContent?.trim() === buttonName,
        );
        if (!(button instanceof HTMLButtonElement)) {
          reject(new Error(`Missing transition control: ${buttonName}`));
          return;
        }

        const samples: MotionSample[] = [];
        let stableFrames = 0;
        let frame = 0;
        button.click();

        const sample = () => {
          const flows = [...document.querySelectorAll<HTMLElement>('.onboarding-flow')];
          const heading = [...document.querySelectorAll('h1')].find(
            (candidate) => candidate.textContent?.trim() === destinationHeading,
          );
          const animatedHeader = heading?.parentElement;
          const style = animatedHeader
            ? getComputedStyle(animatedHeader)
            : undefined;
          const state = flows.map((flow) => flow.textContent ?? '').join(' ');

          samples.push({
            filter: style?.filter ?? 'unmounted',
            flowCount: flows.length,
            opacity: style ? Number(style.opacity) : 0,
            state,
            transform: style?.transform ?? 'unmounted',
          });

          const isSettled =
            Boolean(heading) &&
            style?.opacity === '1' &&
            (style.transform === 'none' ||
              style.transform === 'matrix(1, 0, 0, 1, 0, 0)') &&
            (style.filter === 'none' || style.filter === 'blur(0px)');
          stableFrames = isSettled ? stableFrames + 1 : 0;

          if (stableFrames >= 3) {
            resolve(samples);
            return;
          }
          if (frame++ > 300) {
            reject(new Error(`Transition did not settle: ${destinationHeading}`));
            return;
          }
          requestAnimationFrame(sample);
        };

        requestAnimationFrame(sample);
      }),
    { buttonName, destinationHeading },
  );
}

async function collectToastIconSwap(
  page: Page,
  event: string,
  payload: Record<string, unknown>,
  settledText: string,
) {
  return page.evaluate(
    ({ eventName, eventPayload, settledText }) =>
      new Promise<MotionSample[]>(async (resolve, reject) => {
        const emit = (
          window as typeof window & {
            __emitTauriEvent?: (
              event: string,
              payload: Record<string, unknown>,
            ) => Promise<void>;
          }
        ).__emitTauriEvent;
        if (!emit) {
          reject(new Error('E2E Tauri event bridge is unavailable'));
          return;
        }

        const findIcon = () =>
          document.querySelector<HTMLElement>(
            '[data-sonner-toast] [aria-hidden="true"]',
          );
        const initialIcon = findIcon();
        if (!initialIcon) {
          reject(new Error('Missing contextual toast icon'));
          return;
        }
        const initialToast = initialIcon.closest<HTMLElement>('[data-sonner-toast]');
        const initialRect = initialIcon.getBoundingClientRect();
        const initialToastRect = initialToast?.getBoundingClientRect();
        const initialRelativeX = initialRect.x - (initialToastRect?.x ?? 0);
        const initialRelativeY = initialRect.y - (initialToastRect?.y ?? 0);
        const samples: MotionSample[] = [];
        let stableFrames = 0;
        let frame = 0;

        await emit(eventName, eventPayload);

        const sample = () => {
          const icon = findIcon();
          const animatedIcon = icon?.firstElementChild;
          const style = animatedIcon
            ? getComputedStyle(animatedIcon)
            : undefined;
          const rect = icon?.getBoundingClientRect();
          const toast = document.querySelector<HTMLElement>('[data-sonner-toast]');
          const toastRect = toast?.getBoundingClientRect();
          const relativeX = rect && toastRect ? rect.x - toastRect.x : undefined;
          const relativeY = rect && toastRect ? rect.y - toastRect.y : undefined;

          samples.push({
            filter: style?.filter ?? 'unmounted',
            height: rect?.height,
            opacity: style ? Number(style.opacity) : 0,
            state: toast?.textContent ?? '',
            transform: style?.transform ?? 'unmounted',
            width: rect?.width,
            x: relativeX,
            y: relativeY,
          });

          const geometryStable =
            rect !== undefined &&
            relativeX !== undefined &&
            relativeY !== undefined &&
            Math.abs(relativeX - initialRelativeX) <= 0.5 &&
            Math.abs(relativeY - initialRelativeY) <= 0.5 &&
            Math.abs(rect.width - initialRect.width) <= 0.5 &&
            Math.abs(rect.height - initialRect.height) <= 0.5;
          const isSettled =
            geometryStable &&
            (toast?.textContent ?? '').includes(settledText) &&
            style?.opacity === '1' &&
            (style.transform === 'none' ||
              style.transform === 'matrix(1, 0, 0, 1, 0, 0)') &&
            (style.filter === 'none' || style.filter === 'blur(0px)');
          stableFrames = isSettled ? stableFrames + 1 : 0;

          if (stableFrames >= 3) {
            resolve(samples);
            return;
          }
          if (frame++ > 240) {
            const settledStateSamples = samples.filter((sample) =>
              sample.state?.includes(settledText),
            );
            reject(
              new Error(
                `Icon swap did not settle: ${settledText}; initial=${JSON.stringify({
                  height: initialRect.height,
                  width: initialRect.width,
                  x: initialRelativeX,
                  y: initialRelativeY,
                })}; firstState=${JSON.stringify(settledStateSamples[0])}; lastState=${JSON.stringify(settledStateSamples.at(-1))}; last=${JSON.stringify(samples.at(-1))}`,
              ),
            );
            return;
          }
          requestAnimationFrame(sample);
        };

        requestAnimationFrame(sample);
      }),
    { eventName: event, eventPayload: payload, settledText },
  );
}

test('press scale runs as a transform transition and adapts to live reduced motion', async ({
  page,
}) => {
  await page.emulateMedia({ reducedMotion: 'no-preference' });
  await page.goto('/settings');

  const button = page.getByRole('button', { name: 'Dark', exact: true });
  await expect(button).toBeVisible();
  await expect(button).toHaveCSS(
    'transition-property',
    'color, background-color, box-shadow, transform',
  );
  await expect(button).toHaveCSS('transition-duration', '0.15s');

  const box = await button.boundingBox();
  if (!box) throw new Error('Dark appearance button has no layout box');
  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
  await page.mouse.down();
  await expect
    .poll(() =>
      button.evaluate((element) =>
        element.getAnimations().some(
          (animation) =>
            animation instanceof CSSTransition &&
            animation.transitionProperty === 'transform' &&
            animation.effect?.getTiming().duration === 150,
        ),
      ),
    )
    .toBe(true);
  await expect(button).not.toHaveCSS('transform', 'none');
  await page.mouse.up();

  await page.emulateMedia({ reducedMotion: 'reduce' });
  await expect(button).toHaveCSS('transition-property', 'none');
  await page.mouse.down();
  await expect(button).toHaveCSS('transform', 'none');
  expect(await button.evaluate((element) => element.getAnimations().length)).toBe(0);
  await page.mouse.up();

  await page.emulateMedia({ reducedMotion: 'no-preference' });
  await expect(button).toHaveCSS(
    'transition-property',
    'color, background-color, box-shadow, transform',
  );
});

test('onboarding runs one ordered enter/exit sequence and reacts to reduced motion live', async ({
  page,
}) => {
  await page.emulateMedia({ reducedMotion: 'no-preference' });
  await page.goto('/?__e2e=onboarding');

  const welcome = page.getByRole('heading', { name: 'Welcome to Meetily' });
  await expect(welcome).toBeVisible();
  await expect(page.locator('.onboarding-flow')).toHaveAttribute(
    'data-reduced-motion',
    'false',
  );
  const initialStyle = await welcome.evaluate((heading) => {
    const style = getComputedStyle(heading.parentElement!);
    return {
      filter: style.filter,
      opacity: style.opacity,
      transform: style.transform,
    };
  });
  expect(['none', 'blur(0px)']).toContain(initialStyle.filter);
  expect(initialStyle.opacity).toBe('1');
  expect(initialStyle.transform).toBe('none');

  const normalSamples = await collectStepTransition(
    page,
    'Get Started',
    'Setup Overview',
  );
  await expect(page.getByRole('heading', { name: 'Setup Overview' })).toBeVisible();
  expect(Math.max(...normalSamples.map((sample) => sample.flowCount ?? 0))).toBe(1);
  expect(
    normalSamples.some(
      (sample) =>
        sample.state?.includes('Setup Overview') &&
        (sample.opacity < 1 ||
          !['none', 'matrix(1, 0, 0, 1, 0, 0)'].includes(sample.transform) ||
          !['none', 'blur(0px)'].includes(sample.filter)),
    ),
  ).toBe(true);

  await page.emulateMedia({ reducedMotion: 'reduce' });
  await expect
    .poll(() => page.evaluate(() => matchMedia('(prefers-reduced-motion: reduce)').matches))
    .toBe(true);
  await expect(page.locator('.onboarding-flow')).toHaveAttribute(
    'data-reduced-motion',
    'true',
  );
  const reducedSamples = await collectStepTransition(
    page,
    "Let's Go",
    'Getting things ready',
  );
  expect(Math.max(...reducedSamples.map((sample) => sample.flowCount ?? 0))).toBe(1);
  const mountedReducedSamples = reducedSamples.filter((sample) =>
    sample.state?.includes('Getting things ready'),
  );
  expect(mountedReducedSamples.length).toBeGreaterThan(0);
  expect(
    mountedReducedSamples.every((sample) =>
      ['none', 'matrix(1, 0, 0, 1, 0, 0)'].includes(sample.transform),
    ),
  ).toBe(true);
  expect(
    mountedReducedSamples.every((sample) =>
      ['none', 'blur(0px)'].includes(sample.filter),
    ),
  ).toBe(true);
});

test('contextual icon swaps preserve geometry and accessible status under both motion modes', async ({
  page,
}) => {
  await page.emulateMedia({ reducedMotion: 'no-preference' });
  await page.goto('/');
  await expect(page.getByRole('button', { name: 'Start Recording' })).toBeVisible();
  await page.evaluate(
    () =>
      new Promise<void>((resolve) =>
        requestAnimationFrame(() => requestAnimationFrame(() => resolve())),
      ),
  );

  await emitTauriEvent(page, 'parakeet-model-download-complete', {
    modelName: 'parakeet-tdt-0.6b-v3-int8',
  });
  const toast = page.locator('[data-sonner-toast]').filter({
    hasText: 'Transcription Model (Parakeet)',
  });
  await expect(toast).toContainText('Download complete');
  const icon = toast.locator('[aria-hidden="true"]');
  await expect(icon).toHaveAttribute('aria-hidden', 'true');
  await expect(icon).toHaveAttribute('data-reduced-motion', 'false');
  await expect(toast.getByText('Download complete')).toBeVisible();
  await expect(icon.locator('svg.lucide-check')).toBeVisible();

  const normalSamples = await collectToastIconSwap(
    page,
    'parakeet-model-download-error',
    {
      modelName: 'parakeet-tdt-0.6b-v3-int8',
      error: 'Invalid checksum',
    },
    'File validation failed',
  );
  await expect(toast.getByText('File validation failed')).toBeVisible();
  await expect(icon.locator('svg.lucide-x')).toBeVisible();
  expect(
    normalSamples.some(
      (sample) =>
        !['none', 'matrix(1, 0, 0, 1, 0, 0)'].includes(sample.transform) ||
        !['none', 'blur(0px)'].includes(sample.filter),
    ),
  ).toBe(true);
  const firstNormal = normalSamples[0];
  for (const sample of normalSamples) {
    expect(sample.width).toBeCloseTo(firstNormal.width ?? 0, 1);
    expect(sample.height).toBeCloseTo(firstNormal.height ?? 0, 1);
    expect(sample.x).toBeCloseTo(firstNormal.x ?? 0, 1);
    expect(sample.y).toBeCloseTo(firstNormal.y ?? 0, 1);
  }

  await page.emulateMedia({ reducedMotion: 'reduce' });
  await expect(icon).toHaveAttribute('data-reduced-motion', 'true');
  const reducedSamples = await collectToastIconSwap(
    page,
    'parakeet-model-download-complete',
    { modelName: 'parakeet-tdt-0.6b-v3-int8' },
    'Download complete',
  );
  await expect(toast.getByText('Download complete')).toBeVisible();
  await expect(icon.locator('svg.lucide-check')).toBeVisible();
  expect(
    reducedSamples
      .filter(
        (sample) =>
          !['none', 'matrix(1, 0, 0, 1, 0, 0)', 'unmounted'].includes(
            sample.transform,
          ),
      )
      .slice(0, 5),
  ).toEqual([]);
  expect(
    reducedSamples.every((sample) =>
      ['none', 'blur(0px)', 'unmounted'].includes(sample.filter),
    ),
  ).toBe(true);
});

test('onboarding progress fits a 320px viewport without shrinking or clipping hit areas', async ({
  browser,
}) => {
  const context = await browser.newContext({
    baseURL: 'http://127.0.0.1:3118',
    reducedMotion: 'reduce',
    userAgent:
      'Mozilla/5.0 (Macintosh; Intel Mac OS X 14_5) AppleWebKit/537.36 Chrome/124 Safari/537.36',
    viewport: { width: 320, height: 700 },
  });
  const page = await context.newPage();
  await page.goto('/?__e2e=onboarding');
  await page.getByRole('button', { name: 'Get Started' }).click();
  await expect(page.getByRole('heading', { name: 'Setup Overview' })).toBeVisible();

  const steps = page.getByRole('button', { name: /^Step \d:/ });
  await expect(steps).toHaveCount(4);
  const count = await steps.count();
  const boxes = [];
  for (let index = 0; index < count; index += 1) {
    const step = steps.nth(index);
    const box = await step.boundingBox();
    if (!box) throw new Error(`Step ${index + 1} has no layout box`);
    expect(box.width).toBeGreaterThanOrEqual(40);
    expect(box.height).toBeGreaterThanOrEqual(40);
    expect(box.x).toBeGreaterThanOrEqual(24);
    expect(box.x + box.width).toBeLessThanOrEqual(296);
    boxes.push(box);
  }
  for (let index = 1; index < boxes.length; index += 1) {
    expect(boxes[index].x).toBeGreaterThanOrEqual(
      boxes[index - 1].x + boxes[index - 1].width,
    );
  }
  await expect(steps.nth(1)).toHaveAttribute('aria-current', 'step');
  await expect(steps.nth(0)).toHaveAccessibleName('Step 1: completed');

  const overflow = await page.evaluate(() => ({
    clientWidth: document.documentElement.clientWidth,
    scrollWidth: document.documentElement.scrollWidth,
  }));
  expect(overflow.scrollWidth).toBeLessThanOrEqual(overflow.clientWidth);
  await context.close();
});

test('retains deterministic onboarding visual evidence', async ({ page, browser }) => {
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto('/?__e2e=onboarding');
  await page.getByRole('button', { name: 'Get Started' }).click();
  await expect(page.getByRole('heading', { name: 'Setup Overview' })).toBeVisible();
  await expect(page).toHaveScreenshot(
    'interface-polish-onboarding-step2.png',
    { animations: 'disabled' },
  );

  const narrowContext = await browser.newContext({
    baseURL: 'http://127.0.0.1:3118',
    reducedMotion: 'reduce',
    userAgent:
      'Mozilla/5.0 (Macintosh; Intel Mac OS X 14_5) AppleWebKit/537.36 Chrome/124 Safari/537.36',
    viewport: { width: 320, height: 700 },
  });
  const narrowPage = await narrowContext.newPage();
  await narrowPage.goto('/?__e2e=onboarding');
  await narrowPage.getByRole('button', { name: 'Get Started' }).click();
  await expect(narrowPage.getByRole('button', { name: /^Step \d:/ })).toHaveCount(4);
  await expect(narrowPage).toHaveScreenshot(
    'interface-polish-onboarding-step2-narrow.png',
    { animations: 'disabled' },
  );
  await narrowContext.close();
});

test('reduced-motion store hydrates deterministically and cleans up navigation listeners', async ({
  page,
}) => {
  const hydrationErrors: string[] = [];
  page.on('console', (message) => {
    if (
      message.type() === 'error' &&
      /hydration|did not match|server html/i.test(message.text())
    ) {
      hydrationErrors.push(message.text());
    }
  });
  page.on('pageerror', (error) => {
    if (/hydration|did not match|server html/i.test(error.message)) {
      hydrationErrors.push(error.message);
    }
  });

  await page.addInitScript(() => {
    const originalMatchMedia = window.matchMedia.bind(window);
    const stats = {
      active: new Set<EventListenerOrEventListenerObject>(),
      adds: 0,
      removes: 0,
    };
    (
      window as typeof window & {
        __reducedMotionListenerStats?: typeof stats;
      }
    ).__reducedMotionListenerStats = stats;

    window.matchMedia = (query: string) => {
      const mediaQuery = originalMatchMedia(query);
      if (query !== '(prefers-reduced-motion: reduce)') return mediaQuery;

      const addEventListener = mediaQuery.addEventListener.bind(mediaQuery);
      const removeEventListener = mediaQuery.removeEventListener.bind(mediaQuery);
      Object.defineProperties(mediaQuery, {
        addEventListener: {
          value: (
            type: string,
            listener: EventListenerOrEventListenerObject,
            options?: boolean | AddEventListenerOptions,
          ) => {
            if (type === 'change') {
              stats.adds += 1;
              stats.active.add(listener);
            }
            addEventListener(type, listener, options);
          },
        },
        removeEventListener: {
          value: (
            type: string,
            listener: EventListenerOrEventListenerObject,
            options?: boolean | EventListenerOptions,
          ) => {
            if (type === 'change') {
              stats.removes += 1;
              stats.active.delete(listener);
            }
            removeEventListener(type, listener, options);
          },
        },
      });
      return mediaQuery;
    };
  });
  await page.emulateMedia({ reducedMotion: 'reduce' });

  const response = await page.goto('/');
  if (!response) throw new Error('Home navigation returned no response');
  const serverHtml = await response.text();
  expect(serverHtml).toMatch(
    /data-testid="reduced-motion-hydration-probe"[^>]*data-reduced-motion="false"/,
  );

  const probe = page.getByTestId('reduced-motion-hydration-probe');
  await expect(probe).toHaveAttribute('data-reduced-motion', 'true');
  await expect
    .poll(() =>
      page.evaluate(() => {
        const stats = (
          window as typeof window & {
            __reducedMotionListenerStats: {
              active: Set<EventListenerOrEventListenerObject>;
              adds: number;
              removes: number;
            };
          }
        ).__reducedMotionListenerStats;
        return {
          active: stats.active.size,
          adds: stats.adds,
          removes: stats.removes,
        };
      }),
    )
    .toEqual({ active: 1, adds: 1, removes: 0 });

  await page.getByRole('button', { name: 'Settings' }).click();
  await expect(page).toHaveURL('/settings');
  await expect(probe).toHaveCount(0);
  await expect
    .poll(() =>
      page.evaluate(() => {
        const stats = (
          window as typeof window & {
            __reducedMotionListenerStats: {
              active: Set<EventListenerOrEventListenerObject>;
              removes: number;
            };
          }
        ).__reducedMotionListenerStats;
        return { active: stats.active.size, removes: stats.removes };
      }),
    )
    .toEqual({ active: 0, removes: 1 });

  await page.getByRole('button', { name: 'Home' }).click();
  await expect(page).toHaveURL('/');
  await expect(probe).toHaveAttribute('data-reduced-motion', 'true');
  await expect
    .poll(() =>
      page.evaluate(() => {
        const stats = (
          window as typeof window & {
            __reducedMotionListenerStats: {
              active: Set<EventListenerOrEventListenerObject>;
              adds: number;
              removes: number;
            };
          }
        ).__reducedMotionListenerStats;
        return {
          active: stats.active.size,
          adds: stats.adds,
          removes: stats.removes,
        };
      }),
    )
    .toEqual({ active: 1, adds: 2, removes: 1 });
  expect(hydrationErrors).toEqual([]);
});
