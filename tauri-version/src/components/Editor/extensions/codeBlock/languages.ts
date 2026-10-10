import { LanguageDescription } from '@codemirror/language';
import { languages } from '@codemirror/language-data';

export function fenceLanguage(info: string): LanguageDescription | null {
  const name = /\S*/.exec(info)?.[0];
  if (!name) return null;
  return LanguageDescription.matchLanguageName(languages, name, true);
}
