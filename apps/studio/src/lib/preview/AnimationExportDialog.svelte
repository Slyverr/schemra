<script lang="ts">
	import {
		cancelActiveAnimationExport,
		getAnimationExportStoragePlan,
		isValidAnimationExportBitrate,
		isValidAnimationExportDimensions,
		isValidAnimationExportFps,
		supportsAnimationMp4,
		supportsDirectAnimationFileSave,
		type AnimationExportCapabilitySettings
	} from './animation-export.js';

	export type AnimationBackgroundChoice = 'visual' | 'dark' | 'custom';
	export interface AnimationExportSettings {
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
		{ value: '1280x720', label: '720p · 1280 × 720' },
		{ value: '1920x1080', label: '1080p · 1920 × 1080' },
		{ value: '1080x1080', label: 'Square · 1080 × 1080' },
		{ value: '1080x1350', label: 'Portrait · 1080 × 1350' },
		{ value: '1080x1920', label: 'Vertical · 1080 × 1920' },
		{ value: '2560x1440', label: '1440p · 2560 × 1440' },
		{ value: '3840x2160', label: '4K UHD · 3840 × 2160' },
		{ value: 'custom', label: 'Custom dimensions' }
	] as const;
	const bitrateOptions = [
		{ value: 8_000_000, label: 'Smaller · 8 Mbps' },
		{ value: 12_000_000, label: 'Balanced · 12 Mbps' },
		{ value: 20_000_000, label: 'Higher quality · 20 Mbps' },
		{ value: -1, label: 'Custom bitrate' }
	] as const;
	const frameRateOptions = [24, 30, 60] as const;

	let dialog = $state<HTMLDialogElement>();
	let background = $state<AnimationBackgroundChoice>('visual');
	let customColor = $state('#ffffff');
	let frameRate = $state<number>(30);
	let resolution = $state<(typeof resolutionOptions)[number]['value']>('1920x1080');
	let customWidth = $state('1920');
	let customHeight = $state('1080');
	let bitrate = $state<number>(12_000_000);
	let customBitrateMbps = $state('12');
	let checkingEncoder = $state(false);
	let mp4Supported = $state(false);
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
	const directSaveAvailable = $derived(supportsDirectAnimationFileSave());
	const storagePlan = $derived(getAnimationExportStoragePlan(durationMs, validBitrate ? outputBitrate : 12_000_000, directSaveAvailable));
	const frameCount = $derived(Math.max(1, Math.ceil(durationMs / 1000 * (validFrameRate ? frameRate : 30))));
	const estimatedMegabytes = $derived(validBitrate ? ((durationMs / 1000) * outputBitrate / 8 / 1_000_000).toFixed(1) : '—');
	const estimatedBufferedMegabytes = $derived((storagePlan.estimatedBytes / 1_000_000).toFixed(1));
	const canRender = $derived(
		props.open && !props.busy && mp4Supported && validDimensions && validBitrate && validFrameRate && storagePlan.canExport
	);

	$effect(() => {
		const element = dialog;
		if (!element) return;
		if (props.open && !element.open) element.showModal();
		else if (!props.open && element.open) element.close();
	});

	$effect(() => {
		const key = visualKey;
		if (settingsLoadedFor === key) return;
		resetSettings(props.defaultBackground);
		readSettings(key);
		settingsLoadedFor = key;
	});

	$effect(() => {
		const key = visualKey;
		const values = JSON.stringify({ background, customColor, frameRate, resolution, customWidth, customHeight, bitrate, customBitrateMbps });
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
			width: dimensions.width,
			height: dimensions.height,
			fps: validFrameRate ? frameRate as 24 | 30 | 60 : 30,
			videoBitsPerSecond: validBitrate ? outputBitrate : 12_000_000
		};
		const key = JSON.stringify(candidate);
		if (!enabled) {
			checkingEncoder = false;
			mp4Supported = false;
			currentCapabilityKey = '';
			return;
		}
		let cancelled = false;
		currentCapabilityKey = key;
		checkingEncoder = true;
		mp4Supported = false;
		const timer = window.setTimeout(() => {
			void supportsAnimationMp4(candidate).then(
				(supported) => { if (!cancelled && currentCapabilityKey === key) mp4Supported = supported; },
				() => { if (!cancelled && currentCapabilityKey === key) mp4Supported = false; }
			).finally(() => { if (!cancelled && currentCapabilityKey === key) checkingEncoder = false; });
		}, 150);
		return () => { cancelled = true; window.clearTimeout(timer); };
	});

	function resetSettings(defaultBackground: string) {
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

	function handleCancel(event: Event) {
		event.preventDefault();
		requestClose();
	}

	function requestClose() {
		if (props.busy) cancelActiveAnimationExport();
		props.onOpenChange(false);
	}

	function handleBackdropClick(event: MouseEvent) {
		if (event.target === dialog && !props.busy) requestClose();
	}

	async function renderAnimation() {
		if (!canRender) return;
		exportError = '';
		const settings: AnimationExportSettings = {
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
				exportError = error instanceof Error ? error.message : 'The animation could not be rendered. Try again.';
			}
		}
	}
</script>

<dialog
	bind:this={dialog}
	class="animation-export-dialog"
	aria-labelledby="animation-export-title"
	aria-describedby="animation-export-description"
	oncancel={handleCancel}
	onclick={handleBackdropClick}
>
	<header class="dialog-heading">
		<div>
			<h2 id="animation-export-title">Export animation</h2>
			<p id="animation-export-description">Render “{props.scenarioTitle}” as an MP4 video · {(durationMs / 1000).toFixed(durationMs % 1000 === 0 ? 0 : 1)} s.</p>
		</div>
		<button type="button" class="close-button" aria-label="Close animation export" disabled={props.busy} onclick={requestClose}>×</button>
	</header>

	{#if props.busy}
		<div class="progress-group">
			<div class="progress-label"><span role="status" aria-live="polite">Rendering MP4</span><span>{Math.round(Math.min(1, Math.max(0, props.progress)) * 100)}%</span></div>
			<div class="progress-track" role="progressbar" aria-label="Animation export progress" aria-valuemin="0" aria-valuemax="100" aria-valuenow={Math.round(Math.min(1, Math.max(0, props.progress)) * 100)}>
				<div class="progress-value" style={`width:${Math.min(1, Math.max(0, props.progress)) * 100}%`}></div>
			</div>
		</div>
	{/if}

	<div class="settings-grid">
		<fieldset>
			<legend>Background</legend>
			<div class="background-options">
				{#each [
					{ value: 'visual', label: 'Match diagram' },
					{ value: 'dark', label: 'Dark' },
					{ value: 'custom', label: 'Custom' }
				] as option (option.value)}
					<label class:chosen={background === option.value}>
						<input type="radio" name="animation-background" value={option.value} bind:group={background} disabled={props.busy} />
						<span>{option.label}</span>
					</label>
				{/each}
			</div>
			{#if background === 'custom'}
				<label class="field color-field"><span>Background color</span><input type="color" bind:value={customColor} disabled={props.busy} /></label>
			{/if}
		</fieldset>

		<label class="field">
			<span>Output size</span>
			<select bind:value={resolution} disabled={props.busy}>
				{#each resolutionOptions as option (option.value)}<option value={option.value}>{option.label}</option>{/each}
			</select>
			<small>{#if validDimensions}{dimensions.width} × {dimensions.height} · {(dimensions.width * dimensions.height / 1_000_000).toFixed(1)} megapixels. Diagram stays centered and uncropped.{:else}Choose at least 320 × 180, no side above 3840, and no more than 8.3 megapixels.{/if}</small>
		</label>

		{#if resolution === 'custom'}
			<div class="field-row">
				<label class="field"><span>Width</span><input type="number" min="320" max="3840" step="1" bind:value={customWidth} disabled={props.busy} /></label>
				<label class="field"><span>Height</span><input type="number" min="180" max="3840" step="1" bind:value={customHeight} disabled={props.busy} /></label>
			</div>
		{/if}

		<label class="field">
			<span>Frame rate</span>
			<select bind:value={frameRate} disabled={props.busy}>
				{#each frameRateOptions as fps (fps)}<option value={fps}>{fps} FPS</option>{/each}
			</select>
			<small>Controls playback smoothness.</small>
		</label>

		<label class="field">
			<span>Video bitrate</span>
			<select bind:value={bitrate} disabled={props.busy}>
				{#each bitrateOptions as option (option.value)}<option value={option.value}>{option.label}</option>{/each}
			</select>
		</label>

		{#if bitrate === -1}
			<label class="field"><span>Custom bitrate (Mbps)</span><input type="number" min="0.1" max="50" step="0.1" bind:value={customBitrateMbps} disabled={props.busy} aria-invalid={!validBitrate} /><small>Enter a value from 0.1 to 50 Mbps.</small></label>
		{/if}
	</div>

	{#if checkingEncoder}
		<p class="status" role="status">Checking MP4 encoder for these settings…</p>
	{:else if mp4Supported}
		<p class="status">MP4 encoding is available for these settings.</p>
	{:else}
		<p class="status warning" role="status">No frame-accurate MP4 encoder is available for these settings. Try a smaller size or frame rate.</p>
	{/if}

	{#if exportError}<p class="error" role="alert">{exportError}</p>{/if}
	{#if validBitrate && !storagePlan.canExport}
		<p class="error" role="alert">The estimated video is about {estimatedBufferedMegabytes} MB, above the 32 MiB memory limit. Lower the duration or bitrate, or use a browser that can save directly to a file.</p>
	{/if}

	<footer>
		<p role="status" aria-live="polite">~{frameCount.toLocaleString()} frames · ~{estimatedMegabytes} MB{#if directSaveAvailable && storagePlan.requiresDirectSave} · saved directly to file{/if}</p>
		<div class="actions">
			<button type="button" class="secondary" onclick={requestClose}>{props.busy ? 'Cancel render' : 'Cancel'}</button>
			<button type="button" class="primary" disabled={!canRender} onclick={() => void renderAnimation()}>{props.busy ? 'Rendering…' : 'Render MP4'}</button>
		</div>
	</footer>
</dialog>

<style>
	dialog.animation-export-dialog { width: min(35rem, calc(100vw - 2rem)); max-height: min(90vh, 48rem); overflow: auto; margin: auto; padding: 1.25rem; border: 1px solid var(--border); border-radius: .85rem; color: var(--foreground); background: var(--background); box-shadow: 0 1rem 3rem rgb(7 21 41 / 24%); font: inherit; }
	dialog.animation-export-dialog::backdrop { background: rgb(15 23 42 / 42%); backdrop-filter: blur(2px); }
	.dialog-heading { display: flex; align-items: flex-start; justify-content: space-between; gap: 1rem; margin-bottom: 1.25rem; }
	.dialog-heading h2 { margin: 0; font-size: 1.05rem; font-weight: 650; }
	.dialog-heading p { margin: .25rem 0 0; color: var(--muted-foreground); font-size: .78rem; line-height: 1.45; }
	.close-button { min-width: 2rem; min-height: 2rem; border: 0; border-radius: .4rem; color: var(--muted-foreground); background: transparent; font-size: 1.3rem; cursor: pointer; }
	.settings-grid { display: grid; grid-template-columns: repeat(2,minmax(0,1fr)); gap: 1rem; }
	fieldset { min-width: 0; margin: 0; padding: 0; border: 0; }
	legend,.field > span { display: block; margin-bottom: .45rem; font-size: .78rem; font-weight: 600; }
	.background-options { display: grid; grid-template-columns: repeat(3,minmax(0,1fr)); gap: .35rem; }
	.background-options label { display: flex; min-height: 2.3rem; align-items: center; justify-content: center; gap: .3rem; padding: .35rem; border: 1px solid var(--border); border-radius: .45rem; font-size: .7rem; cursor: pointer; }
	.background-options label.chosen { border-color: var(--ring); background: color-mix(in srgb,var(--accent) 55%,transparent); }
	.background-options input { accent-color: var(--primary); }
	.field { display: block; min-width: 0; }
	.field select,.field input:not([type="color"]) { width: 100%; min-height: 2.35rem; padding: .4rem .55rem; border: 1px solid var(--border); border-radius: .45rem; color: var(--foreground); background: var(--background); font: inherit; font-size: .76rem; }
	.color-field { display: flex; align-items: center; justify-content: space-between; gap: .6rem; margin-top: .6rem; }
	.color-field span { margin: 0; }
	.color-field input { width: 2.5rem; height: 2rem; padding: .15rem; border: 1px solid var(--border); border-radius: .35rem; background: var(--background); }
	.field small { display: block; margin-top: .35rem; color: var(--muted-foreground); font-size: .68rem; line-height: 1.4; }
	.field-row { display: grid; grid-template-columns: repeat(2,minmax(0,1fr)); gap: .65rem; }
	.status,.error { margin: 1rem 0 0; padding: .55rem .7rem; border: 1px solid var(--border); border-radius: .45rem; color: var(--muted-foreground); font-size: .72rem; line-height: 1.4; }
	.status.warning { color: var(--warning, var(--destructive)); }
	.error { border-color: color-mix(in srgb,var(--destructive) 30%,transparent); color: var(--destructive); background: color-mix(in srgb,var(--destructive) 5%,transparent); }
	.progress-group { margin: 0 0 1rem; }
	.progress-label { display: flex; justify-content: space-between; font-size: .72rem; }
	.progress-track { height: .4rem; overflow: hidden; margin-top: .4rem; border-radius: 1rem; background: var(--muted); }
	.progress-value { height: 100%; border-radius: inherit; background: var(--primary); transition: width .15s linear; }
	footer { display: flex; flex-wrap: wrap; align-items: center; justify-content: space-between; gap: .75rem; margin-top: 1.15rem; }
	footer p { margin: 0; color: var(--muted-foreground); font-size: .7rem; font-variant-numeric: tabular-nums; }
	.actions { display: flex; justify-content: flex-end; gap: .5rem; margin-left: auto; }
	button { min-height: 2.2rem; padding: .4rem .75rem; border: 1px solid var(--border); border-radius: .45rem; color: var(--foreground); background: var(--background); font: inherit; font-size: .75rem; cursor: pointer; }
	button.primary { border-color: var(--primary); color: var(--primary-foreground); background: var(--primary); }
	button:hover:not(:disabled) { filter: brightness(.97); }
	button:disabled { cursor: not-allowed; opacity: .52; }
	button:focus-visible,input:focus-visible,select:focus-visible { outline: 2px solid var(--ring); outline-offset: 2px; }
	@media (max-width: 520px) { dialog.animation-export-dialog { padding: 1rem; } .settings-grid { grid-template-columns: minmax(0,1fr); } }
</style>
