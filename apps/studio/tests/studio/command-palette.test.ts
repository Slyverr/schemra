import { describe, expect, it } from 'bun:test';
import {
	buildCommandPaletteEntries,
	commandPaletteFileKind,
	commandPaletteFileName,
	firstCommandPaletteSelection,
	searchCommandPalette,
	type CommandPaletteContext
} from '../../src/lib/studio/command-palette.ts';

const projects: CommandPaletteContext['projects'] = [
	{
		id: 'project-demo',
		name: 'Café network',
		files: [
			{ id: 'visual-main', path: 'visuals/network.schemra', source: 'visual network {}', savedSource: 'visual network {}' },
			{ id: 'scenario-response', path: 'scenarios/request-response.schemra', source: 'scenario response {}', savedSource: 'scenario response {}' },
			{ id: 'source-notes', path: 'notes/notes.schemra', source: '// notes', savedSource: '// old notes' }
		]
	},
	{
		id: 'project-other',
		name: 'Payments',
		files: [
			{ id: 'visual-payments', path: 'visuals/payments.schemra', source: 'visual payments {}', savedSource: 'visual payments {}' }
		]
	}
];

describe('command palette entries', () => {
	it('lists open tabs first and avoids duplicating them among project files', () => {
		const entries = buildCommandPaletteEntries({
			projects,
			activeProjectId: 'project-demo',
			activeFileId: 'visual-main',
			openTabs: [
				{ projectId: 'project-demo', fileId: 'scenario-response' },
				{ projectId: 'project-demo', fileId: 'visual-main' },
				{ projectId: 'project-demo', fileId: 'scenario-response' },
				{ projectId: 'project-missing', fileId: 'missing' }
			],
			actions: [{ id: 'format', label: 'Format source' }]
		});

		expect(entries.slice(0, 2).map((entry) => entry.key)).toEqual([
			'tab:project-demo:scenario-response',
			'tab:project-demo:visual-main'
		]);
		expect(entries.filter((entry) => entry.type === 'file' && entry.fileId === 'scenario-response')).toHaveLength(1);
		expect(entries.filter((entry) => entry.type === 'file' && entry.group === 'projectFiles').map((entry) => entry.type === 'file' ? entry.fileId : '')).toEqual([
			'source-notes',
			'visual-payments'
		]);
	});

	it('preserves action icons, shortcuts, and searchable aliases', () => {
		const entries = buildCommandPaletteEntries({
			projects: [],
			activeProjectId: null,
			activeFileId: null,
			actions: [{
				id: 'format-source',
				label: 'Format source',
				description: 'Format the current document',
				keywords: ['prettify'],
				shortcut: 'Alt+Shift+F',
				icon: 'format'
			}]
		});

		expect(entries).toContainEqual(expect.objectContaining({
			key: 'action:format-source',
			icon: 'format',
			shortcut: 'Alt+Shift+F'
		}));
		expect(searchCommandPalette(entries, 'prettify')).toContainEqual(expect.objectContaining({
			key: 'action:format-source'
		}));
	});

	it('marks the selected tab and unsaved source while classifying visual resources', () => {
		const entries = buildCommandPaletteEntries({
			projects,
			activeProjectId: 'project-demo',
			activeFileId: 'source-notes',
			openTabs: [{ projectId: 'project-demo', fileId: 'source-notes' }]
		});
		const note = entries.find((entry) => entry.type === 'file' && entry.fileId === 'source-notes');
		const visual = entries.find((entry) => entry.type === 'file' && entry.fileId === 'visual-main');

		expect(note).toMatchObject({ kind: 'source', current: true, unsaved: true });
		expect(visual).toMatchObject({ kind: 'visual', current: false, unsaved: false });
	});

	it('searches tabs, project paths, resource kinds, projects, and named actions', () => {
		const entries = buildCommandPaletteEntries({
			projects,
			activeProjectId: 'project-demo',
			activeFileId: null,
			openTabs: [{ projectId: 'project-demo', fileId: 'scenario-response' }],
			actions: [{ id: 'format-source', label: 'Format source', keywords: ['prettify', 'document'] }]
		});

		expect(searchCommandPalette(entries, 'scenario request response').map((entry) => entry.key)).toEqual([
			'tab:project-demo:scenario-response'
		]);
		expect(searchCommandPalette(entries, 'cafe').some((entry) => entry.type === 'project' && entry.projectId === 'project-demo')).toBe(true);
		expect(searchCommandPalette(entries, 'prettify').map((entry) => entry.key)).toEqual(['action:format-source']);
		expect(searchCommandPalette(entries, 'unknown query')).toEqual([]);
	});

	it('infers declaration kind before path and strips only the Schemra extension', () => {
		expect(commandPaletteFileKind({ path: 'sources/custom.schemra', source: 'scenario x {}' })).toBe('scenario');
		expect(commandPaletteFileKind({ path: 'visuals/custom.schemra', source: '// unfinished' })).toBe('visual');
		expect(commandPaletteFileName('scenarios/request-response.SCHEMRA')).toBe('request-response');
	});

	it('chooses the first enabled result for the command primitive', () => {
		const entries = buildCommandPaletteEntries({
			projects: [],
			activeProjectId: null,
			activeFileId: null,
			actions: [
				{ id: 'first', label: 'First action', disabled: true },
				{ id: 'unavailable', label: 'Unavailable action', disabled: true },
				{ id: 'last', label: 'Last action' }
			]
		});

		expect(firstCommandPaletteSelection(entries)).toBe(2);
		expect(firstCommandPaletteSelection([])).toBe(-1);
	});
});
