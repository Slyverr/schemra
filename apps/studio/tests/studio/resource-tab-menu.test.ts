import { describe, expect, it } from 'bun:test';
import { resourceTabMenuPolicy } from '../../src/lib/studio/resource-tab-menu.ts';

describe('resource tab context menu policy', () => {
	const readyProject = {
		initialized: true,
		compiling: false,
		projectValid: true
	};

	it('offers rename only for declared visuals and scenarios', () => {
		expect(resourceTabMenuPolicy({ ...readyProject, kind: 'visual', declarationCount: 1, projectFileCount: 2 }))
			.toEqual({ showRename: true, renameDisabled: false, removeDisabled: false, removesDeclaration: false });
		expect(resourceTabMenuPolicy({ ...readyProject, kind: 'scenario', declarationCount: 1, projectFileCount: 2 }))
			.toEqual({ showRename: true, renameDisabled: false, removeDisabled: false, removesDeclaration: false });
		expect(resourceTabMenuPolicy({ ...readyProject, kind: 'source', declarationCount: 0, projectFileCount: 2 }))
			.toEqual({ showRename: false, renameDisabled: false, removeDisabled: false, removesDeclaration: false });
	});

	it('removes one scenario declaration without deleting its source file', () => {
		expect(resourceTabMenuPolicy({ ...readyProject, kind: 'scenario', declarationCount: 2, projectFileCount: 1 }))
			.toEqual({ showRename: true, renameDisabled: false, removeDisabled: false, removesDeclaration: true });
	});

	it('protects the last project file and disables mutations while unavailable', () => {
		expect(resourceTabMenuPolicy({ ...readyProject, kind: 'visual', declarationCount: 1, projectFileCount: 1 }))
			.toEqual({ showRename: true, renameDisabled: false, removeDisabled: true, removesDeclaration: false });
		expect(resourceTabMenuPolicy({ ...readyProject, kind: 'scenario', declarationCount: 1, projectFileCount: 1 }))
			.toEqual({ showRename: true, renameDisabled: false, removeDisabled: true, removesDeclaration: false });
		expect(resourceTabMenuPolicy({ ...readyProject, compiling: true, kind: 'visual', declarationCount: 1, projectFileCount: 2 }))
			.toEqual({ showRename: true, renameDisabled: true, removeDisabled: true, removesDeclaration: false });
		expect(resourceTabMenuPolicy({ ...readyProject, projectValid: false, kind: 'visual', declarationCount: 1, projectFileCount: 2 }))
			.toEqual({ showRename: true, renameDisabled: true, removeDisabled: true, removesDeclaration: false });
		expect(resourceTabMenuPolicy({ ...readyProject, initialized: false, kind: 'source', declarationCount: 0, projectFileCount: 2 }))
			.toEqual({ showRename: false, renameDisabled: true, removeDisabled: true, removesDeclaration: false });
	});
});
