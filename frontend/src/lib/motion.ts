import type { Variants } from 'framer-motion';

export const createStepContainerVariants = (
  reduceMotion: boolean,
): Variants => ({
  hidden: { opacity: 0 },
  visible: {
    opacity: 1,
    transition: reduceMotion ? { duration: 0.1 } : { staggerChildren: 0.1 },
  },
  exit: { opacity: 0, transition: { duration: 0.15 } },
});

export const createStepItemVariants = (reduceMotion: boolean): Variants => ({
  hidden: reduceMotion
    ? { opacity: 0 }
    : { opacity: 0, y: 12, filter: 'blur(4px)' },
  visible: reduceMotion
    ? { opacity: 1, transition: { duration: 0.1 } }
    : {
        opacity: 1,
        y: 0,
        filter: 'blur(0px)',
        transition: { duration: 0.3 },
      },
  exit: reduceMotion
    ? { opacity: 0, transition: { duration: 0.1 } }
    : {
        opacity: 0,
        y: -12,
        filter: 'blur(4px)',
        transition: { duration: 0.15 },
      },
});

export const iconSpring = {
  type: 'spring',
  duration: 0.3,
  bounce: 0,
} as const;
