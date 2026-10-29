export type ResourceTabKind = 'visual' | 'scenario' | 'source';

export interface ResourceTabMenuState {
	kind: ResourceTabKind;
	declarationCount: number;
	projectFileCount: number;
	initialized: boolean;
	compiling: boolean;
	projectValid: boolean;
}

export interface ResourceTabMenuPolicy {
	showRename: boolean;
	renameDisabled: boolean;
	removeDisabled: boolean;
	removesDeclaration: boolean;
}

/** Keep tab actions consistent with the resource rules enforced by the Studio. */
export function resourceTabMenuPolicy(state: ResourceTabMenuState): ResourceTabMenuPolicy {
	const canEdit = state.initialized && !state.compiling && state.projectValid;
	const removesDeclaration = state.kind === 'scenario' && state.declarationCount > 1;
	const removesWholeFile = state.kind !== 'scenario' || state.declarationCount <= 1;

	return {
		showRename: state.kind !== 'source' && state.declarationCount > 0,
		renameDisabled: !canEdit,
		removeDisabled: !canEdit || (removesWholeFile && state.projectFileCount <= 1),
		removesDeclaration
	};
}
