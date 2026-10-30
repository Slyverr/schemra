export function isInitialWorkspaceViewReady(
	initialized: boolean,
	hasActiveProject: boolean,
	initialCompilationSettled: boolean
): boolean {
	return initialized && (!hasActiveProject || initialCompilationSettled);
}
