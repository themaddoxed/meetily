'use client';

import { usePathname } from 'next/navigation';

import { useLiveReducedMotion } from '@/hooks/use-live-reduced-motion';

import { installTauriBrowserMocks } from './tauri-browser-mocks';

installTauriBrowserMocks();

function ReducedMotionHydrationProbe() {
  const reduceMotion = useLiveReducedMotion();

  return (
    <span
      hidden
      data-testid="reduced-motion-hydration-probe"
      data-reduced-motion={reduceMotion}
    />
  );
}

export function E2EBootstrap() {
  const pathname = usePathname();
  return pathname === '/' ? <ReducedMotionHydrationProbe /> : null;
}
