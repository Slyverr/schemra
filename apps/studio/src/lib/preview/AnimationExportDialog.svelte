<script lang="ts">
	import { X } from '@lucide/svelte';
	import * as Dialog from '../components/ui/dialog/index.js';
	import { Button } from '../components/ui/button/index.js';
	import { Input } from '../components/ui/input/index.js';
	import { Label } from '../components/ui/label/index.js';
	import * as Select from '../components/ui/select/index.js';
	import { Tabs } from 'bits-ui';
	import {
		cancelActiveAnimationExport,
		animationFormatOptions,
		getAnimationExportStoragePlan,
		isValidAnimationExportBitrate,
		isValidAnimationExportDimensions,
		isValidAnimationExportFps,
		supportsAnimationFormat,
		supportsDirectAnimationFileSave,
		type AnimationExportCapabilitySettings,
		type AnimationFormat
	} from './animation-export.js';
	import { studioMessage } from '../i18n/messages.js';

	export type AnimationBackgroundChoice = 'visual' | 'dark' | 'custom';
	export interface AnimationExportSettings {
		format: AnimationFormat;
		background: string;
		durationMs: number;
		fps: 24 | 30 | 60;
		width: number;
		height: number;
		videoBitsPerSecond: number;
	}

	interface Props {
		open: boolean;
		busy: boolean;
		progress: number;
		scenarioTitle: string;
		durationMs: number;
		defaultBackground: string;
		visualId: string | null;
		onOpenChange: (open: boolean) => void;
		onExport: (settings: AnimationExportSettings) => void | Promise<void>;
	}

	const props: Props = $props();
	const resolutionOptions = [
		{ value: '1280x720', labelKey: 'export.resolution.720p' },
		{ value: '1920x1080', labelKey: 'export.resolution.1080p' },
		{ value: '1080x1080', labelKey: 'export.resolution.square' },
		{ value: '1080x1350', labelKey: 'export.resolution.portrait' },
		{ value: '1080x1920', labelKey: 'export.resolution.vertical' },
		{ value: '2560x1440', labelKey: 'export.resolution.1440p' },
		{ value: '3840x2160', labelKey: 'export.resolution.4k' },
		{ value: 'custom', labelKey: 'export.resolution.custom' }
	] as const;
	const bitrateOptions = [
		{ value: 8_000_000, labelKey: 'export.bitrate.smaller' },
		{ value: 12_000_000, labelKey: 'export.bitrate.balanced' },
		{ value: 20_000_000, labelKey: 'export.bitrate.higherQuality' },
		{ value: -1, labelKey: 'export.customBitrate' }
	] as const;
	const backgroundOptions = [
		{ value: 'visual', labelKey: 'export.background.matchDiagram' },
		{ value: 'dark', labelKey: 'export.background.dark' },
		{ value: 'custom', labelKey: 'export.background.custom' }
	] as const;
	const frameRateOptions = [24, 30, 60] as const;

	let settingsTab = $state('video');
	let format = $state<AnimationFormat>('mp4');
	let background = $state<AnimationBackgroundChoice>('visual');
	let customColor = $state('#ffffff');
	let frameRate = $state<number>(30);
	let resolution = $state<(typeof resolutionOptions)[number]['value']>('1920x1080');
	let customWidth = $state('1920');
	let customHeight = $state('1080');
	let bitrate = $state<number>(12_000_000);
	let customBitrateMbps = $state('12');
	let checkingEncoder = $state(false);
	let formatSupported = $state(false);
	let exportError = $state('');
	let settingsLoadedFor = $state('');
	let currentCapabilityKey = $state('');

	const visualKey = $derived(props.visualId?.trim() || 'default');
	const durationMs = $derived(Math.max(0, props.durationMs));
	const dimensions = $derived.by(() => {
		if (resolution === 'custom') return { width: Number(customWidth), height: Number(customHeight) };
		const [width, height] = resolution.split('x').map(Number);
		return { width, height };
	});
	const outputBitrate = $derived(bitrate === -1 ? Math.round(Number(customBitrateMbps) * 1_000_000) : bitrate);
	const validDimensions = $derived(isValidAnimationExportDimensions(dimensions.width, dimensions.height));
	const validBitrate = $derived(isValidAnimationExportBitrate(outputBitrate));
	const validFrameRate = $derived(isValidAnimationExportFps(frameRate));
	const formatLabel = $derived(studioMessage(animationFormatOptions.find((option) => option.value === format)?.labelKey ?? 'export.format.mp4'));
	const bitrateLabel = $derived(studioMessage(bitrateOptions.find(({ value }) => value === bitrate)?.labelKey ?? 'export.customBitrate'));
	const directSaveAvailable = $derived(supportsDirectAnimationFileSave());
	const storagePlan = $derived(getAnimationExportStoragePlan(durationMs, validBitrate ? outputBitrate : 12_000_000, directSaveAvailable));
	const frameCount = $derived(Math.max(1, Math.ceil(durationMs / 1000 * (validFrameRate ? frameRate : 30))));
	const estimatedMegabytes = $derived(validBitrate ? ((durationMs / 1000) * outputBitrate / 8 / 1_000_000).toFixed(1) : '—');
	const estimatedBufferedMegabytes = $derived((storagePlan.estimatedBytes / 1_000_000).toFixed(1));
	const canRender = $derived(
		props.open && !props.busy && formatSupported && validDimensions && validBitrate && validFrameRate && storagePlan.canExport
	);

	$effect(() => {
		const key = visualKey;
		if (settingsLoadedFor === key) return;
		resetSettings(props.defaultBackground);
		readSettings(key);
		settingsLoadedFor = key;
	});

	$effect(() => {
		const key = visualKey;
		const values = JSON.stringify({ format, background, customColor, frameRate, resolution, customWidth, customHeight, bitrate, customBitrateMbps });
		if (settingsLoadedFor !== key) return;
		const timer = window.setTimeout(() => {
			try { localStorage.setItem(`schemra.animation-export.v1.${key}`, values); }
			catch { /* Settings remain available for this session. */ }
		}, 150);
		return () => window.clearTimeout(timer);
	});

	$effect(() => {
		const enabled = props.open && validDimensions && validBitrate && validFrameRate;
		const candidate: AnimationExportCapabilitySettings = {
			format,
			width: dimensions.width,
			height: dimensions.height,
			fps: validFrameRate ? frameRate as 24 | 30 | 60 : 30,
			videoBitsPerSecond: validBitrate ? outputBitrate : 12_000_000
		};
		const key = JSON.stringify(candidate);
		if (!enabled) {
			checkingEncoder = false;
			formatSupported = false;
			currentCapabilityKey = '';
			return;
		}
		let cancelled = false;
		currentCapabilityKey = key;
		checkingEncoder = true;
		formatSupported = false;
		const timer = window.setTimeout(() => {
			void supportsAnimationFormat(candidate).then(
				(supported) => { if (!cancelled && currentCapabilityKey === key) formatSupported = supported; },
				() => { if (!cancelled && currentCapabilityKey === key) formatSupported = false; }
			).finally(() => { if (!cancelled && currentCapabilityKey === key) checkingEncoder = false; });
		}, 150);
		return () => { cancelled = true; window.clearTimeout(timer); };
	});

	function resetSettings(defaultBackground: string) {
		format = 'mp4';
		background = 'visual';
		customColor = normalizeColor(defaultBackground);
		frameRate = 30;
		resolution = '1920x1080';
		customWidth = '1920';
		customHeight = '1080';
		bitrate = 12_000_000;
		customBitrateMbps = '12';
	}

	function readSettings(key: string) {
		try {
			const stored = localStorage.getItem(`schemra.animation-export.v1.${key}`);
			if (!stored) return;
			const value = JSON.parse(stored) as Record<string, unknown>;
			if (value.format === 'mp4' || value.format === 'webm' || value.format === 'mov') format = value.format;
			if (value.background === 'visual' || value.background === 'dark' || value.background === 'custom') background = value.background;
			if (typeof value.customColor === 'string') customColor = normalizeColor(value.customColor);
			if (frameRateOptions.includes(value.frameRate as (typeof frameRateOptions)[number])) frameRate = value.frameRate as number;
			if (resolutionOptions.some(({ value: option }) => option === value.resolution)) resolution = value.resolution as typeof resolution;
			if (typeof value.customWidth === 'string' && /^\d{3,4}$/.test(value.customWidth)) customWidth = value.customWidth;
			if (typeof value.customHeight === 'string' && /^\d{3,4}$/.test(value.customHeight)) customHeight = value.customHeight;
			if (typeof value.bitrate === 'number' && (value.bitrate === -1 || bitrateOptions.some(({ value: option }) => option === value.bitrate))) bitrate = value.bitrate;
			if (typeof value.customBitrateMbps === 'string') {
				const custom = Number(value.customBitrateMbps);
				if (Number.isFinite(custom) && custom >= 0.1 && custom <= 50) customBitrateMbps = value.customBitrateMbps;
			}
		} catch { /* Ignore unavailable or malformed settings. */ }
	}

	function normalizeColor(value: string) {
		const candidate = value.trim();
		if (/^#[\da-f]{6}$/i.test(candidate)) return candidate;
		if (/^#[\da-f]{3}$/i.test(candidate)) return `#${candidate[1]}${candidate[1]}${candidate[2]}${candidate[2]}${candidate[3]}${candidate[3]}`;
		return '#ffffff';
	}

	function handleOpenChange(open: boolean) {
		if (!open) requestClose();
	}

	function requestClose() {
		if (props.busy) cancelActiveAnimationExport();
		props.onOpenChange(false);
	}

	function handleInteractOutside(event: Event) {
		if (props.busy) event.preventDefault();
	}

	async function renderAnimation() {
		if (!canRender) return;
		exportError = '';
		const settings: AnimationExportSettings = {
			format,
			background: background === 'visual'
				? props.defaultBackground
				: background === 'dark'
					? '#18181b'
					: customColor,
			durationMs,
			fps: frameRate as 24 | 30 | 60,
			width: dimensions.width,
			height: dimensions.height,
			videoBitsPerSecond: outputBitrate
		};
		try {
			await props.onExport(settings);
		} catch (error) {
			if (!(error instanceof Error && error.name === 'AbortError')) {
				exportError = error instanceof Error ? error.message : studioMessage('export.renderFailed');
			}
		}
	}
</script>

<Dialog.Root open={props.open} onOpenChange={handleOpenChange}>
	<Dialog.Content class="max-h-[min(90vh,48rem)] w-full gap-5 overflow-y-auto p-5 sm:max-w-[35rem]" showCloseButton={false} onInteractOutside={handleInteractOutside}>
		<Dialog.Header class="gap-1.5 pr-8 text-left">
			<Dialog.Title class="text-base font-semibold leading-6">{studioMessage('export.title')}</Dialog.Title>
			<Dialog.Description class="text-xs leading-[1.45]">{studioMessage('export.description', { scenario: props.scenarioTitle, format: formatLabel, duration: (durationMs / 1000).toFixed(durationMs % 1000 === 0 ? 0 : 1) })}</Dialog.Description>
		</Dialog.Header>

		<Dialog.Close>
			{#snippet child({ props: closeProps })}
				<Button {...closeProps} variant="ghost" size="icon-sm" class="absolute top-3 right-3" aria-label={studioMessage('export.close')} disabled={props.busy}>
					<X size={16} aria-hidden="true" />
				</Button>
			{/snippet}
		</Dialog.Close>

		{#if props.busy}
			<div class="grid gap-1.5" role="status" aria-live="polite">
				<div class="flex justify-between text-xs">
					<span>{studioMessage('export.rendering', { format: formatLabel })}</span>
					<span class="tabular-nums">{Math.round(Math.min(1, Math.max(0, props.progress)) * 100)}%</span>
				</div>
				<div class="h-1.5 overflow-hidden rounded-full bg-muted" role="progressbar" aria-label={studioMessage('export.progress')} aria-valuemin="0" aria-valuemax="100" aria-valuenow={Math.round(Math.min(1, Math.max(0, props.progress)) * 100)}>
					<div class="h-full rounded-full bg-primary transition-[width] duration-150" style:width={`${Math.min(1, Math.max(0, props.progress)) * 100}%`}></div>
				</div>
			</div>
		{/if}

		<Tabs.Root bind:value={settingsTab} activationMode="manual">
			<Tabs.List class="grid w-full grid-cols-2 rounded-md bg-muted p-1" aria-label={studioMessage('export.settingsTabs')}>
				<Tabs.Trigger value="video" class="rounded-sm px-3 py-1.5 text-sm font-medium text-muted-foreground transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring data-[state=active]:bg-background data-[state=active]:text-foreground data-[state=active]:shadow-sm">
					{studioMessage('export.tab.video')}
				</Tabs.Trigger>
				<Tabs.Trigger value="background" class="rounded-sm px-3 py-1.5 text-sm font-medium text-muted-foreground transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring data-[state=active]:bg-background data-[state=active]:text-foreground data-[state=active]:shadow-sm">
					{studioMessage('export.tab.background')}
				</Tabs.Trigger>
			</Tabs.List>

			<Tabs.Content value="video" class="grid grid-cols-1 gap-4 pt-4 min-[521px]:grid-cols-2">
				<div class="grid min-w-0 content-start gap-2">
					<Label for="animation-export-format" class="text-xs font-semibold">{studioMessage('export.format')}</Label>
					<Select.Root type="single" value={format} onValueChange={(value) => { if (value === 'mp4' || value === 'webm' || value === 'mov') format = value; }} disabled={props.busy}>
						<Select.Trigger id="animation-export-format" class="w-full" aria-label={studioMessage('export.format')}>
							<span data-slot="select-value" class="min-w-0 flex-1 truncate">{formatLabel}</span>
						</Select.Trigger>
						<Select.Content class="!z-[1000] w-[var(--bits-select-anchor-width)]">
							<Select.Group>
								{#each animationFormatOptions as option (option.value)}
									<Select.Item value={option.value} label={studioMessage(option.labelKey)}>{studioMessage(option.labelKey)}</Select.Item>
								{/each}
							</Select.Group>
						</Select.Content>
					</Select.Root>
				</div>

				<div class="grid min-w-0 content-start gap-2">
					<Label for="animation-export-resolution" class="text-xs font-semibold">{studioMessage('export.outputSize')}</Label>
					<Select.Root type="single" bind:value={resolution} disabled={props.busy}>
						<Select.Trigger id="animation-export-resolution" class="w-full" aria-describedby={validDimensions ? undefined : 'animation-export-dimensions-note'}>
							<Select.Value />
						</Select.Trigger>
						<Select.Content class="!z-[1000] w-[var(--bits-select-anchor-width)]">
							<Select.Group>
								{#each resolutionOptions as option (option.value)}
									<Select.Item value={option.value} label={studioMessage(option.labelKey)}>{studioMessage(option.labelKey)}</Select.Item>
								{/each}
							</Select.Group>
						</Select.Content>
					</Select.Root>
					{#if !validDimensions}
						<small id="animation-export-dimensions-note" class="text-xs leading-5 text-destructive">{studioMessage('export.dimensionsInvalid')}</small>
					{/if}
				</div>

				{#if resolution === 'custom'}
					<div class="col-span-full grid grid-cols-2 gap-2.5">
						<Label for="animation-export-width" class="flex min-w-0 flex-col items-start gap-2 text-xs font-semibold">
							{studioMessage('export.width')}
							<Input id="animation-export-width" type="number" min="320" max="3840" step="1" bind:value={customWidth} disabled={props.busy} />
						</Label>
						<Label for="animation-export-height" class="flex min-w-0 flex-col items-start gap-2 text-xs font-semibold">
							{studioMessage('export.height')}
							<Input id="animation-export-height" type="number" min="180" max="3840" step="1" bind:value={customHeight} disabled={props.busy} />
						</Label>
					</div>
				{/if}

				<div class="grid min-w-0 content-start gap-2">
					<Label for="animation-export-frame-rate" class="text-xs font-semibold">{studioMessage('export.frameRate')}</Label>
					<Select.Root type="single" value={String(frameRate)} onValueChange={(value) => (frameRate = Number(value))} disabled={props.busy}>
						<Select.Trigger id="animation-export-frame-rate" class="w-full">
							<Select.Value />
						</Select.Trigger>
						<Select.Content class="!z-[1000] w-[var(--bits-select-anchor-width)]">
							<Select.Group>
								{#each frameRateOptions as fps (fps)}
									{@const label = studioMessage('export.fps', { fps })}
									<Select.Item value={String(fps)} {label}>{label}</Select.Item>
								{/each}
							</Select.Group>
						</Select.Content>
					</Select.Root>
				</div>

				<div class="grid min-w-0 content-start gap-2">
					<Label for="animation-export-bitrate" class="text-xs font-semibold">{studioMessage('export.videoBitrate')}</Label>
					<Select.Root type="single" value={String(bitrate)} onValueChange={(value) => (bitrate = Number(value))} disabled={props.busy}>
						<Select.Trigger id="animation-export-bitrate" class="w-full" aria-label={studioMessage('export.videoBitrate')}>
							<span data-slot="select-value" class="min-w-0 flex-1 truncate">{bitrateLabel}</span>
						</Select.Trigger>
						<Select.Content class="!z-[1000] w-[var(--bits-select-anchor-width)]">
							<Select.Group>
								{#each bitrateOptions as option (option.value)}
									<Select.Item value={String(option.value)} label={studioMessage(option.labelKey)}>{studioMessage(option.labelKey)}</Select.Item>
								{/each}
							</Select.Group>
						</Select.Content>
					</Select.Root>
				</div>

				{#if bitrate === -1}
					<div class="grid min-w-0 content-start gap-2">
						<Label for="animation-export-custom-bitrate" class="text-xs font-semibold">{studioMessage('export.customBitrateMbps')}</Label>
						<Input id="animation-export-custom-bitrate" type="number" min="0.1" max="50" step="0.1" bind:value={customBitrateMbps} disabled={props.busy} aria-invalid={!validBitrate} aria-describedby="animation-export-custom-bitrate-note" />
						<small id="animation-export-custom-bitrate-note" class="text-xs leading-5 text-muted-foreground">{studioMessage('export.customBitrateHelp')}</small>
					</div>
				{/if}
			</Tabs.Content>

			<Tabs.Content value="background" class="grid gap-4 pt-4">
				<fieldset class="m-0 min-w-0 border-0 p-0">
					<legend class="mb-2 block text-xs font-semibold">{studioMessage('export.background')}</legend>
					<div class="grid grid-cols-3 gap-1.5" role="group" aria-label={studioMessage('export.background')}>
						{#each backgroundOptions as option (option.value)}
							<Button
								variant={background === option.value ? 'default' : 'outline'}
								class="h-auto min-h-9 whitespace-normal px-2 py-1 text-xs"
								aria-pressed={background === option.value}
								disabled={props.busy}
								onclick={() => (background = option.value)}
							>
								{studioMessage(option.labelKey)}
							</Button>
						{/each}
					</div>
					{#if background === 'custom'}
						<div class="mt-2 flex items-center justify-between gap-2">
							<Label for="animation-background-color" class="text-xs font-medium">{studioMessage('export.backgroundColor')}</Label>
							<Input id="animation-background-color" type="color" bind:value={customColor} disabled={props.busy} class="h-8 w-10 cursor-pointer p-1" />
						</div>
					{/if}
				</fieldset>
			</Tabs.Content>
		</Tabs.Root>

		<p class="sr-only" role="status" aria-live="polite">
			{#if checkingEncoder}
				{studioMessage('export.encoderChecking', { format: formatLabel })}
			{:else if !formatSupported}
				{studioMessage('export.encoderUnavailable', { format: formatLabel })}
			{/if}
		</p>

		{#if exportError}<p class="m-0 rounded-md border border-destructive/30 bg-destructive/5 px-3 py-2 text-sm leading-5 text-destructive" role="alert">{exportError}</p>{/if}
		{#if validBitrate && !storagePlan.canExport}
			<p class="m-0 rounded-md border border-destructive/30 bg-destructive/5 px-3 py-2 text-sm leading-5 text-destructive" role="alert">{studioMessage('export.memoryLimit', { megabytes: estimatedBufferedMegabytes })}</p>
		{/if}

		<Dialog.Footer class="mt-1 flex-row flex-wrap items-center justify-between">
			<p class="m-0 text-xs tabular-nums text-muted-foreground" role="status" aria-live="polite">{studioMessage('export.frameEstimate', { frames: frameCount.toLocaleString(), megabytes: estimatedMegabytes })}{#if directSaveAvailable && storagePlan.requiresDirectSave}{studioMessage('export.savedDirectly')}{/if}</p>
			<div class="ml-auto flex justify-end gap-2">
				<Dialog.Close>
					{#snippet child({ props: cancelProps })}
						<Button {...cancelProps} variant="outline">{props.busy ? studioMessage('export.cancelRender') : studioMessage('export.cancel')}</Button>
					{/snippet}
				</Dialog.Close>
				<Button disabled={!canRender} onclick={() => void renderAnimation()}>{props.busy ? studioMessage('export.rendering', { format: formatLabel }) : studioMessage('export.renderFormat', { format: formatLabel })}</Button>
			</div>
		</Dialog.Footer>
	</Dialog.Content>
</Dialog.Root>
