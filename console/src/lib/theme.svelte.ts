// Theme state: 'system' respects prefers-color-scheme (the default); 'light'/'dark' pin an
// explicit choice via the [data-theme] override in app.css. Persisted so a reload keeps the
// operator's pick.

import { brand } from '$brand';

export type ThemeChoice = 'system' | 'light' | 'dark';

// Namespaced by the active brand, not by a hardcoded product name. Two brands of
// this console served from the same origin used to share one key and silently
// overwrite each other's theme choice; `storagePrefix` is unique per brand, so
// they no longer collide.
const STORAGE_KEY = `${brand.storagePrefix}:theme`;

function readInitial(): ThemeChoice {
  if (typeof localStorage === 'undefined') return 'system';
  const stored = localStorage.getItem(STORAGE_KEY);
  return stored === 'light' || stored === 'dark' ? stored : 'system';
}

class ThemeStore {
  choice = $state<ThemeChoice>(readInitial());

  constructor() {
    $effect.root(() => {
      $effect(() => {
        const root = document.documentElement;
        if (this.choice === 'system') {
          root.removeAttribute('data-theme');
        } else {
          root.setAttribute('data-theme', this.choice);
        }
        try {
          localStorage.setItem(STORAGE_KEY, this.choice);
        } catch {
          /* ignore (private mode / disabled storage) */
        }
      });
    });
  }

  toggle() {
    const effective = this.resolved();
    this.choice = effective === 'dark' ? 'light' : 'dark';
  }

  resolved(): 'light' | 'dark' {
    if (this.choice !== 'system') return this.choice;
    if (typeof matchMedia === 'undefined') return 'light';
    return matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
  }
}

export const theme = new ThemeStore();
