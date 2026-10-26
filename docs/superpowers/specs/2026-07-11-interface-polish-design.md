# Meetily Interface Polish Design

## Intent

Apply the complete `Polish Audit` to the existing Meetily interface without changing product workflows or introducing a new visual direction. The work stays on `build/universal-macos-stable` and preserves the verified light theme, dark theme, Universal macOS packaging, audio behavior, and backend contracts.

## Scope

The implementation covers six coordinated layers:

1. Semantic surface geometry and concentric border radii.
2. Depth, borders, and shadows for cards, controls, and overlays.
3. Hit areas and tactile press feedback for interactive controls.
4. Typography wrapping and stable dynamic numerals.
5. Explicit, interruptible transitions and contextual motion.
6. Reduced-motion behavior and visual regression coverage.

The work does not redesign navigation, change information architecture, animate transcript or summary lists, add dependencies, modify audio/backend code, or alter build signing and packaging behavior.

## Surface Geometry

Do not globally increase `--radius`. The current token remains `0.5rem` so dense controls and third-party components do not expand unexpectedly. Apply geometry by semantic role:

- Form controls and compact buttons use the existing `rounded-md` or `rounded-lg` primitives.
- Standalone cards use `rounded-xl`.
- Elevated dialogs, sheets, and prominent onboarding surfaces use `rounded-2xl`.
- Nested surfaces within an 8 px inset use `rounded-2xl` outside and `rounded-lg` inside.
- Layers separated by more than 24 px are independent surfaces and do not need strict concentric radii.
- Pills, status dots, switches, and circular icon controls retain `rounded-full`.

The first implementation pass targets shared UI primitives, settings cards, onboarding, recording controls, dialogs, toasts, import surfaces, and model cards. Long transcript and summary content remains geometrically compact.

## Borders and Depth

Borders retain accessibility and structural roles:

- Inputs, textareas, selects, focus rings, dividers, tables, and dense list separators keep explicit borders.
- Standalone cards and elevated containers use a semantic layered shadow instead of relying on a hard border for depth.
- Dialogs, sheets, popovers, and dropdowns keep a subtle boundary plus elevation because they may overlap varied content.

Add two semantic shadow variables:

- Light `--shadow-surface`: a low-opacity 1 px black ring plus two shallow lift layers.
- Light `--shadow-surface-hover`: a slightly stronger version of the same layers.
- Dark values: a single low-opacity white ring, strengthened on hover.

Only `box-shadow` transitions. Layout divider borders never animate.

## Interaction Geometry

Interactive controls must expose at least a 40×40 px hit area on application surfaces covered by browser tests:

- The shared default button height becomes 40 px.
- Shared icon buttons become 40×40 px.
- Visually smaller controls may keep their visible geometry only when a non-overlapping pseudo-element or wrapper expands the hit area to 40×40 px.
- Dense segmented controls may use 36 px visible height when their parent provides a 40 px row and the effective pointer target remains at least 40 px.

Buttons receive tactile press feedback:

- `scale(0.96)` on active press.
- Explicit transition properties for scale and the color or shadow properties the variant changes.
- `motion-reduce:transform-none` and `motion-reduce:transition-none`.
- Disabled controls never scale.

## Typography

Typography changes are targeted rather than global:

- Page, dialog, onboarding, and card headings use `text-balance`.
- Short descriptions, helper text, warnings, and empty-state copy use `text-pretty`.
- Long transcripts, generated summaries, code, logs, and preformatted content use normal wrapping.
- Recording time, elapsed time, percentages, progress values, and changing numeric counters use `tabular-nums`.
- Root font smoothing remains `antialiased`; it is verified rather than duplicated per component.

## Motion Architecture

The existing `framer-motion` dependency is used; no package changes are required. Shared variants live in a focused frontend module so timing and reduced-motion behavior stay consistent.

Motion is limited to:

- Onboarding step transitions.
- Dialog and sheet content groups where the primitive already animates visibility.
- Toast and status appearance/removal.
- Contextual icons that swap in place.
- Recording state controls where an icon changes without changing layout.

Motion is excluded from:

- Transcript rows and virtualized transcript content.
- Generated summary blocks and editors.
- Model lists and other long collections.
- Progress width updates, which remain CSS transitions.
- Initial page rendering.

### Enter and Exit

- `AnimatePresence` uses `initial={false}`.
- Onboarding content is split into title, description/content, and actions.
- Children enter with opacity, `translateY(12px)`, and blur from 4 px, staggered by 100 ms.
- Exit uses opacity, `translateY(-12px)`, and blur to 4 px over 150 ms.
- Interactive state motion uses duration-based springs with `duration: 0.3` and `bounce: 0`.

### Contextual Icons

Both icon states remain present while transitioning when layout permits:

- Enter: opacity `0→1`, scale `0.25→1`, blur `4px→0`.
- Exit: opacity `1→0`, scale `1→0.25`, blur `0→4px`.
- The inactive icon is absolutely overlaid so the stable icon defines layout size.
- Icon animation never changes the button's measured width or height.

### Reduced Motion

Use `useReducedMotion` for Framer Motion variants. Under reduced motion:

- Transform, scale, blur, and stagger are disabled.
- Content may use a short opacity transition.
- CSS press transforms and decorative transitions are disabled with `motion-reduce` utilities.
- Functional progress and state visibility remain available.

## Transition Cleanup

Remove every application-owned `transition-all` occurrence under `frontend/src` and replace it according to the properties that actually change:

- Colors: `transition-colors`.
- Opacity: `transition-opacity`.
- Scale/translate/rotate: `transition-transform`.
- Width: `transition-[width]`.
- Shadow: `transition-[box-shadow]`.
- Mixed contextual icons: `transition-[opacity,filter,scale]`.

Do not add `will-change` proactively. It is allowed only if browser verification shows a reproducible first-frame stutter on `transform`, `opacity`, or `filter`.

## Components and Boundaries

Implementation is grouped by responsibility:

- `frontend/src/app/globals.css`: semantic shadow variables and reusable surface utilities.
- `frontend/src/lib/motion.ts`: shared motion variants and reduced-motion selectors.
- `frontend/src/components/ui/*`: button geometry, dialogs, sheets, popovers, dropdowns, inputs, and related primitives.
- Onboarding components: staggered step content and concentric surfaces.
- Recording/status/toast components: hit areas, tabular values, contextual icon states.
- Settings/model/import surfaces: role-based radii, shadows, headings, and descriptions.
- Tests: static policy assertions, computed geometry checks, reduced-motion behavior, and existing light/dark route coverage.

No large component is split solely for this polish pass. A helper is extracted only when the same motion behavior has at least two consumers.

## Testing Strategy

Use test-driven changes in small batches:

1. Static policy test fails on `transition-all` and missing shared polish tokens.
2. Primitive browser tests verify 40×40 hit areas, semantic radii, press/reduced-motion classes, and overlay geometry.
3. Typography tests verify balanced headings, pretty short copy, and tabular dynamic values on representative routes.
4. Motion tests emulate `prefers-reduced-motion` and verify no transform-driven initial animation.
5. Existing light/dark surface tests and screenshots guard theme contrast and layout.
6. Full Playwright, theme audit, Next production build, and Universal packaging configuration tests remain green.

Screenshot changes are accepted only when they match the specified geometry and motion states. Tests must not weaken semantic contrast or ignore newly light surfaces.

## Acceptance Criteria

- No `transition-all` remains in application-owned code under `frontend/src`.
- Tested shared and route-level interactive targets are at least 40×40 px.
- Shared buttons press to exactly `0.96` and disable transform under reduced motion.
- Representative cards, nested surfaces, and overlays use role-based concentric radii.
- Surface shadows adapt correctly in light and dark themes.
- Representative headings use balanced wrapping and short copy uses pretty wrapping.
- Recording/progress numeric values use tabular numerals.
- Onboarding, dialogs/toasts, and contextual icons follow the specified enter/exit values.
- `AnimatePresence` wrappers do not animate initial page render.
- Reduced-motion mode removes transform, blur, scale, and stagger effects.
- Transcript, summary, virtualized lists, audio behavior, and backend contracts are unchanged.
- Theme audit, full E2E, production frontend build, and Universal build configuration tests pass.

## Risks and Controls

- **Density regression:** avoid a global radius or spacing token bump; verify settings and model rows at their existing viewport sizes.
- **Motion overload:** animate only the named surfaces and exclude long lists and initial page load.
- **Hit-area overlap:** browser tests inspect effective geometry; pseudo-elements are used only where neighboring targets cannot overlap.
- **Dark-theme depth loss:** dark shadows use a white alpha ring instead of invisible black elevation.
- **Snapshot churn:** update only snapshots whose geometry changes intentionally and review each changed image.
- **Performance regression:** use explicit CSS properties, no blanket `will-change`, and no list-level Framer Motion wrappers.
