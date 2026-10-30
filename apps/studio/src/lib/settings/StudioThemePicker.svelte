<script lang="ts">
	import { tick } from 'svelte';
	import { Popover } from 'bits-ui';
	import { ChevronDown } from '@lucide/svelte';
	import { Button } from '../components/ui/button/index.js';
	import { Input } from '../components/ui/input/index.js';
	import { studioMessage } from '../i18n/messages.js';
	import { nextThemeOptionIndex } from './theme-picker.js';
	import { studioThemeOptions, type StudioTheme } from './appearance.js';
	import StudioThemeSwatch from './StudioThemeSwatch.svelte';
	import { themePickerClasses as classes } from './settings-classes.js';

	interface Props {
		theme: StudioTheme;
		systemThemeLabel: string;
		popoverBoundary?: Element | null;
		onChange: (theme: StudioTheme) => void;
	}

	let { theme, systemThemeLabel, popoverBoundary, onChange }: Props = $props();
	let isOpen = $state(false);
	let search = $state('');
	let trigger = $state<HTMLButtonElement | null>(null);
	let searchInput = $state<HTMLInputElement | null>(null);
	let activeIndex = $state(0);

	const selectedTheme = $derived(studioThemeOptions.find(({ value }) => value === theme) ?? studioThemeOptions[1]!);
	const filteredThemes = $derived(
		studioThemeOptions.filter(({ labelKey }) =>
			studioMessage(labelKey).toLocaleLowerCase().includes(search.trim().toLocaleLowerCase())
		)
	);
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

<fieldset class="m-0 min-w-0 border-0 p-0">
	<legend class="mb-2 block p-0 text-sm font-semibold text-foreground">{studioMessage('settings.studioTheme')}</legend>
	<Popover.Root bind:open={isOpen}>
		<Popover.Trigger>
			{#snippet child({ props })}
				<Button
					{...props}
					bind:ref={trigger}
					variant="outline"
					class={classes.trigger}
					type="button"
					aria-haspopup="dialog"
					aria-expanded={isOpen}
					aria-controls="studio-theme-options"
					onclick={openPicker}
				>
					<StudioThemeSwatch theme={selectedTheme.value} />
					<span class={classes.triggerCopy}>
						<span class="truncate font-[550]">{studioMessage(selectedTheme.labelKey)}</span>
						{#if theme === 'system'}
							<span class="truncate text-[.65rem] text-muted-foreground">
								{studioMessage('settings.systemThemeUsing', { theme: systemThemeLabel })}
							</span>
						{/if}
					</span>
					<ChevronDown size={15} aria-hidden="true" />
				</Button>
			{/snippet}
		</Popover.Trigger>
		<Popover.Portal to={popoverBoundary ?? 'body'}>
			<Popover.Content
				side="bottom"
				align="start"
				sideOffset={6}
				collisionPadding={10}
				collisionBoundary={popoverBoundary ?? []}
				id="studio-theme-options"
				role="dialog"
				aria-label={studioMessage('settings.studioTheme')}
			>
				{#snippet child({ props, wrapperProps })}
					<div {...wrapperProps} class="!z-[1000]">
						<div {...props} class={`${classes.popover} ${classes.popoverContent}`}>
							<Input
								bind:ref={searchInput}
								bind:value={search}
								type="search"
								role="combobox"
								aria-expanded="true"
								aria-controls="studio-theme-list"
								aria-activedescendant={activeOptionId}
								aria-autocomplete="list"
								placeholder={studioMessage('settings.searchThemesPlaceholder')}
								aria-label={studioMessage('settings.searchThemes')}
								class={classes.search}
								oninput={() => (activeIndex = 0)}
								onkeydown={handleSearchKeydown}
							/>
							{#if filteredThemes.length > 0}
								<ul id="studio-theme-list" class={classes.list} role="listbox" aria-label={studioMessage('settings.themeOptions')}>
									{#each filteredThemes as option, index (option.value)}
									<li role="presentation">
										<Button
											id={`studio-theme-option-${option.value}`}
											variant="ghost"
											class={`${classes.option} ${activeIndex === index ? classes.activeOption : ''}`}
											type="button"
											role="option"
											tabindex={-1}
											aria-selected={theme === option.value}
											onpointermove={() => (activeIndex = index)}
											onpointerdown={(event) => event.preventDefault()}
											onclick={() => chooseTheme(option.value)}
										>
											<StudioThemeSwatch theme={option.value} />
											<span class="min-w-0 truncate text-left">{studioMessage(option.labelKey)}</span>
										</Button>
									</li>
									{/each}
								</ul>
							{:else}
								<p class={classes.empty}>
									{studioMessage('settings.noMatchingThemes')}
								</p>
							{/if}
						</div>
					</div>
				{/snippet}
			</Popover.Content>
		</Popover.Portal>
	</Popover.Root>
</fieldset>
