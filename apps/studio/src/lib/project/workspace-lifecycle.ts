import type { WorkspaceControllerState } from './workspace-controller.js';

/** Whether leaving now could abandon an unfinished workspace or source-file save. */
export function shouldWarnBeforeUnload(state: WorkspaceControllerState): boolean {
	if (!state.initialized || ['idle', 'loading', 'dirty', 'saving', 'error', 'conflict'].includes(state.saveStatus)) {
		return true;
	}

	return state.projects.some((project) => project.files.some((file) => file.source !== file.savedSource));
}
