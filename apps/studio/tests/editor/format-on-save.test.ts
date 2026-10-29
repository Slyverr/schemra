import { describe, expect, it } from 'bun:test';
import { formatSourceForSave } from '../../src/lib/editor/format-on-save.js';

function deferred<T>() {
	let resolve!: (value: T) => void;
	return { promise: new Promise<T>((done) => { resolve = done; }), resolve };
}

describe('format source on save', () => {
	it('returns the current text without invoking the formatter when formatting is disabled', async () => {
		let source = 'latest';
		let calls = 0;
		const result = await formatSourceForSave(() => source, false, async (value) => {
			calls += 1;
			return value;
		});

		expect(result).toBe('latest');
		expect(calls).toBe(0);
	});

	it('formats a stable buffer', async () => {
		await expect(formatSourceForSave(() => 'unformatted', true, async () => 'formatted')).resolves.toBe('formatted');
	});

	it('retries after edits during formatting and uses the latest stable snapshot', async () => {
		let source = 'before';
		const first = deferred<string>();
		let call = 0;
		const pending = formatSourceForSave(() => source, true, async (value) => {
			call += 1;
			if (call === 1) return first.promise;
			return `${value}!`;
		});

		source = 'after edit';
		first.resolve('stale formatted');

		expect(await pending).toBe('after edit!');
		expect(call).toBe(2);
	});

	it('saves the latest text if the buffer keeps changing', async () => {
		let source = 'one';
		let call = 0;
		const pending = formatSourceForSave(() => source, true, async (value) => {
			call += 1;
			source = `edit-${call}`;
			return `${value}-formatted`;
		});

		expect(await pending).toBe('edit-3');
		expect(call).toBe(3);
	});

	it('saves current text if formatting fails', async () => {
		let source = 'current';
		await expect(formatSourceForSave(() => source, true, async () => {
			source = 'newer';
			throw new Error('formatter unavailable');
		})).resolves.toBe('newer');
	});
});
