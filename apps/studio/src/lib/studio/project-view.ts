import type { ProjectCompileResult, ProjectSource } from '../schemra.js';

export function makeSourcesKey(sources: readonly ProjectSource[]): string {
	return JSON.stringify(sources.map(({ id, path, source }) => [id, path, source]));
}

export function compareSourceFiles(left: { path: string }, right: { path: string }): number {
	return sourceRank(left.path) - sourceRank(right.path) || left.path.localeCompare(right.path);
}

export function isErrorDiagnostic(value: unknown): boolean {
	return typeof value === 'object' && value !== null && 'severity' in value && value.severity === 'error';
}

export function stabilizeFileIdentity<T extends {
	id: string;
	kind: string;
	visual_id: string | null;
	declarations: unknown[];
	diagnostics: unknown[];
}>(current: readonly T[], lastValid: readonly T[], documentIds: readonly string[]): T[] {
	const currentById = new Map(current.map((file) => [file.id, file]));
	const lastValidById = new Map(lastValid.map((file) => [file.id, file]));
	return documentIds.flatMap((id) => {
		const currentFile = currentById.get(id);
		const previousFile = lastValidById.get(id);
		if (!currentFile) return previousFile ? [previousFile] : [];
		if (!previousFile || !currentFile.diagnostics.some(isErrorDiagnostic)) return [currentFile];
		return [{
			...currentFile,
			kind: previousFile.kind,
			visual_id: previousFile.visual_id,
			declarations: previousFile.declarations
		}];
	});
}

export function isValidProjectCompilation(result: ProjectCompileResult): boolean {
	if (result.error || result.visuals.length === 0) return false;
	if (result.files.some((file) => file.diagnostics.some(isErrorDiagnostic))) return false;
	return result.visuals.some(({ output }) => Boolean(output.svg && output.scene));
}

function sourceRank(path: string): number {
	if (path.startsWith('visuals/')) return 0;
	if (path.startsWith('scenarios/')) return 1;
	return 2;
}
