export interface StoredJson {
	found: boolean;
	value?: unknown;
}

/** Read one local preference record without making browser storage a requirement. */
export function readStoredJson(key: string): StoredJson {
	try {
		const serialized = globalThis.localStorage.getItem(key);
		if (serialized === null) return { found: false };
		return { found: true, value: JSON.parse(serialized) as unknown };
	} catch {
		return { found: false };
	}
}

/** Persist a JSON-compatible preference value; return false when storage is unavailable. */
export function writeStoredJson(key: string, value: unknown): boolean {
	try {
		const serialized = JSON.stringify(value);
		if (typeof serialized !== 'string') return false;
		globalThis.localStorage.setItem(key, serialized);
		return true;
	} catch {
		return false;
	}
}
