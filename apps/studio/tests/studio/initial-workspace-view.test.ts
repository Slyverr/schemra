import { describe, expect, test } from 'bun:test';
import { isInitialWorkspaceViewReady } from '../../src/lib/studio/initial-workspace-view.js';

describe('initial workspace view readiness', () => {
	test('keeps the loading view while the workspace restores', () => {
		expect(isInitialWorkspaceViewReady(false, true, false)).toBe(false);
	});

	test('waits for the first project compilation before showing the editor', () => {
		expect(isInitialWorkspaceViewReady(true, true, false)).toBe(false);
	});

	test('shows an initialized project after its first compilation settles', () => {
		expect(isInitialWorkspaceViewReady(true, true, true)).toBe(true);
	});

	test('shows an initialized empty workspace without waiting for compilation', () => {
		expect(isInitialWorkspaceViewReady(true, false, false)).toBe(true);
	});
});
