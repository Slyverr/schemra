import { expect, test } from 'bun:test';
import { studioTitle } from '../src/lib/studio-title.ts';

test('the starter page title is Schemra', () => {
	expect(studioTitle).toBe('Schemra');
});
