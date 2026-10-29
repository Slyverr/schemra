import { describe, expect, it } from 'bun:test';
import { shouldWarnBeforeUnload } from '../../src/lib/project/workspace-lifecycle.js';
import type { WorkspaceControllerState } from '../../src/lib/project/workspace-controller.js';

function state(overrides: Partial<WorkspaceControllerState> = {}): WorkspaceControllerState {
	return {
		projects: [],
		activeProjectId: 'project',
		selectedFileByProject: {},
		selectedScenarioByProject: {},
		initialized: true,
		saveStatus: 'saved',
		error: null,
		...overrides
	};
}

describe('workspace unload protection', () => {
	it('allows leaving after the workspace is saved and all sources are clean', () => {
		expect(shouldWarnBeforeUnload(state())).toBe(false);
	});

	it('warns while workspace persistence is pending or has failed', () => {
		for (const saveStatus of ['idle', 'loading', 'dirty', 'saving', 'error', 'conflict'] as const) {
			expect(shouldWarnBeforeUnload(state({ saveStatus }))).toBe(true);
		}
	});

	it('warns when a source file has edits even if the browser workspace is saved', () => {
		expect(shouldWarnBeforeUnload(state({
			projects: [{
				id: 'project',
				name: 'Example',
				files: [{
					id: 'file',
					path: 'visuals/example.schemra',
					source: 'edited',
					savedSource: 'original',
					handle: null,
					mode: 'memory'
				}]
			}]
		}))).toBe(true);
	});
});
