<script lang="ts">
	import { onMount } from 'svelte';
	import { FileCode2, Palette } from '@lucide/svelte';
	import StudioThemePicker from './StudioThemePicker.svelte';
	import {
		EDITOR_ZOOM_LEVELS,
		type EditorIndentSize,
		type EditorPreferences,
		type EditorZoom
	} from './editor.js';
	import {
		editorThemeOptions,
		editorThemeStyles,
		resolveEditorTheme,
		resolveStudioTheme,
		studioThemeOptions,
		type ThemePreferences
	} from './appearance.js';
	import { studioMessage, type StudioMessageId } from '../i18n/messages.js';

	interface Props {
		themePreferences: ThemePreferences;
		editorPreferences: EditorPreferences;
		popoverBoundary?: Element | null;
		onThemePreferencesChange: (value: ThemePreferences) => void;
		onEditorPreferencesChange: (value: EditorPreferences) => void;
	}

	type BooleanEditorPreference =
		| 'lineNumbers'
		| 'indentGuides'
		| 'activeLineHighlight'
		| 'matchingBracketHighlight'
		| 'autoIndent'
		| 'autoClosePairs'
		| 'formatOnSave';

	let {
		themePreferences,
		editorPreferences,
		popoverBoundary,
		onThemePreferencesChange,
		onEditorPreferencesChange
	}: Props = $props();
	let systemPrefersDark = $state(false);
	let activeCategory = $state<'appearance' | 'editor'>('appearance');
	const resolvedStudioTheme = $derived(resolveStudioTheme(themePreferences.studio, systemPrefersDark));
	const activeStudioThemeLabel = $derived(
		studioMessage(studioThemeOptions.find(({ value }) => value === resolvedStudioTheme)?.labelKey ?? 'theme.schemraLight')
	);

	const booleanPreferences: Array<{
		key: BooleanEditorPreference;
		labelKey: StudioMessageId;
		descriptionKey: StudioMessageId;
	}> = [
		{ key: 'lineNumbers', labelKey: 'settings.lineNumbers', descriptionKey: 'settings.lineNumbersDescription' },
		{ key: 'indentGuides', labelKey: 'settings.blockScopeGuides', descriptionKey: 'settings.blockScopeGuidesDescription' },
		{ key: 'activeLineHighlight', labelKey: 'settings.currentLineHighlight', descriptionKey: 'settings.currentLineHighlightDescription' },
		{ key: 'matchingBracketHighlight', labelKey: 'settings.matchingBrackets', descriptionKey: 'settings.matchingBracketsDescription' },
		{ key: 'autoIndent', labelKey: 'settings.autoIndent', descriptionKey: 'settings.autoIndentDescription' },
		{ key: 'autoClosePairs', labelKey: 'settings.autoClosePairs', descriptionKey: 'settings.autoClosePairsDescription' },
		{ key: 'formatOnSave', labelKey: 'settings.formatOnSave', descriptionKey: 'settings.formatOnSaveDescription' }
	];

	onMount(() => {
		const preference = window.matchMedia('(prefers-color-scheme: dark)');
		const update = () => (systemPrefersDark = preference.matches);
		update();
		preference.addEventListener('change', update);
		return () => preference.removeEventListener('change', update);
	});

	function updateTheme<Key extends keyof ThemePreferences>(key: Key, value: ThemePreferences[Key]) {
		onThemePreferencesChange({ ...themePreferences, [key]: value });
	}

	function updateEditor<Key extends keyof EditorPreferences>(key: Key, value: EditorPreferences[Key]) {
		onEditorPreferencesChange({ ...editorPreferences, [key]: value });
	}

	function updateBooleanEditor(key: BooleanEditorPreference, value: boolean) {
		updateEditor(key, value);
	}

	function chooseZoom(value: string) {
		const zoom = Number(value);
		if (EDITOR_ZOOM_LEVELS.includes(zoom as EditorZoom)) updateEditor('zoom', zoom as EditorZoom);
	}

	function chooseIndentSize(value: string) {
		updateEditor('indentSize', Number(value) === 4 ? 4 satisfies EditorIndentSize : 2);
	}

</script>

<div class="settings-panel">
	<nav class="settings-categories" aria-label={studioMessage('settings.categories')}>
		<p>{studioMessage('settings.preferences')}</p>
		<button type="button" class:active={activeCategory === 'appearance'} aria-current={activeCategory === 'appearance' ? 'page' : undefined} onclick={() => (activeCategory = 'appearance')}>
			<Palette size={16} aria-hidden="true" /><span>{studioMessage('settings.appearance')}</span>
		</button>
		<button type="button" class:active={activeCategory === 'editor'} aria-current={activeCategory === 'editor' ? 'page' : undefined} onclick={() => (activeCategory = 'editor')}>
			<FileCode2 size={16} aria-hidden="true" /><span>{studioMessage('settings.editor')}</span>
		</button>
	</nav>
	<div class="settings-content">
	{#if activeCategory === 'appearance'}
	<section class="settings-section" aria-labelledby="appearance-heading">
		<StudioThemePicker
			theme={themePreferences.studio}
			systemThemeLabel={activeStudioThemeLabel}
			{popoverBoundary}
			onChange={(value) => updateTheme('studio', value)}
		/>

		<div class="section-heading">
			<h2 id="appearance-heading">{studioMessage('settings.appearanceOptions')}</h2>
		</div>

		<div class="appearance-options">
		<label class="preference-row" for="reduce-motion">
			<span class="preference-copy">
				<span class="preference-title">{studioMessage('settings.reduceMotion')}</span>
				<span class="preference-description">{studioMessage('settings.reduceMotionDescription')}</span>
			</span>
			<input
				id="reduce-motion"
				type="checkbox"
				class="preference-switch"
				checked={themePreferences.reduceMotion}
				onchange={(event) => updateTheme('reduceMotion', event.currentTarget.checked)}
			/>
		</label>
		</div>
	</section>
	{:else}

	<section class="settings-section" aria-labelledby="editor-heading">
		<div class="section-heading">
			<h2 id="editor-heading">{studioMessage('settings.syntaxColors')}</h2>
		</div>

		<fieldset class="theme-picker" aria-label={studioMessage('settings.editorSyntaxTheme')}>
			<div class="editor-theme-options">
				{#each editorThemeOptions as option (option.value)}
					<label class="editor-theme-option">
						<span class="theme-option-heading">
							<input
								type="radio"
								name="editor-theme"
								value={option.value}
								checked={themePreferences.editor === option.value}
								onchange={() => updateTheme('editor', option.value)}
							/>
							<span>{studioMessage(option.labelKey)}</span>
						</span>
						<span class="syntax-preview" style={editorThemeStyles(resolveEditorTheme(option.value, resolvedStudioTheme))} aria-hidden="true">
							<code><span class="syntax-keyword">move</span><span class="syntax-foreground"> request </span><span class="syntax-punctuation">-&gt;</span><span class="syntax-foreground"> export</span></code>
							<code><span class="syntax-property">label</span><span class="syntax-string"> "Saved" </span><span class="syntax-property">duration</span><span class="syntax-number"> 720</span></code>
						</span>
					</label>
				{/each}
			</div>
		</fieldset>

		<div class="section-heading">
			<h2>{studioMessage('settings.editorOptions')}</h2>
		</div>

		<div class="select-grid">
			<label class="setting-select" for="editor-zoom">
				<span class="setting-label">{studioMessage('settings.textSize')}</span>
				<select id="editor-zoom" value={editorPreferences.zoom} onchange={(event) => chooseZoom(event.currentTarget.value)}>
					{#each EDITOR_ZOOM_LEVELS as zoom (zoom)}
						<option value={zoom}>{zoom}%</option>
					{/each}
				</select>
			</label>
			<label class="setting-select" for="editor-indent-size">
				<span class="setting-label">{studioMessage('settings.indentSize')}</span>
				<select id="editor-indent-size" value={editorPreferences.indentSize} onchange={(event) => chooseIndentSize(event.currentTarget.value)}>
					<option value="2">{studioMessage('settings.spaces', { count: 2 })}</option>
					<option value="4">{studioMessage('settings.spaces', { count: 4 })}</option>
				</select>
			</label>
			<div class="setting-select diagnostic-setting">
				<label class="setting-label" for="diagnostic-presentation">{studioMessage('settings.diagnosticMessages')}</label>
				<select
					id="diagnostic-presentation"
					value={editorPreferences.diagnosticPresentation}
					onchange={(event) => updateEditor('diagnosticPresentation', event.currentTarget.value === 'inline' ? 'inline' : 'hover')}
				>
					<option value="hover">{studioMessage('settings.hover')}</option>
					<option value="inline">{studioMessage('settings.inline')}</option>
				</select>
			</div>
		</div>

		<div class="preference-grid">
			{#each booleanPreferences as preference (preference.key)}
				<label class="preference-row" for={`editor-${preference.key}`}>
					<span class="preference-copy">
						<span class="preference-title">{studioMessage(preference.labelKey)}</span>
						<span class="preference-description">{studioMessage(preference.descriptionKey)}</span>
					</span>
					<input
						id={`editor-${preference.key}`}
						type="checkbox"
						class="preference-switch"
						checked={editorPreferences[preference.key]}
						onchange={(event) => updateBooleanEditor(preference.key, event.currentTarget.checked)}
					/>
				</label>
			{/each}
		</div>
	</section>
	{/if}
	</div>
</div>

<style>
	.settings-panel { display: grid; min-height: 100%; grid-template-columns: 9rem minmax(0, 1fr); color: var(--foreground); }
	.settings-categories { display: grid; align-content: start; gap: 0; padding: 0; border-right: 1px solid var(--border); background: color-mix(in srgb, var(--muted) 25%, transparent); }
	.settings-categories p { margin: 0 0 .375rem; padding: .65rem .75rem 0; color: var(--muted-foreground); font-size: .625rem; font-weight: 600; text-transform: uppercase; letter-spacing: .06em; }
	.settings-categories button { display: flex; min-width: 0; min-height: 2.25rem; align-items: center; gap: .5rem; padding: .3rem .75rem; border: 0; border-radius: 0; color: var(--muted-foreground); background: transparent; font: inherit; font-size: .8125rem; text-align: left; cursor: pointer; }
	.settings-categories button:hover, .settings-categories button.active { color: var(--foreground); background: var(--muted); }
	.settings-categories button.active { font-weight: 600; }
	.settings-categories button :global(svg) { flex: 0 0 auto; }
	.settings-content { min-width: 0; padding: .75rem 1rem; container-name: settings-panel; container-type: inline-size; }
	.settings-section { display: grid; min-width: 0; gap: .75rem; }
	.section-heading { display: grid; gap: .125rem; margin-bottom: .125rem; }
	.section-heading h2 { margin: 0; font-size: .875rem; font-weight: 650; }
	.appearance-options { border-block: 1px solid color-mix(in srgb, var(--border) 70%, transparent); }
	.appearance-options .preference-row { min-height: 2.75rem; padding: .4rem .5rem; border: 0; border-radius: 0; background: transparent; }
	.setting-label { color: var(--foreground); font-size: .75rem; font-weight: 600; }
	.setting-select { display: grid; min-width: 0; gap: .25rem; }
	select { width: 100%; min-width: 0; height: 2rem; padding: 0 .5rem; border: 1px solid var(--input); border-radius: .375rem; color: var(--foreground); background: var(--background); font: inherit; font-size: .75rem; }
	select:focus-visible, input:focus-visible { outline: 2px solid var(--ring); outline-offset: 2px; }
	.editor-theme-option { display: grid; min-width: 0; gap: .375rem; padding: .5rem; border: 1px solid color-mix(in srgb, var(--border) 75%, transparent); border-radius: .375rem; background: color-mix(in srgb, var(--muted) 22%, var(--background)); cursor: pointer; }
	.editor-theme-option:has(input:checked) { border-color: var(--ring); background: color-mix(in srgb, var(--accent) 42%, var(--background)); }
	.theme-option-heading { display: flex; min-width: 0; align-items: center; gap: .35rem; font-size: .75rem; font-weight: 550; line-height: 1.25; }
	.theme-option-heading input { width: .875rem; height: .875rem; flex: 0 0 auto; margin: 0; accent-color: var(--primary); }
	.editor-theme-options { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: .375rem; }
	.editor-theme-option { gap: .25rem; }
	.syntax-preview { display: grid; min-width: 0; gap: .125rem; overflow: hidden; padding: .3rem .4rem; border: 1px solid var(--editor-border); border-radius: .25rem; color: var(--editor-foreground); background: var(--editor-background); font-size: .625rem; line-height: 1.35; }
	.syntax-preview code { white-space: nowrap; }
	.syntax-keyword { color: var(--editor-syntax-keyword); font-weight: 650; }
	.syntax-property { color: var(--editor-syntax-property); }
	.syntax-string { color: var(--editor-syntax-string); }
	.syntax-number { color: var(--editor-syntax-number); }
	.syntax-punctuation { color: var(--editor-syntax-punctuation); }
	.syntax-foreground { color: var(--editor-foreground); }
	.preference-grid { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: .5rem; }
	.preference-row { display: flex; min-width: 0; min-height: 4.5rem; align-items: center; justify-content: space-between; gap: .5rem; padding: .5rem; border: 1px solid color-mix(in srgb, var(--border) 80%, transparent); border-radius: .5rem; background: color-mix(in srgb, var(--muted) 22%, var(--background)); cursor: pointer; }
	.preference-copy { display: grid; min-width: 0; gap: .125rem; }
	.preference-title { font-size: .75rem; font-weight: 550; line-height: 1.25; }
	.preference-description { color: var(--muted-foreground); font-size: .6875rem; line-height: 1.3; }
	.preference-switch { position: relative; display: block; width: 2rem; height: 1.125rem; flex: 0 0 auto; margin: 0; appearance: none; border: 1px solid var(--input); border-radius: 999px; background: var(--muted); cursor: pointer; transition: border-color 120ms ease, background-color 120ms ease; }
	.preference-switch::before { position: absolute; inset: .12rem auto .12rem .12rem; width: .82rem; border-radius: 999px; background: var(--background); box-shadow: 0 1px 2px rgb(0 0 0 / 18%); content: ''; transition: transform 120ms ease; }
	.preference-switch:checked { border-color: var(--primary); background: var(--primary); }
	.preference-switch:checked::before { background: var(--primary-foreground); transform: translateX(.8rem); }
	.select-grid { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: .5rem; }
	.diagnostic-setting { grid-column: 1 / -1; }
	@container settings-panel (max-width: 34rem) {
		.preference-grid { grid-template-columns: minmax(0, 1fr); }
	}
	@container settings-panel (max-width: 30rem) {
		.editor-theme-options { grid-template-columns: repeat(2, minmax(0, 1fr)); }
	}
	@container settings-panel (max-width: 22rem) {
		.editor-theme-options { grid-template-columns: minmax(0, 1fr); }
	}
	@media (max-width: 35rem) {
		.settings-panel { grid-template-columns: 7rem minmax(0, 1fr); }
		.settings-categories button { gap: .375rem; padding-inline: .4rem; font-size: .75rem; }
		.settings-categories button :global(svg) { width: 1.25rem; height: 1.25rem; }
		.settings-content { padding: .75rem .5rem; }
	}
</style>
