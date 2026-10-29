<script lang="ts">
	import { onMount } from 'svelte';
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

	interface Props {
		themePreferences: ThemePreferences;
		editorPreferences: EditorPreferences;
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
		onThemePreferencesChange,
		onEditorPreferencesChange
	}: Props = $props();
	let systemPrefersDark = $state(false);
	const resolvedStudioTheme = $derived(resolveStudioTheme(themePreferences.studio, systemPrefersDark));
	const activeStudioThemeLabel = $derived(
		studioThemeOptions.find(({ value }) => value === resolvedStudioTheme)?.label ?? 'Schemra Light'
	);

	const booleanPreferences: Array<{
		key: BooleanEditorPreference;
		label: string;
		description: string;
	}> = [
		{ key: 'lineNumbers', label: 'Line numbers', description: 'Show a numbered code gutter.' },
		{ key: 'indentGuides', label: 'Block scope guides', description: 'Show guides inside nested blocks.' },
		{ key: 'activeLineHighlight', label: 'Current line highlight', description: 'Shade the caret line.' },
		{ key: 'matchingBracketHighlight', label: 'Matching brackets', description: 'Highlight the matching brace.' },
		{ key: 'autoIndent', label: 'Auto-indent', description: 'Carry block indentation to new lines.' },
		{ key: 'autoClosePairs', label: 'Auto-close pairs', description: 'Insert matching brackets and quotes.' },
		{ key: 'formatOnSave', label: 'Format on save', description: 'Format the document when saving.' }
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
	<section class="settings-section" aria-labelledby="appearance-heading">
		<div class="section-heading">
			<h2 id="appearance-heading">Appearance</h2>
			<p>Choose the Studio and editor color themes.</p>
		</div>

		<div class="setting-card">
			<fieldset class="theme-picker">
				<legend class="setting-label">Studio theme</legend>
				<div class="studio-theme-options">
					{#each studioThemeOptions as option (option.value)}
						<label class="theme-option">
							<span class="theme-option-heading">
								<input
									type="radio"
									name="studio-theme"
									value={option.value}
									checked={themePreferences.studio === option.value}
									onchange={() => updateTheme('studio', option.value)}
								/>
								<span>{option.label}</span>
							</span>
							<span class={`studio-swatch ${option.previewClass}`} aria-hidden="true">
								<span class="swatch-card">
									<span class="swatch-dot"></span>
									<span class="swatch-lines"><i></i><i></i><i></i></span>
								</span>
							</span>
						</label>
					{/each}
				</div>
			</fieldset>
			{#if themePreferences.studio === 'system'}
				<p class="setting-hint">Using {activeStudioThemeLabel} based on your system preference.</p>
			{/if}
		</div>

		<div class="setting-card">
			<fieldset class="theme-picker">
				<legend class="setting-label">Editor syntax theme</legend>
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
								<span>{option.label}</span>
							</span>
							<span class="syntax-preview" style={editorThemeStyles(resolveEditorTheme(option.value, resolvedStudioTheme))} aria-hidden="true">
								<code>
									<span class="syntax-keyword">move</span><span class="syntax-foreground"> packet </span>
									<span class="syntax-punctuation">-&gt;</span><span class="syntax-foreground"> server</span>
								</code>
							</span>
							<span class="theme-description">{option.description}</span>
						</label>
					{/each}
				</div>
			</fieldset>
		</div>

		<label class="preference-row" for="reduce-motion">
			<span class="preference-copy">
				<span class="preference-title">Reduce interface motion</span>
				<span class="preference-description">Minimize transitions and animated interface effects.</span>
			</span>
			<input
				id="reduce-motion"
				type="checkbox"
				checked={themePreferences.reduceMotion}
				onchange={(event) => updateTheme('reduceMotion', event.currentTarget.checked)}
			/>
		</label>
	</section>

	<section class="settings-section" aria-labelledby="editor-heading">
		<div class="section-heading">
			<h2 id="editor-heading">Editor</h2>
			<p>Adjust reading and editing behavior.</p>
		</div>

		<div class="select-grid">
			<label class="setting-card" for="editor-zoom">
				<span class="setting-label">Text size</span>
				<select id="editor-zoom" value={editorPreferences.zoom} onchange={(event) => chooseZoom(event.currentTarget.value)}>
					{#each EDITOR_ZOOM_LEVELS as zoom (zoom)}
						<option value={zoom}>{zoom}%</option>
					{/each}
				</select>
			</label>
			<label class="setting-card" for="editor-indent-size">
				<span class="setting-label">Indent size</span>
				<select id="editor-indent-size" value={editorPreferences.indentSize} onchange={(event) => chooseIndentSize(event.currentTarget.value)}>
					<option value="2">2 spaces</option>
					<option value="4">4 spaces</option>
				</select>
			</label>
			<div class="setting-card diagnostic-setting">
				<label class="setting-label" for="diagnostic-presentation">Diagnostic messages</label>
				<select
					id="diagnostic-presentation"
					value={editorPreferences.diagnosticPresentation}
					onchange={(event) => updateEditor('diagnosticPresentation', event.currentTarget.value === 'inline' ? 'inline' : 'hover')}
				>
					<option value="hover">Hover</option>
					<option value="inline">Inline</option>
				</select>
			</div>
		</div>

		<div class="preference-grid">
			{#each booleanPreferences as preference (preference.key)}
				<label class="preference-row" for={`editor-${preference.key}`}>
					<span class="preference-copy">
						<span class="preference-title">{preference.label}</span>
						<span class="preference-description">{preference.description}</span>
					</span>
					<input
						id={`editor-${preference.key}`}
						type="checkbox"
						checked={editorPreferences[preference.key]}
						onchange={(event) => updateBooleanEditor(preference.key, event.currentTarget.checked)}
					/>
				</label>
			{/each}
		</div>
	</section>
</div>

<style>
	.settings-panel { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); align-items: start; gap: 1rem; color: var(--foreground); }
	.settings-section { display: grid; min-width: 0; gap: .625rem; padding: .875rem; border: 1px solid var(--border); border-radius: .75rem; background: var(--background); }
	.section-heading { display: grid; gap: .125rem; margin-bottom: .125rem; }
	.section-heading h2 { margin: 0; font-size: .875rem; font-weight: 650; }
	.section-heading p, .setting-hint { margin: 0; color: var(--muted-foreground); font-size: .6875rem; line-height: 1.4; }
	.setting-card { display: grid; min-width: 0; gap: .375rem; padding: .625rem; border: 1px solid color-mix(in srgb, var(--border) 75%, transparent); border-radius: .5rem; background: color-mix(in srgb, var(--muted) 22%, var(--background)); }
	.setting-label { color: var(--foreground); font-size: .6875rem; font-weight: 600; }
	select { width: 100%; min-width: 0; height: 2rem; padding: 0 .5rem; border: 1px solid var(--input); border-radius: .375rem; color: var(--foreground); background: var(--background); font: inherit; font-size: .75rem; }
	select:focus-visible, input:focus-visible { outline: 2px solid var(--ring); outline-offset: 2px; }
	.theme-picker { min-width: 0; margin: 0; padding: 0; border: 0; }
	.theme-picker legend { margin-bottom: .4rem; padding: 0; }
	.studio-theme-options { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: .375rem; }
	.theme-option, .editor-theme-option { display: grid; min-width: 0; gap: .375rem; padding: .5rem; border: 1px solid color-mix(in srgb, var(--border) 75%, transparent); border-radius: .5rem; background: color-mix(in srgb, var(--muted) 22%, var(--background)); cursor: pointer; }
	.theme-option:has(input:checked), .editor-theme-option:has(input:checked) { border-color: var(--ring); background: color-mix(in srgb, var(--accent) 42%, var(--background)); }
	.theme-option-heading { display: flex; min-width: 0; align-items: center; gap: .35rem; font-size: .6875rem; font-weight: 550; line-height: 1.25; }
	.theme-option-heading input { width: .875rem; height: .875rem; flex: 0 0 auto; margin: 0; accent-color: var(--primary); }
	.studio-swatch { display: flex; height: 2rem; align-items: center; padding: .25rem; overflow: hidden; border: 1px solid var(--swatch-border); border-radius: .375rem; background: var(--swatch-background); }
	.studio-swatch.schemra-light { --swatch-background: #f7faff; --swatch-border: #d6e2f1; --swatch-card: #ffffff; --swatch-foreground: #102442; --swatch-muted: #5c6f87; --swatch-primary: #0758c8; }
	.studio-swatch.schemra-dark { --swatch-background: #09172d; --swatch-border: #23476e; --swatch-card: #0d2342; --swatch-foreground: #eaf3ff; --swatch-muted: #a9c1dd; --swatch-primary: #28aaff; }
	.studio-swatch.system { --swatch-background: linear-gradient(110deg, #f7faff 0 48%, #09172d 52%); --swatch-border: #9eb4ce; --swatch-card: linear-gradient(110deg, #ffffff 0 48%, #0d2342 52%); --swatch-foreground: #102442; --swatch-muted: #71839b; --swatch-primary: #0758c8; }
	.swatch-card { display: flex; width: 100%; height: 100%; align-items: center; gap: .375rem; padding: 0 .375rem; border: 1px solid var(--swatch-border); border-radius: .25rem; background: var(--swatch-card); }
	.swatch-dot { width: .5rem; height: .5rem; flex: 0 0 auto; border-radius: 999px; background: var(--swatch-primary); }
	.swatch-lines { display: grid; flex: 1 1 auto; gap: .125rem; }
	.swatch-lines i { display: block; width: 100%; height: .125rem; border-radius: 999px; background: var(--swatch-muted); opacity: .65; }
	.swatch-lines i:first-child { width: 70%; height: .1875rem; background: var(--swatch-foreground); opacity: .85; }
	.swatch-lines i:last-child { width: 55%; }
	.editor-theme-options { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: .375rem; }
	.editor-theme-option { gap: .25rem; }
	.theme-description { color: var(--muted-foreground); font-size: .625rem; line-height: 1.3; }
	.syntax-preview { display: block; min-width: 0; overflow: hidden; padding: .3rem .4rem; border: 1px solid var(--editor-border); border-radius: .375rem; color: var(--editor-foreground); background: var(--editor-background); font-size: .625rem; line-height: 1.5; }
	.syntax-preview code { white-space: nowrap; }
	.syntax-keyword { color: var(--editor-syntax-keyword); font-weight: 650; }
	.syntax-punctuation { color: var(--editor-syntax-punctuation); }
	.syntax-foreground { color: var(--editor-foreground); }
	.preference-grid { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: .375rem; }
	.preference-row { display: flex; min-width: 0; min-height: 3rem; align-items: center; justify-content: space-between; gap: .5rem; padding: .5rem .625rem; border: 1px solid color-mix(in srgb, var(--border) 75%, transparent); border-radius: .5rem; background: color-mix(in srgb, var(--muted) 22%, var(--background)); cursor: pointer; }
	.preference-copy { display: grid; min-width: 0; gap: .125rem; }
	.preference-title { font-size: .6875rem; font-weight: 550; line-height: 1.25; }
	.preference-description { color: var(--muted-foreground); font-size: .625rem; line-height: 1.3; }
	.preference-row input { width: 1rem; height: 1rem; flex: 0 0 auto; margin: 0; accent-color: var(--primary); }
	.select-grid { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: .375rem; }
	.diagnostic-setting { grid-column: 1 / -1; }
	@media (max-width: 760px) { .settings-panel { grid-template-columns: minmax(0, 1fr); } }
	@media (max-width: 440px) { .preference-grid, .select-grid, .editor-theme-options { grid-template-columns: minmax(0, 1fr); } .diagnostic-setting { grid-column: auto; } }
</style>
