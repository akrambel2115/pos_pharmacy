import ar from './ar.json';
import fr from './fr.json';

export type Language = 'ar' | 'fr';

class TranslationState {
  current = $state<Language>('fr');
  
  setLanguage(lang: Language) {
    this.current = lang;
    if (typeof document !== 'undefined') {
      document.documentElement.lang = lang;
      document.documentElement.dir = lang === 'ar' ? 'rtl' : 'ltr';
    }
  }
}

export const langState = new TranslationState();

const translations = { ar, fr };

export type TranslationKey = keyof typeof fr;

// Svelte 5 translation helper
export function t(key: TranslationKey, vars?: Record<string, string | number>): string {
  const dict = translations[langState.current];
  let text = (dict as any)[key] || (fr as any)[key] || key;
  if (vars) {
    Object.keys(vars).forEach(k => {
      text = text.replace(`{${k}}`, String(vars[k]));
    });
  }
  return text;
}
