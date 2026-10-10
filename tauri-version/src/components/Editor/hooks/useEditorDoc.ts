import { useCallback, useEffect, useRef, useState } from 'react';
import { markOpenStage } from '../../../modules/perf/openTrace';
import { warmDataviewResults } from '../extensions/dataview';
import {
    flushDocuments,
    isDocumentMissing,
    onDocumentMissing,
    onDocumentSaved,
    onDocumentSaveFailed,
    openDocument,
    releaseDocument,
    type OpenedDocument,
} from '../../../modules/documents/documentGateway';
import {
    isRenameConflict,
    renameWorkspaceFile,
} from '../../../modules/documents/renameWorkspaceFile';
import { fileStem } from '../../../modules/paths';
import { failureReason, noteOpenError } from '../noteOpenError';

export function useEditorDoc(filePath: string, workspacePath?: string | null) {
    const filePathRef = useRef(filePath);
    filePathRef.current = filePath;
    const [opened, setOpened] = useState<OpenedDocument | null>(null);
    const [missing, setMissing] = useState(false);
    const [openError, setOpenError] = useState<Error | null>(null);
    const [saveError, setSaveError] = useState<string | null>(null);
    const title = fileStem(filePath);

    useEffect(() => {
        let mounted = true;
        const path = filePathRef.current;
        markOpenStage('request');
        openDocument(path)
            .then((document) => {
                if (!mounted) return;
                markOpenStage('doc');
                void warmDataviewResults(workspacePath, path, document.text);
                setOpened(document);
            })
            .catch((error: unknown) => {
                if (!mounted) return;
                if (isDocumentMissing(error)) {
                    setMissing(true);
                    return;
                }
                console.error(error);
                setOpenError(noteOpenError(path, error));
            });
        return () => {
            mounted = false;
            void releaseDocument(filePathRef.current)
                .catch((error) => console.error('Failed to release the document', error));
        };
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, []);

    useEffect(() => {
        const path = () => filePathRef.current;
        const unsubscribe = [
            onDocumentMissing(path, () => setMissing(true)),
            onDocumentSaved(path, () => setSaveError(null)),
            onDocumentSaveFailed(path, (reason) => setSaveError(reason)),
        ];
        return () => unsubscribe.forEach((stop) => stop());
    }, []);

    const renameTo = async (nextTitle: string): Promise<boolean> => {
        try {
            return (await renameWorkspaceFile(filePath, nextTitle)) !== null;
        } catch (error) {
            if (isRenameConflict(error)) console.warn(`File ${nextTitle.trim()} already exists! Reverting.`);
            else console.error('Rename failed', error);
            return false;
        }
    };

    const reportFailure = useCallback((error: unknown) => {
        console.error('Failed to sync the note with its document', error);
        setSaveError(failureReason(error));
    }, []);

    const retrySave = () => {
        void flushDocuments().catch((error) => console.error('Failed to save documents', error));
    };

    return {
        saveError,
        retrySave,
        reportFailure,
        opened,
        isReady: opened !== null,
        title,
        renameTo,
        missing,
        openError,
    };
}
