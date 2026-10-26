# Interface polish visual evidence

This branch retains deterministic Playwright snapshots for the Wave 5b onboarding surface. Both are captured with reduced motion and the repository Tauri browser fixtures; the narrow case uses a macOS user agent so all four progress steps are rendered.

| Evidence | Viewport | SHA-256 |
| --- | --- | --- |
| `frontend/tests/interface-polish-acceptance.spec.ts-snapshots/interface-polish-onboarding-step2-chromium-darwin.png` | 1440 x 1000 | `6e90dec6dc58c98d35b05f0fc9501d8dd29c1dc8bed9dc9f24db903df44a3152` |
| `frontend/tests/interface-polish-acceptance.spec.ts-snapshots/interface-polish-onboarding-step2-narrow-chromium-darwin.png` | 320 x 700 | `bed6eef6ad432308c8557f09606860c3297a25f513e121813af66334543b7b24` |

The desktop and narrow snapshots were inspected directly. The progress controls, headings, cards, and primary action remain legible without clipping or overlap. At 320 px, four 40 x 40 progress controls remain inside the 24 px content gutters, with non-overlapping hit areas and responsive connectors.

Reproduce from `frontend/`:

```bash
pnpm exec playwright test tests/interface-polish-acceptance.spec.ts -g "visual evidence" --workers=1
```

Update intentionally changed snapshots with:

```bash
pnpm exec playwright test tests/interface-polish-acceptance.spec.ts -g "visual evidence" --workers=1 --update-snapshots
```

The in-app Browser runtime reported no available browser (`agent.browsers.list() = []`), so the repository's pinned Playwright Chromium path was used. The Playwright web server binds explicitly to `127.0.0.1:3118`; no external network or GitHub operation is part of reproduction.

Verification on this branch:

- Focused interface policy/runtime/acceptance: `19/19` passed after the hydration-safe reduced-motion change. The runtime case verifies deterministic server `false` markup, client hydration to the live reduced-motion value without a mismatch, and exact listener removal/remount across client navigation.
- Full Playwright coverage before the final hook-only hydration change: `77/77` passed as four sequential single-worker shards (`20 + 26 + 15 + 16`). A fresh expanded full-suite rerun was not completed because the local execution approval capacity rejected the remaining shard; no `79/79` claim is made.
- TypeScript, theme audit, production E2E boundary (`5/5`), and fixture-free production build (`11/11` static pages) passed.
- The hydration probe remains test-only: production rejects the real E2E bootstrap and normal production resolves it to the no-op bootstrap.
