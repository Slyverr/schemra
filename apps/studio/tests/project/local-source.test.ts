import { describe, expect, it } from 'bun:test';
import {
	LocalSourceFileError,
	openLocalSchemraSource,
	projectPathForLocalSource,
	readSchemraUpload,
	saveLocalSchemraSource,
	type LocalSourceFilePlatform
} from '../../src/lib/project/local-source.ts';
import type { SchemraFileHandle } from '../../src/lib/project/session.ts';

function file(name = 'traffic.schemra', source = 'visual traffic {}'): File {
	return new File([source], name, { type: 'text/plain' });
}

function makeHandle(name = 'traffic.schemra', content = 'visual traffic {}') {
	const events: string[] = [];
	const handle: SchemraFileHandle = {
		name,
		async getFile() { events.push('getFile'); return file(name, content); },
		async createWritable() {
			events.push('createWritable');
			return {
				async write(value) { events.push(`write:${value}`); },
				async close() { events.push('close'); },
				async abort() { events.push('abort'); }
			};
		},
		async queryPermission() { events.push('query'); return 'granted'; },
		async requestPermission() { events.push('request'); return 'granted'; }
	};
	return { handle, events };
}

describe('local Schemra source files', () => {
	it('opens through the native picker and retains the handle, text, and filename', async () => {
		const { handle, events } = makeHandle('traffic-overview.schemra', 'visual traffic_overview {}');
		let pickerOptions: unknown;
		const platform: LocalSourceFilePlatform = {
			async showOpenFilePicker(options) { pickerOptions = options; return [handle]; }
		};

		const result = await openLocalSchemraSource(platform);

		expect(result).toEqual({
			status: 'opened',
			document: {
				name: 'traffic-overview.schemra',
				source: 'visual traffic_overview {}',
				handle,
				mode: 'local'
			}
		});
		expect(pickerOptions).toMatchObject({ multiple: false, types: [{ accept: { 'text/plain': ['.schemra'] } }] });
		expect(events).toEqual(['getFile']);
	});

	it('reports native picker cancellation without turning it into an error', async () => {
		const platform: LocalSourceFilePlatform = {
			async showOpenFilePicker() { throw new DOMException('Dismissed', 'AbortError'); }
		};

		expect(await openLocalSchemraSource(platform)).toEqual({ status: 'cancelled' });
	});

	it('wraps native picker failures without confusing them with cancellation', async () => {
		const cause = new Error('picker unavailable');
		const platform: LocalSourceFilePlatform = {
			async showOpenFilePicker() { throw cause; }
		};

		await expect(openLocalSchemraSource(platform)).rejects.toMatchObject({
			name: 'LocalSourceFileError', operation: 'open', cause
		});
	});

	it('asks the caller to use the upload fallback if the native picker is unavailable', async () => {
		expect(await openLocalSchemraSource({})).toEqual({ status: 'upload-required' });
		expect(await readSchemraUpload(null)).toEqual({ status: 'cancelled' });
	});

	it('reads uploaded text without a persistent handle and preserves its name', async () => {
		const source = 'scenario service_flow {\r\n  wait 10ms\r\n}';
		const result = await readSchemraUpload(file(' Service.schemra', source));
		expect(result).toEqual({
			status: 'opened',
			document: { name: ' Service.schemra', source, handle: null, mode: 'download' }
		});
	});

	it('classifies opened paths from the declaration header', () => {
		expect(projectPathForLocalSource('main.schemra', '// notes\n\nvisual traffic {}')).toBe('visuals/main.schemra');
		expect(projectPathForLocalSource('flow.schemra', 'scenario requests for visual traffic {}')).toBe('scenarios/flow.schemra');
		expect(projectPathForLocalSource('fragment.schemra', 'node broken {}')).toBe('sources/fragment.schemra');
	});

	it('wraps failures while reading a picked file', async () => {
		const { handle } = makeHandle();
		handle.getFile = async () => { throw new Error('read failed'); };
		const platform: LocalSourceFilePlatform = {
			async showOpenFilePicker() { return [handle]; }
		};

		await expect(openLocalSchemraSource(platform)).rejects.toMatchObject({
			name: 'LocalSourceFileError', operation: 'read', cause: new Error('read failed')
		});
	});

	it('rejects non-Schemra files and source files above the configured limit', async () => {
		await expect(readSchemraUpload(file('diagram.txt'))).rejects.toMatchObject({
			name: 'LocalSourceFileError', operation: 'read'
		});
		const oversized = {
			name: 'large.schemra',
			size: 2 * 1024 * 1024 + 1,
			text: async () => ''
		} as File;
		await expect(readSchemraUpload(oversized)).rejects.toThrow('2 MiB per-file limit');
	});

	it('writes to an existing handle after checking permission and closes the write', async () => {
		const { handle, events } = makeHandle();
		const result = await saveLocalSchemraSource({ name: 'ignored.schemra', source: 'visual main {}', handle }, {});

		expect(result).toEqual({ status: 'saved', name: handle.name, handle, mode: 'local' });
		expect(events).toEqual(['query', 'createWritable', 'write:visual main {}', 'close']);
	});

	it('requests permission when needed and reports denial without touching the file', async () => {
		const { handle, events } = makeHandle();
		handle.queryPermission = async () => { events.push('query-prompt'); return 'prompt'; };
		handle.requestPermission = async () => { events.push('request-denied'); return 'denied'; };

		await expect(saveLocalSchemraSource({ name: handle.name, source: 'visual main {}', handle }, {})).rejects.toMatchObject({
			name: 'LocalSourceFileError', operation: 'save', message: `Write access to ${handle.name} was not granted.`
		});
		expect(events).toEqual(['query-prompt', 'request-denied']);
	});

	it('does not attempt a write if permission remains pending and cannot be requested', async () => {
		const { handle, events } = makeHandle();
		handle.queryPermission = async () => { events.push('query-prompt'); return 'prompt'; };
		handle.requestPermission = undefined;

		await expect(saveLocalSchemraSource({ name: handle.name, source: '', handle }, {})).rejects.toMatchObject({
			name: 'LocalSourceFileError', operation: 'save', message: `Write access to ${handle.name} was not granted.`
		});
		expect(events).toEqual(['query-prompt']);
	});

	it('uses Save As when supported and returns the chosen handle', async () => {
		const { handle, events } = makeHandle('renamed.schemra');
		let suggestedName = '';
		const platform: LocalSourceFilePlatform = {
			async showSaveFilePicker(options) { suggestedName = options.suggestedName; return handle; }
		};

		const result = await saveLocalSchemraSource({ name: 'source.schemra', source: 'visual main {}' }, platform);

		expect(suggestedName).toBe('source.schemra');
		expect(result).toEqual({ status: 'saved', name: 'renamed.schemra', handle, mode: 'local' });
		expect(events).toEqual(['query', 'createWritable', 'write:visual main {}', 'close']);
	});

	it('does not start a download when Save As is cancelled', async () => {
		let downloaded = false;
		const platform: LocalSourceFilePlatform = {
			async showSaveFilePicker() { throw new DOMException('Dismissed', 'AbortError'); },
			download() { downloaded = true; }
		};

		expect(await saveLocalSchemraSource({ name: 'main.schemra', source: '' }, platform)).toEqual({ status: 'cancelled' });
		expect(downloaded).toBe(false);
	});

	it('downloads when no file handle or native Save As picker exists', async () => {
		const downloads: Array<[string, string]> = [];
		const platform: LocalSourceFilePlatform = {
			download(name, source) { downloads.push([name, source]); }
		};

		const result = await saveLocalSchemraSource({ name: 'main.schemra', source: 'visual main {}' }, platform);

		expect(result).toEqual({ status: 'saved', name: 'main.schemra', handle: null, mode: 'download' });
		expect(downloads).toEqual([['main.schemra', 'visual main {}']]);
	});

	it('aborts a failed file write and keeps the write error visible', async () => {
		const { handle, events } = makeHandle();
		handle.createWritable = async () => ({
			async write() { throw new Error('disk is read-only'); },
			async close() { events.push('close'); },
			async abort() { events.push('abort'); }
		});

		await expect(saveLocalSchemraSource({ name: handle.name, source: 'visual main {}', handle }, {})).rejects.toMatchObject({
			name: 'LocalSourceFileError', operation: 'save', cause: new Error('disk is read-only')
		});
		expect(events).toEqual(['query', 'abort']);
	});

	it('rejects invalid output names, oversized saves, and missing download capability', async () => {
		await expect(saveLocalSchemraSource({ name: '../escape.schemra', source: '' }, {})).rejects.toBeInstanceOf(LocalSourceFileError);
		await expect(saveLocalSchemraSource({ name: 'large.schemra', source: 'x'.repeat(2 * 1024 * 1024 + 1) }, {})).rejects.toThrow('2 MiB');
		await expect(saveLocalSchemraSource({ name: 'main.schemra', source: '' }, {})).rejects.toThrow('cannot save a local Schemra file');
	});
});
