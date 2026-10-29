import { MAX_SOURCE_FILE_BYTES, type SchemraFileHandle } from './session.js';

export interface LocalSchemraSource {
	name: string;
	source: string;
	handle: SchemraFileHandle | null;
	mode: 'local' | 'download';
}

export type OpenLocalSourceResult =
	| { status: 'opened'; document: LocalSchemraSource }
	| { status: 'cancelled' }
	| { status: 'upload-required' };

export type SaveLocalSourceResult =
	| { status: 'saved'; name: string; handle: SchemraFileHandle | null; mode: 'local' | 'download' }
	| { status: 'cancelled' };

export interface LocalSourceFilePlatform {
	showOpenFilePicker?: (options: { multiple: false; types: SourceFileType[] }) => Promise<SchemraFileHandle[]>;
	showSaveFilePicker?: (options: { suggestedName: string; types: SourceFileType[] }) => Promise<SchemraFileHandle>;
	download?: (name: string, source: string) => void;
}

interface SourceFileType {
	description: string;
	accept: Record<string, string[]>;
}

export interface SaveLocalSourceOptions {
	name: string;
	source: string;
	handle?: SchemraFileHandle | null;
}

/** Place an opened source under its declared resource category. */
export function projectPathForLocalSource(name: string, source: string): string {
	const fileName = validateSourceName(name, 'read');
	const declaration = readHeaderWords(source);
	const folder = declaration?.[0] === 'visual' ? 'visuals' : declaration?.[0] === 'scenario' ? 'scenarios' : 'sources';
	return `${folder}/${fileName}`;
}

export class LocalSourceFileError extends Error {
	readonly operation: 'open' | 'read' | 'save';

	constructor(operation: LocalSourceFileError['operation'], message: string, options?: ErrorOptions) {
		super(message, options);
		this.name = 'LocalSourceFileError';
		this.operation = operation;
	}
}

const SCHEMRA_FILE_TYPES: SourceFileType[] = [{
	description: 'Schemra source',
	accept: { 'text/plain': ['.schemra'] }
}];

/** Open a source file using the native picker when available. Unsupported browsers can use the upload input. */
export async function openLocalSchemraSource(
	platform: LocalSourceFilePlatform = browserFilePlatform()
): Promise<OpenLocalSourceResult> {
	if (!platform.showOpenFilePicker) return { status: 'upload-required' };

	let handles: SchemraFileHandle[];
	try {
		handles = await platform.showOpenFilePicker({ multiple: false, types: SCHEMRA_FILE_TYPES });
	} catch (error) {
		if (isAbortError(error)) return { status: 'cancelled' };
		throw fileError('open', 'The selected Schemra file could not be opened.', error);
	}

	const handle = handles[0];
	if (!handle) return { status: 'cancelled' };
	let file: File;
	try {
		file = await handle.getFile();
	} catch (error) {
		throw fileError('read', `Could not read ${handle.name}.`, error);
	}
	return { status: 'opened', document: await readSourceFile(file, handle, 'local') };
}

/** Read a file-input upload. Passing null represents the user cancelling the picker. */
export async function readSchemraUpload(file: File | null | undefined): Promise<OpenLocalSourceResult> {
	if (!file) return { status: 'cancelled' };
	return { status: 'opened', document: await readSourceFile(file, null, 'download') };
}

/** Save to an existing file handle, open Save As when supported, or download a .schemra file. */
export async function saveLocalSchemraSource(
	options: SaveLocalSourceOptions,
	platform: LocalSourceFilePlatform = browserFilePlatform()
): Promise<SaveLocalSourceResult> {
	const name = validateSourceName(options.name, 'save');
	validateSourceSize(options.source, 'save');
	let handle = options.handle ?? null;

	if (!handle && platform.showSaveFilePicker) {
		try {
			handle = await platform.showSaveFilePicker({
				suggestedName: name,
				types: SCHEMRA_FILE_TYPES
			});
		} catch (error) {
			if (isAbortError(error)) return { status: 'cancelled' };
			throw fileError('save', `Could not choose a destination for ${name}.`, error);
		}
	}

	if (handle) {
		await writeToHandle(handle, options.source);
		return { status: 'saved', name: validateSourceName(handle.name, 'save'), handle, mode: 'local' };
	}

	if (!platform.download) {
		throw new LocalSourceFileError('save', 'This browser cannot save a local Schemra file or start a download.');
	}
	try {
		platform.download(name, options.source);
	} catch (error) {
		throw fileError('save', `Could not download ${name}.`, error);
	}
	return { status: 'saved', name, handle: null, mode: 'download' };
}

async function readSourceFile(file: File, handle: SchemraFileHandle | null, mode: LocalSchemraSource['mode']): Promise<LocalSchemraSource> {
	const name = validateSourceName(file.name, 'read');
	if (file.size > MAX_SOURCE_FILE_BYTES) {
		throw new LocalSourceFileError('read', `${name} exceeds the ${MAX_SOURCE_FILE_BYTES / 1024 / 1024} MiB per-file limit.`);
	}
	try {
		const source = await file.text();
		validateSourceSize(source, 'read');
		return { name, source, handle, mode };
	} catch (error) {
		if (error instanceof LocalSourceFileError) throw error;
		throw fileError('read', `Could not read ${name}.`, error);
	}
}

async function writeToHandle(handle: SchemraFileHandle, source: string): Promise<void> {
	if (handle.queryPermission) {
		let permission: PermissionState;
		try {
			permission = await handle.queryPermission({ mode: 'readwrite' });
			if (permission !== 'granted' && handle.requestPermission) {
				permission = await handle.requestPermission({ mode: 'readwrite' });
			}
		} catch (error) {
			throw fileError('save', `Could not check write access for ${handle.name}.`, error);
		}
		if (permission !== 'granted') {
			throw new LocalSourceFileError('save', `Write access to ${handle.name} was not granted.`);
		}
	}

	let writable: Awaited<ReturnType<SchemraFileHandle['createWritable']>> | undefined;
	try {
		writable = await handle.createWritable();
		await writable.write(source);
		await writable.close();
	} catch (error) {
		try {
			await writable?.abort?.();
		} catch {
			// Keep the write or close failure as the visible cause.
		}
		throw fileError('save', `Could not save ${handle.name}.`, error);
	}
}

function validateSourceName(name: string, operation: LocalSourceFileError['operation']): string {
	if (!name || /[\\/\u0000-\u001f\u007f]/.test(name) || !name.toLowerCase().endsWith('.schemra')) {
		throw new LocalSourceFileError(operation, 'Choose a file with a valid .schemra filename.');
	}
	return name;
}

function validateSourceSize(source: string, operation: LocalSourceFileError['operation']): void {
	const sourceBytes = new TextEncoder().encode(source).byteLength;
	if (sourceBytes > MAX_SOURCE_FILE_BYTES) {
		throw new LocalSourceFileError(operation, `A source file must be no larger than ${MAX_SOURCE_FILE_BYTES / 1024 / 1024} MiB.`);
	}
}

function readHeaderWords(source: string): [string, string] | null {
	let offset = source.charCodeAt(0) === 0xfeff ? 1 : 0;
	const readWord = (): string | null => {
		while (offset < source.length) {
			if (/\s/.test(source[offset]!)) {
				offset += 1;
				continue;
			}
			if (source.startsWith('//', offset)) {
				const lineEnd = source.indexOf('\n', offset + 2);
				offset = lineEnd === -1 ? source.length : lineEnd + 1;
				continue;
			}
			break;
		}
		const match = /^[A-Za-z_][A-Za-z0-9_]*/.exec(source.slice(offset));
		if (!match) return null;
		offset += match[0].length;
		return match[0];
	};
	const kind = readWord();
	if (kind !== 'visual' && kind !== 'scenario') return null;
	const id = readWord();
	return id ? [kind, id] : null;
}

function isAbortError(error: unknown): boolean {
	return typeof error === 'object' && error !== null && 'name' in error && error.name === 'AbortError';
}

function fileError(operation: LocalSourceFileError['operation'], message: string, cause: unknown): LocalSourceFileError {
	return new LocalSourceFileError(operation, message, cause instanceof Error ? { cause } : undefined);
}

function browserFilePlatform(): LocalSourceFilePlatform {
	const browser = globalThis as typeof globalThis & {
		showOpenFilePicker?: LocalSourceFilePlatform['showOpenFilePicker'];
		showSaveFilePicker?: LocalSourceFilePlatform['showSaveFilePicker'];
	};
	const platform: LocalSourceFilePlatform = {};
	if (browser.showOpenFilePicker) platform.showOpenFilePicker = browser.showOpenFilePicker.bind(browser);
	if (browser.showSaveFilePicker) platform.showSaveFilePicker = browser.showSaveFilePicker.bind(browser);
	if (typeof document !== 'undefined' && typeof URL !== 'undefined' && typeof URL.createObjectURL === 'function') {
		platform.download = (name, source) => {
			const url = URL.createObjectURL(new Blob([source], { type: 'text/plain;charset=utf-8' }));
			const link = document.createElement('a');
			link.href = url;
			link.download = name;
			link.click();
			window.setTimeout(() => URL.revokeObjectURL(url), 0);
		};
	}
	return platform;
}
