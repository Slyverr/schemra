export type SourceFormatter = (source: string) => Promise<string>;

const MAX_FORMAT_ATTEMPTS = 3;

/** Format the latest stable buffer, or save the latest text if editing never settles. */
export async function formatSourceForSave(
	readSource: () => string,
	formatOnSave: boolean,
	formatSource: SourceFormatter
): Promise<string> {
	if (!formatOnSave) return readSource();

	for (let attempt = 0; attempt < MAX_FORMAT_ATTEMPTS; attempt += 1) {
		const source = readSource();
		let formatted: string;
		try {
			formatted = await formatSource(source);
		} catch {
			return readSource();
		}
		const latest = readSource();
		if (latest === source) return formatted;
	}

	return readSource();
}
