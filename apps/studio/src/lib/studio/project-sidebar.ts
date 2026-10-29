export type SidebarScenario = {
	id: string;
	fileId: string;
	title: string;
	dirty: boolean;
	hasErrors: boolean;
};

export type SidebarVisual = {
	id: string;
	fileId: string;
	path: string;
	title: string;
	pathHint: string | null;
	dirty: boolean;
	scenarios: SidebarScenario[];
};

export type SidebarResource = {
	fileId: string;
	resourceId: string | null;
	kind: 'component' | 'scenario' | 'source';
	title: string;
	path: string;
	dirty: boolean;
	hasErrors: boolean;
};

export type SidebarDocumentInput = {
	id: string;
	path: string;
	source: string;
	savedSource: string;
};

export type SidebarCompilationInput = {
	id: string;
	path: string;
	kind: string;
	visual_id: string | null;
	diagnostics: unknown[];
	declarations: Array<{ kind: string; id: string | null; title: string }>;
};

export type ProjectSidebarModel = {
	visuals: SidebarVisual[];
	primaryVisuals: SidebarVisual[];
	exampleVisuals: SidebarVisual[];
	otherSources: SidebarResource[];
};

export function buildProjectSidebarModel(
	documents: readonly SidebarDocumentInput[],
	compilation: readonly SidebarCompilationInput[]
): ProjectSidebarModel {
	const compiledById = new Map(compilation.map((file) => [file.id, file]));
	const allVisuals = documents
		.filter((document) => compiledById.get(document.id)?.kind === 'visual' || document.path.startsWith('visuals/'))
		.map((document): SidebarVisual => {
			const compiled = compiledById.get(document.id);
			const declaration = compiled?.declarations.find((item) => item.kind === 'visual');
			const id = compiled?.visual_id ?? declaration?.id ?? document.id;
			const scenarios = compilation.flatMap((scenarioFile) => {
				if (scenarioFile.kind !== 'scenario' || scenarioFile.visual_id !== id) return [];
				const sourceDocument = documents.find(({ id: fileId }) => fileId === scenarioFile.id);
				return scenarioFile.declarations
					.filter((item) => item.kind === 'scenario' && item.id !== null)
					.map((item) => ({
						id: item.id as string,
						fileId: scenarioFile.id,
						title: item.title || sourceLabel(scenarioFile.path),
						dirty: Boolean(sourceDocument && sourceDocument.source !== sourceDocument.savedSource),
						hasErrors: scenarioFile.diagnostics.some(isErrorDiagnostic)
					}));
			});
			return {
				id,
				fileId: document.id,
				path: document.path,
				title: declaration?.title || sourceLabel(document.path),
				pathHint: null,
				dirty: document.source !== document.savedSource,
				scenarios
			};
		});
	for (const visual of allVisuals) {
		visual.pathHint = duplicateVisualPathHint(allVisuals, visual);
	}
	const exampleVisuals = allVisuals.filter(({ path }) => path.startsWith('test-suite/visuals/focused/'));
	const exampleIds = new Set(exampleVisuals.map(({ id }) => id));
	const primaryVisuals = allVisuals.filter(({ id }) => !exampleIds.has(id));
	const visualIds = new Set(allVisuals.map(({ id }) => id));
	const visualFileIds = new Set(allVisuals.map(({ fileId }) => fileId));
	const linkedScenarioFiles = new Set(compilation
		.filter((file) => file.kind === 'scenario' && file.visual_id !== null && visualIds.has(file.visual_id))
		.map(({ id }) => id));
	const otherSources = documents
		.filter((document) => !visualFileIds.has(document.id) && !linkedScenarioFiles.has(document.id))
		.map((document) => resourceFromDocument(document, compiledById.get(document.id)));

	return { visuals: allVisuals, primaryVisuals, exampleVisuals, otherSources };
}

export function sourceLabel(path: string) {
	return path.split('/').at(-1)?.replace(/\.schemra$/i, '') ?? path;
}

function resourceFromDocument(document: SidebarDocumentInput, compiled?: SidebarCompilationInput): SidebarResource {
	const kind = compiled?.kind === 'component' || document.path.startsWith('components/')
		? 'component'
		: compiled?.kind === 'scenario' || document.path.startsWith('scenarios/')
			? 'scenario'
			: 'source';
	const scenarioTitle = kind === 'scenario'
		? compiled?.declarations.find((item) => item.kind === 'scenario')?.title
		: null;
	const scenarioId = kind === 'scenario'
		? compiled?.declarations.find((item) => item.kind === 'scenario')?.id ?? null
		: null;
	return {
		fileId: document.id,
		resourceId: scenarioId,
		kind,
		title: scenarioTitle || document.path.split('/').at(-1) || document.path,
		path: document.path,
		dirty: document.source !== document.savedSource,
		hasErrors: compiled?.diagnostics.some(isErrorDiagnostic) ?? false
	};
}

function duplicateVisualPathHint(visuals: readonly SidebarVisual[], target: SidebarVisual): string | null {
	const duplicates = visuals.filter(({ title }) => title === target.title);
	if (duplicates.length < 2) return null;
	const path = target.path.replace(/\.schemra$/i, '');
	const segments = path.split('/');
	const otherPaths = duplicates.filter(({ id }) => id !== target.id).map(({ path: other }) => other.replace(/\.schemra$/i, ''));
	for (let count = 1; count <= segments.length; count += 1) {
		const hint = segments.slice(-count).join('/');
		if (otherPaths.every((other) => other !== hint && !other.endsWith(`/${hint}`))) return hint;
	}
	return path;
}

function isErrorDiagnostic(value: unknown) {
	return typeof value === 'object' && value !== null && 'severity' in value && value.severity === 'error';
}
