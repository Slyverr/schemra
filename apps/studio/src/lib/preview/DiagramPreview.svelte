
<script lang="ts">
	import { onDestroy, onMount } from 'svelte';
	import { studioMessage } from '../i18n/messages.js';
	import { cn } from '../utils.js';
	import { createScenarioPlayback, type ProjectSource, type ScenarioPlayback } from '../schemra';
	import {
		createDiagramPngBlob,
		createDiagramSvgBlob,
		diagramExportFileName,
		downloadDiagramBlob,
		serializeDiagramSvg,
		type DiagramExportFormat
	} from './export';
	import { printDiagramPdf } from './pdf-export.js';
	import AnimationExportDialog, { type AnimationExportSettings } from './AnimationExportDialog.svelte';
	import { cancelActiveAnimationExport, exportAnimation } from './animation-export.js';
	import { applyPreviewFrame, createAnimationFrameRenderer, type AnimationFrameRenderer } from './frame-renderer.js';
	import { calculateSceneFocusTransform, clampPreviewZoom, clampScenePan, getPreviewConnectionBounds, insetPreviewViewport, previewGridTransform, zoomPanAtPoint } from './viewport';
	import PlaybackTimeline from './PlaybackTimeline.svelte';
	import PreviewToolbar from './PreviewToolbar.svelte';
	import {
		previewTimeAtElapsed,
		readPreviewFrame,
		readPreviewScenario,
		readPreviewScene,
		sanitizeSvgMarkup,
		type PreviewFrame,
		type PreviewScene
	} from './model';

	interface Props {
		svg: string | null;
		scene: unknown | null;
		scenario: unknown | null;
		sources: ProjectSource[];
		visualId: string | null;
		scenarios?: readonly { id: string; title: string }[];
		stale?: boolean;
		onScenarioSelect?: (scenarioId: string | null) => void;
	}

	const props: Props = $props();
	onDestroy(() => cancelActiveAnimationExport());
	const componentId = $props.id();
	const previewCanvasId = `${componentId}-canvas`;
	let canvas = $state<HTMLDivElement | undefined>();
	let previewRoot = $state<HTMLElement | undefined>();
	let svgHost = $state<HTMLDivElement | undefined>();
	let viewport = $state({ width: 0, height: 0 });
	let controlInsets = $state({ top: 0, bottom: 0 });
	let safeSvg = $state('');
	let playback = $state<ScenarioPlayback | null>(null);
	let playbackError = $state<string | null>(null);
	let frame = $state<PreviewFrame | null>(null);
	let playing = $state(false);
	let timeMs = $state(0);
	let zoomPercent = $state(100);
	let pan = $state({ x: 0, y: 0 });
	let exporting = $state<DiagramExportFormat | null>(null);
	let exportError = $state<string | null>(null);
	let animationExportOpen = $state(false);
	let animationExportBusy = $state(false);
	let animationExportProgress = $state(0);
	let presentationMode = $state(false);
	let presentationError = $state<string | null>(null);
	let playbackStartedAt = 0;
	const scene = $derived(readPreviewScene(props.scene));
	const scenario = $derived(readPreviewScenario(props.scenario));
	const durationMs = $derived(playback?.durationMs ?? scenario?.duration_ms ?? 0);
	const contentViewport = $derived(insetPreviewViewport(viewport, controlInsets));
	const displaySize = $derived(svgDisplaySize(contentViewport, scene));
	const gridTransform = $derived(previewGridTransform(viewport, pan, zoomPercent));
	const errorOverlayClasses = cn(
		'absolute left-3 top-[3.75rem] z-20 m-0 max-w-[calc(100%_-_1.5rem)]',
		'rounded-[.4rem] border border-destructive bg-background px-3 py-2 text-xs text-destructive',
		'[overflow-wrap:anywhere]'
	);
	const canExportAnimation = $derived(Boolean(!(props.stale ?? false) && safeSvg && playback && durationMs > 0 && !animationExportBusy));
	let dragStart = $state<{ pointerId: number; x: number; y: number; panX: number; panY: number } | null>(null);

	onMount(() => {
		const syncPresentationMode = () => {
			presentationMode = document.fullscreenElement === previewRoot;
		};
		document.addEventListener('fullscreenchange', syncPresentationMode);
		return () => document.removeEventListener('fullscreenchange', syncPresentationMode);
	});

	$effect(() => {
		const markup = props.svg;
		try {
			safeSvg = markup ? sanitizeSvgMarkup(markup, undefined, undefined, { omitCanvasBackground: true }) : '';
		} catch {
			safeSvg = '';
		}
	});

	$effect(() => {
		const stage = canvas;
		const root = previewRoot;
		const currentScene = scene;
		if (!stage || !root || typeof ResizeObserver === 'undefined') return;

		let measurementFrame = 0;
		const measure = () => {
			const nextViewport = { width: stage.clientWidth, height: stage.clientHeight };
			const canvasBounds = stage.getBoundingClientRect();
			const toolbar = root.querySelector<HTMLElement>('.preview-toolbar');
			const timeline = root.querySelector<HTMLElement>('.timeline');
			const toolbarBounds = toolbar?.getBoundingClientRect();
			const timelineBounds = timeline?.getBoundingClientRect();
			const nextInsets = {
				top: toolbarBounds ? Math.max(0, toolbarBounds.bottom - canvasBounds.top) : 0,
				bottom: timelineBounds ? Math.max(0, canvasBounds.bottom - timelineBounds.top) : 0
			};
			const previousContent = insetPreviewViewport(viewport, controlInsets);
			const nextContent = insetPreviewViewport(nextViewport, nextInsets);
			const centerShift = (nextContent.centerOffsetY ?? 0) - (previousContent.centerOffsetY ?? 0);

			viewport = nextViewport;
			controlInsets = nextInsets;
			if (currentScene) {
				const nextPan = zoomPercent <= 100
					? { x: 0, y: nextContent.centerOffsetY ?? 0 }
					: { x: pan.x, y: pan.y + centerShift };
				pan = clampScenePan(nextPan, nextContent, currentScene, zoomPercent);
			}
		};
		const scheduleMeasure = () => {
			cancelAnimationFrame(measurementFrame);
			measurementFrame = requestAnimationFrame(measure);
		};
		const resizeObserver = new ResizeObserver(scheduleMeasure);
		const observeControls = () => {
			resizeObserver.observe(stage);
			resizeObserver.observe(root);
			for (const control of root.querySelectorAll<HTMLElement>('.preview-toolbar, .timeline')) {
				resizeObserver.observe(control);
			}
			scheduleMeasure();
		};
		const mutationObserver = new MutationObserver(observeControls);
		mutationObserver.observe(root, { childList: true });
		observeControls();

		return () => {
			cancelAnimationFrame(measurementFrame);
			mutationObserver.disconnect();
			resizeObserver.disconnect();
		};
	});

	$effect(() => {
		if ((props.stale ?? false) && animationExportOpen) animationExportOpen = false;
	});

	$effect(() => {
		const sourceSnapshot = props.sources.map(({ id, path, source }) => ({ id, path, source }));
		const targetVisualId = props.visualId;
		const selectedScenario = scenario;
		let cancelled = false;
		let ownedPlayback: ScenarioPlayback | null = null;
		playback = null;
		playbackError = null;
		frame = null;
		playing = false;
		timeMs = 0;

		if (!selectedScenario || !targetVisualId) return;
		void createScenarioPlayback(sourceSnapshot, targetVisualId, selectedScenario.id).then(
			(sampler) => {
				if (cancelled) sampler.dispose();
				else {
					ownedPlayback = sampler;
					playback = sampler;
				}
			},
			(error: unknown) => {
				if (!cancelled) playbackError = error instanceof Error ? error.message : studioMessage('preview.playbackInitializeFailed');
			}
		);
		return () => {
			cancelled = true;
			ownedPlayback?.dispose();
		};
	});

	$effect(() => {
		const sampler = playback;
		const sampleTime = timeMs;
		if (!sampler) {
			frame = null;
			return;
		}
		try {
			frame = readPreviewFrame(sampler.sample(sampleTime));
		} catch (error) {
			playing = false;
			playbackError = error instanceof Error ? error.message : studioMessage('preview.playbackFrameFailed');
			frame = null;
		}
	});

	$effect(() => {
		const markup = safeSvg;
		const currentFrame = frame;
		const currentScene = scene;
		const host = svgHost;
		if (!markup || !host) return;
		const root = host.querySelector('svg');
		if (!(root instanceof SVGSVGElement)) return;
		applyPreviewFrame(root, currentFrame, currentScene, scenario);
	});

	$effect(() => {
		const isPlaying = playing;
		const duration = durationMs;
		if (!isPlaying || duration <= 0) return;
		let animationFrame = 0;
		const tick = (timestamp: number) => {
			timeMs = previewTimeAtElapsed(timestamp - playbackStartedAt, duration);
			if (timeMs >= duration) {
				playing = false;
				return;
			}
			animationFrame = requestAnimationFrame(tick);
		};
		animationFrame = requestAnimationFrame(tick);
		return () => cancelAnimationFrame(animationFrame);
	});

	function togglePlayback() {
		if (!playback || durationMs <= 0) return;
		if (playing) {
			playing = false;
			return;
		}
		if (timeMs >= durationMs) timeMs = 0;
		playbackStartedAt = performance.now() - timeMs;
		playing = true;
	}

	function resetPlayback() {
		playing = false;
		timeMs = 0;
		playbackStartedAt = 0;
	}

	function seek(nextTimeMs: number) {
		timeMs = nextTimeMs;
	}

	function pauseOnTimelineInput(event: KeyboardEvent) {
		if (['ArrowDown', 'ArrowLeft', 'ArrowRight', 'ArrowUp', 'End', 'Home', 'PageDown', 'PageUp'].includes(event.key)) {
			playing = false;
		}
	}

	function setZoom(value: number, anchor?: { x: number; y: number }) {
		const nextZoom = clampPreviewZoom(value);
		if (nextZoom === zoomPercent) return;
		const previousZoom = zoomPercent;
		zoomPercent = nextZoom;
		if (nextZoom <= 100) {
			pan = { x: 0, y: contentViewport.centerOffsetY ?? 0 };
			return;
		}
		if (anchor) pan = zoomPanAtPoint(pan, previousZoom, nextZoom, anchor, contentViewport);
		clampPan();
	}

	function zoomOnWheel(event: WheelEvent) {
		const nextZoom = clampPreviewZoom(Math.round(zoomPercent - event.deltaY * 0.1));
		if (nextZoom === zoomPercent || !canvas) return;
		event.preventDefault();
		const bounds = canvas.getBoundingClientRect();
		setZoom(nextZoom, { x: event.clientX - bounds.left, y: event.clientY - bounds.top });
	}

	function startPan(event: PointerEvent) {
		if (zoomPercent <= 100 || event.button !== 0 || !event.isPrimary) return;
		dragStart = { pointerId: event.pointerId, x: event.clientX, y: event.clientY, panX: pan.x, panY: pan.y };
		(event.currentTarget as HTMLDivElement).setPointerCapture(event.pointerId);
	}

	function movePan(event: PointerEvent) {
		if (!dragStart || event.pointerId !== dragStart.pointerId) return;
		pan = { x: dragStart.panX + event.clientX - dragStart.x, y: dragStart.panY + event.clientY - dragStart.y };
		clampPan();
	}

	function endPan(event: PointerEvent) {
		if (!dragStart || event.pointerId !== dragStart.pointerId) return;
		dragStart = null;
	}

	function panWithKeyboard(event: KeyboardEvent) {
		if (zoomPercent <= 100) return;
		const distance = event.shiftKey ? 96 : 40;
		if (event.key === 'ArrowLeft') pan.x -= distance;
		else if (event.key === 'ArrowRight') pan.x += distance;
		else if (event.key === 'ArrowUp') pan.y -= distance;
		else if (event.key === 'ArrowDown') pan.y += distance;
		else if (event.key === 'Home') {
			fitDiagram();
			event.preventDefault();
			return;
		} else return;
		event.preventDefault();
		clampPan();
	}

	function clampPan() {
		if (!scene) return;
		pan = clampScenePan(pan, contentViewport, scene, zoomPercent);
	}

	async function togglePresentationMode() {
		if (!previewRoot) return;
		presentationError = null;
		try {
			if (document.fullscreenElement === previewRoot) await document.exitFullscreen();
			else await previewRoot.requestFullscreen();
		} catch (error) {
			presentationError = error instanceof Error ? error.message : studioMessage('preview.presentationError');
		}
	}

	function handleTargetFocus(bounds: PreviewScene['nodes'][number]['bounds']) {
		focusBounds(bounds);
	}

	function handleConnectionFocus(edgeIndex: number) {
		if (!scene) return;
		const bounds = getPreviewConnectionBounds(scene, edgeIndex);
		if (!bounds) return;
		focusBounds(bounds);
	}

	function fitDiagram() {
		zoomPercent = 100;
		pan = { x: 0, y: contentViewport.centerOffsetY ?? 0 };
	}

	function focusNode(nodeId: string) {
		const currentScene = scene;
		const node = currentScene?.nodes.find(({ id }) => id === nodeId);
		if (!node) return;
		focusBounds(node.bounds);
	}

	function focusBounds(bounds: PreviewScene['nodes'][number]['bounds']) {
		const currentScene = scene;
		if (!canvas || !currentScene) return;
		const transform = calculateSceneFocusTransform(
			contentViewport,
			{ width: currentScene.width, height: currentScene.height },
			bounds
		);
		if (!transform) return;
		zoomPercent = transform.zoomPercent;
		pan = { x: transform.x, y: transform.y };
		clampPan();
	}

	function openAnimationExport() {
		if (!canExportAnimation) return;
		animationExportProgress = 0;
		animationExportOpen = true;
	}

	export async function exportDiagram(format: DiagramExportFormat) {
		const root = svgHost?.querySelector('svg');
		if (!root) {
			exportError = studioMessage('preview.exportDiagramNotReady');
			return;
		}
		exporting = format;
		exportError = null;
		try {
			const name = scenario?.title || root.querySelector('title')?.textContent?.trim() || props.visualId || 'Schemra diagram';
			if (format === 'pdf') {
				await printDiagramPdf(serializeDiagramSvg(root), name);
			} else {
				const fileName = diagramExportFileName(name, format);
				const blob = format === 'svg' ? createDiagramSvgBlob(root) : await createDiagramPngBlob(root);
				downloadDiagramBlob(blob, fileName);
			}
		} catch (error) {
			exportError = error instanceof Error ? error.message : studioMessage('preview.exportDiagramFailed');
		} finally {
			exporting = null;
		}
	}

	function renderAnimation(settings: AnimationExportSettings): Promise<void> {
		const currentScene = scene;
		const currentScenario = scenario;
		const sampler = playback;
		if (!canExportAnimation || !safeSvg || !currentScene || !currentScenario || !sampler || durationMs <= 0) {
			return Promise.reject(new Error(studioMessage('preview.animationDurationRequired')));
		}
		let renderFrame: AnimationFrameRenderer;
		try {
			renderFrame = createAnimationFrameRenderer(safeSvg, currentScene, currentScenario, sampler, svgHost ?? document.documentElement);
		} catch (error) {
			return Promise.reject(error instanceof Error ? error : new Error(studioMessage('preview.animationPrepareFailed')));
		}
		animationExportBusy = true;
		animationExportProgress = 0;
		const job = exportAnimation({
			format: settings.format,
			durationMs: settings.durationMs,
			width: settings.width,
			height: settings.height,
			fps: settings.fps,
			videoBitsPerSecond: settings.videoBitsPerSecond,
			background: settings.background,
			fileName: currentScenario.title,
			renderFrame,
			onProgress: (progress) => (animationExportProgress = progress)
		});
		return job.then(() => {
			animationExportOpen = false;
		}).finally(() => {
			renderFrame.dispose();
			animationExportBusy = false;
		});
	}

	function getDiagramBackground(markup: string, currentScene: PreviewScene | null): string {
		if (!markup || !currentScene || typeof DOMParser === 'undefined') return '#ffffff';
		const parsed = new DOMParser().parseFromString(markup, 'image/svg+xml');
		if (parsed.querySelector('parsererror')) return '#ffffff';
		const background = [...parsed.documentElement.children].find((element) =>
			element.localName === 'rect' &&
			Number(element.getAttribute('width')) === currentScene.width &&
			Number(element.getAttribute('height')) === currentScene.height
		);
		return background?.getAttribute('fill') || '#ffffff';
	}

	function svgDisplaySize(size: { width: number; height: number }, scene: PreviewScene | null) {
		if (!scene || size.width <= 0 || size.height <= 0) return { width: '100%', height: '100%' };
		const scale = Math.min(size.width / scene.width, size.height / scene.height);
		return {
			width: `${Math.max(1, scene.width * scale)}px`,
			height: `${Math.max(1, scene.height * scale)}px`
		};
	}

</script>

<section
	class="relative flex h-full min-h-0 min-w-0 flex-[1_1_auto] overflow-hidden bg-muted text-foreground fullscreen:h-screen fullscreen:w-screen fullscreen:min-h-0 fullscreen:bg-muted"
	bind:this={previewRoot}
	aria-label={studioMessage('pane.previewPanel')}
>
	{#if safeSvg}
		<PreviewToolbar
			scene={scene}
			scenario={scenario}
			scenarios={props.scenarios ?? []}
			{zoomPercent}
			{presentationMode}
			canExportAnimation={canExportAnimation}
			{animationExportBusy}
			onScenarioSelect={props.onScenarioSelect}
			onFocusTarget={handleTargetFocus}
			onFocusNode={focusNode}
			onFocusConnection={handleConnectionFocus}
			onZoom={(value) => setZoom(value)}
			onFit={fitDiagram}
			onTogglePresentation={() => void togglePresentationMode()}
			onExportAnimation={openAnimationExport}
		/>
	{/if}

	<!-- The canvas supports wheel zoom, pointer panning, and keyboard panning. -->
	<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
	<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
	<div
		id={previewCanvasId}
		class={cn(
			'absolute inset-0 overflow-hidden outline-none select-none',
			zoomPercent > 100 && !dragStart ? 'cursor-grab' : '',
			dragStart ? 'cursor-grabbing' : ''
		)}
		bind:this={canvas}
		tabindex="0"
		role="application"
		aria-label={studioMessage('preview.canvas')}
		style:touch-action={zoomPercent > 100 ? 'none' : 'auto'}
		onpointerdown={startPan}
		onpointermove={movePan}
		onpointerup={endPan}
		onpointercancel={endPan}
		onkeydown={panWithKeyboard}
		onwheel={zoomOnWheel}
	>
		<svg class="pointer-events-none absolute inset-0 size-full text-muted-foreground" aria-hidden="true">
			<defs>
				<pattern
					id={`${componentId}-grid`}
					width="22"
					height="22"
					patternUnits="userSpaceOnUse"
					patternTransform={`translate(${gridTransform.x} ${gridTransform.y}) scale(${gridTransform.scale})`}
				>
					<circle cx="1" cy="1" r="0.75" fill="currentColor" opacity="0.16" />
				</pattern>
			</defs>
			<rect width="100%" height="100%" fill={`url(#${componentId}-grid)`} />
		</svg>
		{#if safeSvg}
			<div
				class="absolute left-1/2 top-1/2 origin-center [&_svg.schemra-preview-svg]:block [&_svg.schemra-preview-svg]:size-full"
				bind:this={svgHost}
				style:width={displaySize.width}
				style:height={displaySize.height}
				style:transform={`translate(calc(-50% + ${pan.x}px), calc(-50% + ${pan.y}px)) scale(${zoomPercent / 100})`}
			>
				{@html safeSvg}
			</div>
		{:else}
			<p class="absolute inset-0 m-0 grid place-items-center p-4 text-center text-sm text-muted-foreground">
				{props.svg ? studioMessage('preview.unsafeDiagram') : studioMessage('preview.noVisualAvailable')}
			</p>
		{/if}
	</div>

	{#if scenario}
		<PlaybackTimeline
			durationMs={durationMs}
			{timeMs}
			{playing}
			ready={Boolean(playback)}
			onToggle={togglePlayback}
			onReset={resetPlayback}
			onSeek={seek}
			onTimelinePointerDown={() => (playing = false)}
			onTimelineKeydown={pauseOnTimelineInput}
		/>
	{/if}
	{#if playbackError}
		<p class={errorOverlayClasses} role="alert">
			{studioMessage('preview.playbackUnavailable', { message: playbackError })}
		</p>
	{/if}
	{#if exportError}
		<p class={errorOverlayClasses} role="alert">
			{studioMessage('preview.exportFailed', { message: exportError })}
		</p>
	{/if}
	{#if presentationError}
		<p class={errorOverlayClasses} role="alert">{presentationError}</p>
	{/if}
	{#if scenario}
		<AnimationExportDialog
			open={animationExportOpen}
			busy={animationExportBusy}
			progress={animationExportProgress}
			scenarioTitle={scenario.title}
			durationMs={durationMs}
			defaultBackground={getDiagramBackground(safeSvg, scene)}
			visualId={props.visualId}
			onOpenChange={(open) => (animationExportOpen = open)}
			onExport={renderAnimation}
		/>
	{/if}
</section>
