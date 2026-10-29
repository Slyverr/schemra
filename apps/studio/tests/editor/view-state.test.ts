import { describe, expect, it } from 'bun:test';
import {
	editorDocumentKey,
	rememberEditorViewState,
	restoreEditorViewState,
	type EditorViewState
} from '../../src/lib/editor/view-state.js';

describe('editor document view state', () => {
	it('keeps selection and both scroll positions separate per project document', () => {
		const states = new Map<string, EditorViewState>();
		const first = editorDocumentKey('project-a', 'file-1');
		const second = editorDocumentKey('project-b', 'file-1');
		rememberEditorViewState(states, first, {
			selection: { start: 7, end: 10, direction: 'forward' },
			scrollLeft: 24,
			scrollTop: 360
		});
		rememberEditorViewState(states, second, {
			selection: { start: 2, end: 2, direction: 'none' },
			scrollLeft: 0,
			scrollTop: 48
		});

		expect(restoreEditorViewState(states, first, 100)).toEqual({
			selection: { start: 7, end: 10, direction: 'forward' },
			scrollLeft: 24,
			scrollTop: 360
		});
		expect(restoreEditorViewState(states, second, 100).scrollTop).toBe(48);
	});

	it('clamps saved selection to the new document length and normalizes invalid scroll values', () => {
		const states = new Map<string, EditorViewState>();
		rememberEditorViewState(states, 'document', {
			selection: { start: 15, end: 21, direction: 'backward' },
			scrollLeft: -1,
			scrollTop: Number.NaN
		});

		expect(restoreEditorViewState(states, 'document', 12)).toEqual({
			selection: { start: 12, end: 12, direction: 'backward' },
			scrollLeft: 0,
			scrollTop: 0
		});
	});

	it('starts new documents at the beginning', () => {
		expect(restoreEditorViewState(new Map(), 'new', 20)).toEqual({
			selection: { start: 0, end: 0, direction: 'none' },
			scrollLeft: 0,
			scrollTop: 0
		});
	});

	it('does not collide when project and document identifiers contain separators', () => {
		expect(editorDocumentKey('project:one', 'file')).not.toBe(editorDocumentKey('project', 'one:file'));
	});
});
