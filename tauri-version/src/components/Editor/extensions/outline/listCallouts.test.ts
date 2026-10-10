import { Text } from '@codemirror/state';
import { describe, expect, it } from 'vitest';
import {
    buildListCalloutDecorations,
    matchListCallout,
} from './listCallouts';

describe('list callouts', () => {
    it('matches callout chars after a complete list prefix', () => {
        expect(matchListCallout('- ! note')).toEqual({
            charFrom: 2,
            charTo: 3,
            callout: { char: '!', tone: 'important' },
        });
        expect(matchListCallout('\t1) $ cash')).toMatchObject({
            charFrom: 4,
            callout: { char: '$', tone: 'success' },
        });
    });

    it('rejects incomplete or unknown markers', () => {
        expect(matchListCallout('- !')).toBeNull();
        expect(matchListCallout('- x note')).toBeNull();
        expect(matchListCallout('! note')).toBeNull();
        expect(matchListCallout('-')).toBeNull();
    });

    it('decorates the line and replaces the callout char away from caret', () => {
        const doc = Text.of(['- ! hello']);
        const ranges: Array<[number, number]> = [];
        buildListCalloutDecorations(doc, [{ from: 0, to: doc.length }], 5)
            .between(0, doc.length, (from, to) => {
                ranges.push([from, to]);
            });
        expect(ranges).toEqual([[0, 0], [2, 3]]);
    });

    it('keeps the raw callout char when caret touches it', () => {
        const doc = Text.of(['- ! hello']);
        const ranges: Array<[number, number]> = [];
        buildListCalloutDecorations(doc, [{ from: 0, to: doc.length }], 2)
            .between(0, doc.length, (from, to) => {
                ranges.push([from, to]);
            });
        expect(ranges).toEqual([[0, 0]]);
    });
});
