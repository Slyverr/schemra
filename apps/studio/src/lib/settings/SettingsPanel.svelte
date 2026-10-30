<script lang="ts">
	import { onMount } from 'svelte';
	import { FileCode2, Palette } from '@lucide/svelte';
	import { Button } from '../components/ui/button/index.js';
	import { Label } from '../components/ui/label/index.js';
	import * as Select from '../components/ui/select/index.js';
	import StudioThemePicker from './StudioThemePicker.svelte';
	import SettingsPreferenceSwitch from './SettingsPreferenceSwitch.svelte';
	import { settingsCategoryVariants, settingsClasses as classes } from './settings-classes.js';
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
		studioMessage(
			studioThemeOptions.find(({ value }) => value === resolvedStudioTheme)?.labelKey ??
				'theme.schemraLight'
		)
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

	function chooseZoom(value: string) {
		const zoom = Number(value);
		if (EDITOR_ZOOM_LEVELS.includes(zoom as EditorZoom)) updateEditor('zoom', zoom as EditorZoom);
	}

	function chooseIndentSize(value: string) {
		updateEditor('indentSize', Number(value) === 4 ? (4 satisfies EditorIndentSize) : 2);
	}
</script>

<div class={classes.panel}>
	<nav class={classes.navigation} aria-label={studioMessage('settings.categories')}>
		<p class={classes.navigationLabel}>
			{studioMessage('settings.preferences')}
		</p>
		<Button
			variant="ghost"
			class={settingsCategoryVariants({ active: activeCategory === 'appearance' })}
			type="button"
			aria-current={activeCategory === 'appearance' ? 'page' : undefined}
			onclick={() => (activeCategory = 'appearance')}
		>
			<Palette size={16} aria-hidden="true" />
			<span>{studioMessage('settings.appearance')}</span>
		</Button>
		<Button
			variant="ghost"
			class={settingsCategoryVariants({ active: activeCategory === 'editor' })}
			type="button"
			aria-current={activeCategory === 'editor' ? 'page' : undefined}
			onclick={() => (activeCategory = 'editor')}
		>
			<FileCode2 size={16} aria-hidden="true" />
			<span>{studioMessage('settings.editor')}</span>
		</Button>
	</nav>

	<div class={classes.content}>
		{#if activeCategory === 'appearance'}
			<section class={classes.section} aria-labelledby="appearance-heading">
				<StudioThemePicker
					theme={themePreferences.studio}
					systemThemeLabel={activeStudioThemeLabel}
					{popoverBoundary}
					onChange={(value) => updateTheme('studio', value)}
				/>

				<div class={classes.sectionHeading}>
					<h2 id="appearance-heading" class={classes.sectionTitle}>
						{studioMessage('settings.appearanceOptions')}
					</h2>
				</div>

				<div class={classes.appearanceOptions}>
					<SettingsPreferenceSwitch
						id="reduce-motion"
						label={studioMessage('settings.reduceMotion')}
						description={studioMessage('settings.reduceMotionDescription')}
						variant="inline"
						checked={themePreferences.reduceMotion}
						onCheckedChange={(value) => updateTheme('reduceMotion', value)}
					/>
				</div>
			</section>
		{:else}
			<section class={classes.section} aria-labelledby="editor-heading">
				<div class={classes.sectionHeading}>
					<h2 id="editor-heading" class={classes.sectionTitle}>
						{studioMessage('settings.syntaxColors')}
					</h2>
				</div>

				<fieldset class="m-0 min-w-0 border-0 p-0" aria-label={studioMessage('settings.editorSyntaxTheme')}>
					<div class={classes.themeOptions}>
						{#each editorThemeOptions as option (option.value)}
							<label class={classes.themeOption}>
								<span class={classes.themeOptionLabel}>
									<input
										type="radio"
										name="editor-theme"
										value={option.value}
										checked={themePreferences.editor === option.value}
										onchange={() => updateTheme('editor', option.value)}
										class="m-0 size-3.5 shrink-0 accent-primary"
									/>
									<span>{studioMessage(option.labelKey)}</span>
								</span>
								<span class={classes.syntaxPreview} style={editorThemeStyles(resolveEditorTheme(option.value, resolvedStudioTheme))} aria-hidden="true">
									<code class={classes.syntaxCode}><span class="font-[650] text-[var(--editor-syntax-keyword)]">move</span><span class="text-[var(--editor-foreground)]"> request </span><span class="text-[var(--editor-syntax-punctuation)]">-&gt;</span><span class="text-[var(--editor-foreground)]"> export</span></code>
									<code class={classes.syntaxCode}><span class="text-[var(--editor-syntax-property)]">label</span><span class="text-[var(--editor-syntax-string)]"> "Saved" </span><span class="text-[var(--editor-syntax-property)]">duration</span><span class="text-[var(--editor-syntax-number)]"> 720</span></code>
								</span>
							</label>
						{/each}
					</div>
				</fieldset>

				<div class={classes.sectionHeading}>
					<h2 id="editor-options-heading" class={classes.sectionTitle}>
						{studioMessage('settings.editorOptions')}
					</h2>
				</div>

				<div class={classes.settingGrid}>
					<div class="grid min-w-0 gap-1">
						<Label for="editor-zoom" class={classes.settingLabel}>{studioMessage('settings.textSize')}</Label>
						<Select.Root type="single" value={String(editorPreferences.zoom)} onValueChange={chooseZoom}>
							<Select.Trigger id="editor-zoom" class={classes.selectTrigger}><Select.Value /></Select.Trigger>
							<Select.Content align="start">
								<Select.Group>
									{#each EDITOR_ZOOM_LEVELS as zoom (zoom)}
										<Select.Item value={String(zoom)} label={`${zoom}%`}>{zoom}%</Select.Item>
									{/each}
								</Select.Group>
							</Select.Content>
						</Select.Root>
					</div>

					<div class="grid min-w-0 gap-1">
						<Label for="editor-indent-size" class={classes.settingLabel}>{studioMessage('settings.indentSize')}</Label>
						<Select.Root type="single" value={String(editorPreferences.indentSize)} onValueChange={chooseIndentSize}>
							<Select.Trigger id="editor-indent-size" class={classes.selectTrigger}><Select.Value /></Select.Trigger>
							<Select.Content align="start">
								<Select.Group>
									<Select.Item value="2" label={studioMessage('settings.spaces', { count: 2 })}>{studioMessage('settings.spaces', { count: 2 })}</Select.Item>
									<Select.Item value="4" label={studioMessage('settings.spaces', { count: 4 })}>{studioMessage('settings.spaces', { count: 4 })}</Select.Item>
								</Select.Group>
							</Select.Content>
						</Select.Root>
					</div>

					<div class="col-span-full grid min-w-0 gap-1">
						<Label for="diagnostic-presentation" class={classes.settingLabel}>{studioMessage('settings.diagnosticMessages')}</Label>
						<Select.Root
							type="single"
							value={editorPreferences.diagnosticPresentation}
							onValueChange={(value) => updateEditor('diagnosticPresentation', value === 'inline' ? 'inline' : 'hover')}
						>
							<Select.Trigger id="diagnostic-presentation" class={classes.selectTrigger}><Select.Value /></Select.Trigger>
							<Select.Content align="start">
								<Select.Group>
									<Select.Item value="hover" label={studioMessage('settings.hover')}>{studioMessage('settings.hover')}</Select.Item>
									<Select.Item value="inline" label={studioMessage('settings.inline')}>{studioMessage('settings.inline')}</Select.Item>
								</Select.Group>
							</Select.Content>
						</Select.Root>
					</div>
				</div>

				<div class={classes.preferenceGrid}>
					{#each booleanPreferences as preference (preference.key)}
						{@const id = `editor-${preference.key}`}
						<SettingsPreferenceSwitch
							{id}
							label={studioMessage(preference.labelKey)}
							description={studioMessage(preference.descriptionKey)}
							checked={editorPreferences[preference.key]}
							onCheckedChange={(value) => updateEditor(preference.key, value)}
						/>
					{/each}
				</div>
			</section>
		{/if}
	</div>
</div>
