import { t } from '../../../../i18n';
import { Prec, type Extension } from '@codemirror/state';
import { EditorView, keymap } from '@codemirror/view';
import {
    aquilumCodeMirrorTheme,
    aquilumFieldSetup,
    aquilumSingleLineExtensions,
} from '../../../Common/codeMirror';
import { tableCellFieldTheme } from './tableCellFieldTheme';
import type { TableCellFieldHandlers } from './tableCellField';

export function buildTableCellFieldExtensions(
    field: {
        destroyed: boolean;
        handlers: TableCellFieldHandlers | null;
        allowBlurExit: boolean;
        containsFocus: () => boolean;
    },
    hostView: EditorView,
    onHostFocusChange: (focused: boolean) => void,
): Extension[] {
    return [
        aquilumFieldSetup,
        aquilumCodeMirrorTheme,
        tableCellFieldTheme,
        aquilumSingleLineExtensions,
        EditorView.lineWrapping,
        EditorView.contentAttributes.of({
            'aria-label': t('editor.table.cellAria'),
            'aria-multiline': 'false',
            autocapitalize: 'off',
            autocomplete: 'off',
            spellcheck: 'false',
        }),
        EditorView.focusChangeEffect.of((_state, focusing) => {
            queueMicrotask(() => {
                if (!field.destroyed) onHostFocusChange(focusing);
            });
            return null;
        }),
        Prec.highest(keymap.of([
            {
                key: 'Tab',
                run: () => {
                    field.handlers?.onNavigate('next');
                    return true;
                },
                shift: () => {
                    field.handlers?.onNavigate('prev');
                    return true;
                },
            },
            {
                key: 'Enter',
                run: () => {
                    field.handlers?.onNavigate('down');
                    return true;
                },
            },
            {
                key: 'Escape',
                run: () => {
                    field.handlers?.onNavigate('escape');
                    return true;
                },
            },
        ])),
        Prec.highest(EditorView.domEventHandlers({
            keydown(event) {
                if (
                    event.key === 'Tab'
                    || event.key === 'Enter'
                    || event.key === 'Escape'
                ) {
                    event.stopPropagation();
                }
                return false;
            },
            blur() {
                queueMicrotask(() => {
                    if (field.destroyed || !field.allowBlurExit) return;
                    if (field.containsFocus()) return;
                    field.handlers?.onBlur();
                });
                return false;
            },
        })),
        EditorView.updateListener.of((update) => {
            if (update.docChanged || update.geometryChanged) {
                queueMicrotask(() => {
                    if (!field.destroyed) hostView.requestMeasure();
                });
            }
        }),
    ];
}
