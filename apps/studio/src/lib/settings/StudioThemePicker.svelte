<script lang="ts">
	import { tick } from 'svelte';
	import { Popover } from 'bits-ui';
	import { ChevronDown } from '@lucide/svelte';
	import { studioMessage } from '../i18n/messages.js';
	import { nextThemeOptionIndex } from './theme-picker.js';
	import { studioThemeOptions, type StudioTheme } from './appearance.js';

	interface Props {
		theme: StudioTheme;
		systemThemeLabel: string;
		popoverBoundary?: Element | null;
		onChange: (theme: StudioTheme) => void;
	}

	let { theme, systemThemeLabel, popoverBoundary, onChange }: Props = $props();
	let isOpen = $state(false);
	let search = $state('');
	let trigger = $state<HTMLButtonElement>();
	let searchInput = $state<HTMLInputElement>();
	let activeIndex = $state(0);

	const selectedTheme = $derived(studioThemeOptions.find(({ value }) => value === theme) ?? studioThemeOptions[1]!);
	const filteredThemes = $derived(studioThemeOptions.filter(({ labelKey }) =>
		studioMessage(labelKey).toLocaleLowerCase().includes(search.trim().toLocaleLowerCase())
	));
	const activeOptionId = $derived(
		filteredThemes[activeIndex] ? `studio-theme-option-${filteredThemes[activeIndex]!.value}` : undefined
	);

	$effect(() => {
		if (isOpen) void tick().then(() => searchInput?.focus());
	});

	function openPicker() {
		if (isOpen) {
			closePicker();
			return;
		}
		search = '';
		activeIndex = Math.max(0, studioThemeOptions.findIndex(({ value }) => value === theme));
		isOpen = true;
	}

	function closePicker() {
		isOpen = false;
		search = '';
		void tick().then(() => trigger?.focus());
	}

	function chooseTheme(value: StudioTheme) {
		onChange(value);
		closePicker();
	}

	function handleSearchKeydown(event: KeyboardEvent) {
		if (event.key === 'Escape') {
			event.preventDefault();
			closePicker();
			return;
		}
		if (event.key === 'Enter' && filteredThemes[activeIndex]) {
			event.preventDefault();
			chooseTheme(filteredThemes[activeIndex]!.value);
			return;
		}

		const key = event.key;
		if ((key !== 'ArrowDown' && key !== 'ArrowUp' && key !== 'Home' && key !== 'End') || filteredThemes.length === 0) return;
		event.preventDefault();
		activeIndex = nextThemeOptionIndex(activeIndex, filteredThemes.length, key);
		void tick().then(() => {
			if (activeOptionId) document.getElementById(activeOptionId)?.scrollIntoView({ block: 'nearest' });
		});
	}
</script>

<fieldset class="theme-picker">
	<legend class="setting-label">{studioMessage('settings.studioTheme')}</legend>
	<div class="theme-combobox">
	<Popover.Root bind:open={isOpen}>
		<Popover.Trigger>
			{#snippet child({ props })}
				<button {...props} bind:this={trigger} class="theme-trigger" type="button" aria-haspopup="dialog" aria-expanded={isOpen} aria-controls="studio-theme-options" onclick={openPicker}>
			<span class={`studio-swatch ${selectedTheme.previewClass}`} aria-hidden="true">
				<span class="swatch-card"><span class="swatch-dot"></span><span class="swatch-lines"><i></i><i></i><i></i></span></span>
			</span>
			<span class="theme-trigger-copy">
				<span class="theme-trigger-name">{studioMessage(selectedTheme.labelKey)}</span>
				{#if theme === 'system'}<span class="theme-trigger-system">{studioMessage('settings.systemThemeUsing', { theme: systemThemeLabel })}</span>{/if}
			</span>
			<ChevronDown size={15} aria-hidden="true" />
				</button>
			{/snippet}
		</Popover.Trigger>
		<Popover.Portal to={popoverBoundary ?? 'body'}>
			<Popover.Content side="bottom" align="start" sideOffset={6} collisionPadding={10} collisionBoundary={popoverBoundary ?? []} class="theme-picker-popover z-[1] w-[min(24rem,var(--bits-floating-available-width,24rem))] max-h-[min(22rem,60vh,var(--bits-floating-available-height,60vh))]" id="studio-theme-options" role="dialog" aria-label={studioMessage('settings.studioTheme')}>
				{#snippet child({ props, wrapperProps })}
					<div {...wrapperProps} class="!z-[1000]">
						<div {...props}>
				<input bind:this={searchInput} bind:value={search} type="search" role="combobox" aria-expanded="true" aria-controls="studio-theme-list" aria-activedescendant={activeOptionId} aria-autocomplete="list" placeholder={studioMessage('settings.searchThemesPlaceholder')} aria-label={studioMessage('settings.searchThemes')} oninput={() => (activeIndex = 0)} onkeydown={handleSearchKeydown} />
				{#if filteredThemes.length > 0}
					<ul id="studio-theme-list" role="listbox" aria-label={studioMessage('settings.themeOptions')}>
						{#each filteredThemes as option, index (option.value)}
							<li role="presentation">
								<button id={`studio-theme-option-${option.value}`} type="button" role="option" tabindex="-1" aria-selected={theme === option.value} class:active={activeIndex === index} onpointermove={() => (activeIndex = index)} onpointerdown={(event) => event.preventDefault()} onclick={() => chooseTheme(option.value)}>
									<span class={`studio-swatch ${option.previewClass}`} aria-hidden="true"><span class="swatch-card"><span class="swatch-dot"></span><span class="swatch-lines"><i></i><i></i><i></i></span></span></span>
									<span>{studioMessage(option.labelKey)}</span>
								</button>
							</li>
						{/each}
					</ul>
				{:else}
					<p class="no-theme-results">{studioMessage('settings.noMatchingThemes')}</p>
				{/if}
						</div>
					</div>
				{/snippet}
			</Popover.Content>
		</Popover.Portal>
	</Popover.Root>
	</div>
</fieldset>

<style>
	.theme-picker { min-width: 0; margin: 0; padding: 0; border: 0; }
	.theme-picker legend { margin-bottom: .5rem; padding: 0; font-size: .875rem; font-weight: 600; }
	.setting-label { color: var(--foreground); font-size: .6875rem; font-weight: 600; }
	.theme-combobox { position: relative; }
	.theme-trigger { display: grid; width: 100%; min-height: 3rem; grid-template-columns: 3rem minmax(0, 1fr) auto; align-items: center; gap: .75rem; padding: .25rem .5rem; border: 1px solid var(--input); border-radius: .45rem; color: var(--foreground); background: var(--background); font: inherit; font-size: .75rem; text-align: left; cursor: pointer; }
	.theme-trigger:hover, .theme-trigger[aria-expanded='true'] { border-color: var(--ring); background: var(--accent); }
	.theme-trigger .studio-swatch { width: 3rem; height: 2.5rem; }
	.theme-trigger-copy { display: grid; min-width: 0; gap: .1rem; }
	.theme-trigger-name, .theme-trigger-system { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
	.theme-trigger-name { font-weight: 550; }
	.theme-trigger-system { color: var(--muted-foreground); font-size: .65rem; }
	.studio-swatch { display: flex; height: 2rem; align-items: center; padding: .25rem; overflow: hidden; border: 1px solid var(--swatch-border); border-radius: .375rem; background: var(--swatch-background); }
	:global(.theme-picker-popover) { display: grid; gap: .4rem; padding: .5rem; overflow: hidden; border: 1px solid var(--border); border-radius: .55rem; color: var(--foreground); background: var(--popover); box-shadow: 0 .65rem 1.8rem rgb(7 21 41 / 16%); }
	:global(.theme-picker-popover > input) { width: 100%; min-width: 0; height: 2.1rem; padding: 0 .55rem; border: 1px solid var(--input); border-radius: .375rem; color: var(--foreground); background: var(--background); font: inherit; font-size: .75rem; }
	:global(.theme-picker-popover ul) { display: grid; min-height: 0; gap: .15rem; margin: 0; padding: 0; overflow: auto; list-style: none; }
	:global(.theme-picker-popover li button) { display: grid; width: 100%; min-height: 3rem; grid-template-columns: 3rem minmax(0, 1fr); align-items: center; gap: .75rem; padding: .25rem .4rem; border: 0; border-radius: .35rem; color: var(--foreground); background: transparent; font: inherit; font-size: .75rem; text-align: left; cursor: pointer; }
	:global(.theme-picker-popover li button:hover), :global(.theme-picker-popover li button.active) { background: var(--accent); }
	:global(.theme-picker-popover .studio-swatch) { width: 3rem; height: 2.5rem; }
	.no-theme-results { margin: 0; padding: .6rem; color: var(--muted-foreground); font-size: .72rem; }
	.studio-swatch.schemra-light { --swatch-background: #f7faff; --swatch-border: #d6e2f1; --swatch-card: #ffffff; --swatch-foreground: #102442; --swatch-muted: #5c6f87; --swatch-primary: #0758c8; }
	.studio-swatch.schemra-dark { --swatch-background: #09172d; --swatch-border: #23476e; --swatch-card: #0d2342; --swatch-foreground: #eaf3ff; --swatch-muted: #a9c1dd; --swatch-primary: #28aaff; }
	.studio-swatch.monochrome-light { --swatch-background: #ffffff; --swatch-border: #d4d4d4; --swatch-card: #f5f5f5; --swatch-foreground: #171717; --swatch-muted: #737373; --swatch-primary: #262626; }
	.studio-swatch.monochrome-dark { --swatch-background: #171717; --swatch-border: #414141; --swatch-card: #202020; --swatch-foreground: #f5f5f5; --swatch-muted: #b3b3b3; --swatch-primary: #e5e5e5; }
	.studio-swatch.catppuccin-latte { --swatch-background: #eff1f5; --swatch-border: #ccd0da; --swatch-card: #e6e9ef; --swatch-foreground: #4c4f69; --swatch-muted: #6c6f85; --swatch-primary: #8839ef; }
	.studio-swatch.catppuccin-mocha { --swatch-background: #1e1e2e; --swatch-border: #45475a; --swatch-card: #181825; --swatch-foreground: #cdd6f4; --swatch-muted: #a6adc8; --swatch-primary: #cba6f7; }
	.studio-swatch.nord { --swatch-background: #2e3440; --swatch-border: #4c566a; --swatch-card: #3b4252; --swatch-foreground: #eceff4; --swatch-muted: #d8dee9; --swatch-primary: #88c0d0; }
	.studio-swatch.dracula { --swatch-background: #282a36; --swatch-border: #44475a; --swatch-card: #21222c; --swatch-foreground: #f8f8f2; --swatch-muted: #bdc1d5; --swatch-primary: #bd93f9; }
	.studio-swatch.system { --swatch-background: linear-gradient(110deg, #f7faff 0 48%, #09172d 52%); --swatch-border: #9eb4ce; --swatch-card: linear-gradient(110deg, #ffffff 0 48%, #0d2342 52%); --swatch-foreground: #102442; --swatch-muted: #71839b; --swatch-primary: #0758c8; }
	.swatch-card { display: flex; width: 100%; height: 100%; align-items: center; gap: .375rem; padding: 0 .375rem; border-radius: .25rem; background: var(--swatch-card); }
	.swatch-dot { width: .5rem; height: .5rem; flex: 0 0 auto; border-radius: 999px; background: var(--swatch-primary); }
	.swatch-lines { display: grid; flex: 1 1 auto; gap: .125rem; }
	.swatch-lines i { display: block; width: 100%; height: .125rem; border-radius: 999px; background: var(--swatch-muted); opacity: .65; }
	.swatch-lines i:first-child { width: 70%; height: .1875rem; background: var(--swatch-foreground); opacity: .85; }
	.swatch-lines i:last-child { width: 55%; }
</style>
