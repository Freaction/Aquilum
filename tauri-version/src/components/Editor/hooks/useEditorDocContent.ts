import { useLayoutEffect, useState } from 'react';
import type { Text } from '@codemirror/state';

export const DOCUMENT_HEAD_LIMIT = 16_384;

export function documentHead(doc: Text): string {
  return doc.sliceString(0, Math.min(doc.length, DOCUMENT_HEAD_LIMIT));
}

export function useEditorDocContent(initialBody: string) {
  const [docContent, setDocContent] = useState(initialBody);

  useLayoutEffect(() => {
    setDocContent(initialBody);
  }, [initialBody]);

  return { initialBody, docContent, setDocContent };
}
