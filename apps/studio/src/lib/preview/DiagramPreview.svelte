
<script lang="ts">
	import { onDestroy, tick } from 'svelte';
	import { createScenarioPlayback, type ProjectSource, type ScenarioPlayback } from '../schemra';
	import {
		createDiagramPngBlob,
		createDiagramSvgBlob,
		diagramExportFileName,
		downloadDiagramBlob
	} from './export';
	import AnimationExportDialog, { type AnimationExportSettings } from './AnimationExportDialog.svelte';
	import { cancelActiveAnimationExport, exportAnimation } from './animation-export.js';
	import { calculateNodeFocusZoom, calculateNodeScrollTarget, getPreviewConnectionBounds } from './viewport';
	import SceneNavigator from './SceneNavigator.svelte';
	import {
		previewTimeAtElapsed,
		routePathData,
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
	}

	type PreparedAnimationFrameRenderer = ((timeMs: number, background: string) => string) & {
		dispose: () => void;
	};

	const props: Props = $props();
	onDestroy(() => cancelActiveAnimationExport());
	const componentId = $props.id();
	const previewCanvasId = `${componentId}-canvas`;
	let canvas = $state<HTMLDivElement | undefined>();
	let svgHost = $state<HTMLDivElement | undefined>();
	let viewport = $state({ width: 0, height: 0 });
	let safeSvg = $state('');
	let playback = $state<ScenarioPlayback | null>(null);
	let playbackError = $state<string | null>(null);
	let frame = $state<PreviewFrame | null>(null);
	let playing = $state(false);
	let timeMs = $state(0);
	let zoomPercent = $state(100);
	let selectedNodeId = $state('');
	let exporting = $state<'svg' | 'png' | null>(null);
	let exportError = $state<string | null>(null);
	let animationExportOpen = $state(false);
	let animationExportBusy = $state(false);
	let animationExportProgress = $state(0);
	let playbackStartedAt = 0;
	let focusRequestId = 0;
	interface FrameRenderCache {
		progressPaths: Map<number, SVGPathElement>;
		markerGroups: Map<number, SVGGElement>;
		highlightGroups: Map<number, SVGGraphicsElement[]>;
	}
	const frameRenderCaches = new WeakMap<SVGSVGElement, FrameRenderCache>();
	const scene = $derived(readPreviewScene(props.scene));
	const scenario = $derived(readPreviewScenario(props.scenario));
	const durationMs = $derived(playback?.durationMs ?? scenario?.duration_ms ?? 0);
	const displaySize = $derived(svgDisplaySize(viewport, scene, zoomPercent));

	$effect(() => {
		const nodes = scene?.nodes ?? [];
		if (selectedNodeId && !nodes.some(({ id }) => id === selectedNodeId)) {
			selectedNodeId = '';
			focusRequestId += 1;
		}
	});

	$effect(() => {
		const markup = props.svg;
		try {
			safeSvg = markup ? sanitizeSvgMarkup(markup) : '';
		} catch {
			safeSvg = '';
		}
	});

	$effect(() => {
		const stage = canvas;
		if (!stage || typeof ResizeObserver === 'undefined') return;
		const observer = new ResizeObserver(([entry]) => {
			if (!entry) return;
			viewport = { width: entry.contentRect.width, height: entry.contentRect.height };
		});
		observer.observe(stage);
		return () => observer.disconnect();
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
				if (!cancelled) playbackError = error instanceof Error ? error.message : 'Playback could not be initialized.';
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
			playbackError = error instanceof Error ? error.message : 'Playback frame could not be sampled.';
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
		applyFrame(root, currentFrame, currentScene, scenario);
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

	function seek(event: Event) {
		playing = false;
		timeMs = Number((event.currentTarget as HTMLInputElement).value);
	}

	function setZoom(value: number) {
		zoomPercent = Math.max(50, Math.min(500, value));
	}

	function handleNodeSelection(event: Event) {
		const nodeId = (event.currentTarget as HTMLSelectElement).value;
		if (!nodeId) {
			fitDiagram();
			return;
		}
		selectedNodeId = nodeId;
		void focusNode(nodeId);
	}

	function handleTargetFocus(bounds: PreviewScene['nodes'][number]['bounds']) {
		selectedNodeId = '';
		void focusBounds(bounds);
	}

	function handleConnectionFocus(edgeIndex: number) {
		if (!scene) return;
		const bounds = getPreviewConnectionBounds(scene, edgeIndex);
		if (!bounds) return;
		selectedNodeId = '';
		void focusBounds(bounds);
	}

	function fitDiagram() {
		focusRequestId += 1;
		selectedNodeId = '';
		zoomPercent = 100;
		canvas?.scrollTo({ left: 0, top: 0, behavior: 'smooth' });
	}

	async function focusNode(nodeId: string) {
		const currentScene = scene;
		const node = currentScene?.nodes.find(({ id }) => id === nodeId);
		if (!node) return;
		selectedNodeId = nodeId;
		await focusBounds(node.bounds, nodeId);
	}

	async function focusBounds(bounds: PreviewScene['nodes'][number]['bounds'], nodeId = '') {
		const requestId = ++focusRequestId;
		const currentScene = scene;
		if (!canvas || !currentScene) return;

		const styles = getComputedStyle(canvas);
		const horizontalPadding = (Number.parseFloat(styles.paddingLeft) || 0) + (Number.parseFloat(styles.paddingRight) || 0);
		const verticalPadding = (Number.parseFloat(styles.paddingTop) || 0) + (Number.parseFloat(styles.paddingBottom) || 0);
		const focusZoom = calculateNodeFocusZoom(
			{ width: currentScene.width, height: currentScene.height },
			bounds,
			{
				width: Math.max(0, canvas.clientWidth - horizontalPadding),
				height: Math.max(0, canvas.clientHeight - verticalPadding)
			}
		);
		if (focusZoom !== null) zoomPercent = focusZoom;
		await tick();
		if (requestId !== focusRequestId || selectedNodeId !== nodeId || scene !== currentScene || !canvas || !svgHost) return;

		const latestScene = scene;
		if (!latestScene) return;
		const canvasRect = canvas.getBoundingClientRect();
		const sheetRect = svgHost.getBoundingClientRect();
		const target = calculateNodeScrollTarget(
			{ width: latestScene.width, height: latestScene.height },
			bounds,
			{ width: sheetRect.width, height: sheetRect.height },
			{
				x: sheetRect.left - canvasRect.left + canvas.scrollLeft,
				y: sheetRect.top - canvasRect.top + canvas.scrollTop
			},
			{ width: canvas.clientWidth, height: canvas.clientHeight },
			{ x: canvas.scrollWidth - canvas.clientWidth, y: canvas.scrollHeight - canvas.clientHeight }
		);
		if (target) canvas.scrollTo({ left: target.left, top: target.top, behavior: 'smooth' });
	}

	async function exportDiagram(format: 'svg' | 'png') {
		const root = svgHost?.querySelector('svg');
		if (!root) {
			exportError = 'The diagram is not ready to export.';
			return;
		}
		exporting = format;
		exportError = null;
		try {
			const name = scenario?.title || root.querySelector('title')?.textContent?.trim() || props.visualId || 'Schemra diagram';
			const fileName = diagramExportFileName(name, format);
			const blob = format === 'svg' ? createDiagramSvgBlob(root) : await createDiagramPngBlob(root);
			downloadDiagramBlob(blob, fileName);
		} catch (error) {
			exportError = error instanceof Error ? error.message : 'The diagram could not be exported.';
		} finally {
			exporting = null;
		}
	}

	function renderAnimation(settings: AnimationExportSettings): Promise<void> {
		const currentScene = scene;
		const currentScenario = scenario;
		const sampler = playback;
		if (!safeSvg || !currentScene || !currentScenario || !sampler || durationMs <= 0) {
			return Promise.reject(new Error('Select a scenario with a renderable duration first.'));
		}
		let renderFrame: ReturnType<typeof createAnimationFrameRenderer>;
		try {
			renderFrame = createAnimationFrameRenderer(safeSvg, currentScene, currentScenario, sampler);
		} catch (error) {
			return Promise.reject(error instanceof Error ? error : new Error('The current diagram could not be prepared for animation export.'));
		}
		animationExportBusy = true;
		animationExportProgress = 0;
		const job = exportAnimation({
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

	function createAnimationFrameRenderer(
		sourceSvg: string,
		currentScene: PreviewScene,
		currentScenario: NonNullable<ReturnType<typeof readPreviewScenario>>,
		sampler: ScenarioPlayback
	): PreparedAnimationFrameRenderer {
		const parsed = new DOMParser().parseFromString(sourceSvg, 'image/svg+xml');
		if (parsed.querySelector('parsererror')) throw new Error('The current diagram could not be prepared for animation export.');
		const root = parsed.documentElement as unknown as SVGSVGElement;
		const backgroundRect = [...root.children].find((element) =>
			element.localName === 'rect' &&
			Number(element.getAttribute('width')) === currentScene.width &&
			Number(element.getAttribute('height')) === currentScene.height
		) as SVGRectElement | undefined;
		if (!backgroundRect) throw new Error('The diagram background could not be prepared for export.');
		const colors = getComputedStyle(svgHost ?? document.documentElement);
		const primary = colors.getPropertyValue('--primary').trim() || '#005bd3';
		const primaryForeground = colors.getPropertyValue('--primary-foreground').trim() || '#ffffff';
		const serializer = new XMLSerializer();
		const renderHost = document.createElement('div');
		renderHost.setAttribute('aria-hidden', 'true');
		Object.assign(renderHost.style, {
			position: 'fixed',
			left: '0',
			top: '0',
			width: `${currentScene.width}px`,
			height: `${currentScene.height}px`,
			opacity: '0',
			pointerEvents: 'none',
			zIndex: '-1'
		});
		renderHost.append(root);
		document.body.append(renderHost);

		const renderFrame = ((timeMs: number, background: string) => {
			const time = previewTimeAtElapsed(timeMs, sampler.durationMs);
			applyFrame(root, readPreviewFrame(sampler.sample(time)), currentScene, currentScenario);
			backgroundRect.setAttribute('fill', background);
			return serializer.serializeToString(root)
				.replaceAll('var(--primary-foreground)', primaryForeground)
				.replaceAll('var(--primary)', primary)
				.replaceAll('var(--background)', background);
		}) as PreparedAnimationFrameRenderer;
		renderFrame.dispose = () => renderHost.remove();
		return renderFrame;
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

	function formatTime(value: number) {
		return `${(Math.max(0, value) / 1000).toFixed(2)}s`;
	}

	function svgDisplaySize(size: { width: number; height: number }, scene: PreviewScene | null, zoom: number) {
		if (!scene || size.width <= 0 || size.height <= 0) return { width: '100%', height: '100%' };
		const scale = Math.min(size.width / scene.width, size.height / scene.height) * (zoom / 100);
		return {
			width: `${Math.max(1, scene.width * scale)}px`,
			height: `${Math.max(1, scene.height * scale)}px`
		};
	}

	function applyFrame(root: SVGSVGElement, value: PreviewFrame | null, scene: PreviewScene | null, selectedScenario: ReturnType<typeof readPreviewScenario>) {
		const cache = frameRenderCacheFor(root);
		const { progressPaths, markerGroups, highlightGroups } = cache;
		const linkLayer = ensureLayer(root, 'schemra-preview-links');
		const markerLayer = ensureLayer(root, 'schemra-preview-markers');
		const highlightLayer = ensureLayer(root, 'schemra-preview-highlights', true);
		const activeEdges = new Set<number>();
		const activeTokens = new Set<number>();
		const activeHighlights = new Set<number>();

		for (const edge of value?.edges ?? []) {
			const path = scene?.edges[edge.edgeIndex]?.path;
			const data = path ? routePathData(path, edge.forward) : '';
			if (!data || edge.progress <= 0) continue;
			activeEdges.add(edge.instanceId);
			let progressPath = progressPaths.get(edge.instanceId);
			if (!progressPath) {
				progressPath = root.ownerDocument.createElementNS(SVG_NS, 'path');
				progressPath.setAttribute('fill', 'none');
				progressPath.setAttribute('stroke', 'var(--primary)');
				progressPath.setAttribute('stroke-width', '3');
				progressPath.setAttribute('stroke-linecap', 'round');
				progressPath.setAttribute('stroke-linejoin', 'round');
				progressPath.setAttribute('pathLength', '1');
				progressPath.setAttribute('aria-hidden', 'true');
				progressPaths.set(edge.instanceId, progressPath);
				linkLayer.append(progressPath);
			}
			progressPath.setAttribute('d', data);
			progressPath.setAttribute('stroke-dasharray', `${edge.progress} 1`);
		}

		for (const highlight of value?.highlights ?? []) {
			const node = [...root.querySelectorAll<SVGGElement>(':scope > g[data-node-id]')]
				.find((candidate) => candidate.getAttribute('data-node-id') === highlight.nodeId);
			const shape = node?.querySelector<SVGGraphicsElement>('[data-node-background="true"]');
			if (!shape) continue;
			activeHighlights.add(highlight.instanceId);
			const length = typeof (shape as SVGGeometryElement).getTotalLength === 'function'
				? (shape as SVGGeometryElement).getTotalLength()
				: 0;
			const originLength = highlight.origin && length > 0
				? nearestPathLength(shape as SVGGeometryElement, length, highlight.origin.x, highlight.origin.y)
				: 0;
			const traceLength = length * 0.025;
			let outlines = highlightGroups.get(highlight.instanceId);
			const outlineCount = highlight.effect === 'trace' ? 2 : 1;
			if (!outlines || outlines.length !== outlineCount) {
				for (const outline of outlines ?? []) outline.remove();
				outlines = Array.from({ length: outlineCount }, () => {
					const outline = shape.cloneNode(false) as SVGGraphicsElement;
					outline.removeAttribute('data-node-background');
					outline.removeAttribute('filter');
					outline.setAttribute('fill', 'none');
					outline.setAttribute('stroke', 'var(--primary)');
					outline.setAttribute('stroke-width', '2.5');
					outline.setAttribute('pointer-events', 'none');
					outline.setAttribute('aria-hidden', 'true');
					highlightLayer.append(outline);
					return outline;
				});
				highlightGroups.set(highlight.instanceId, outlines);
			}
			for (let index = 0; index < outlines.length; index += 1) {
				const outline = outlines[index]!;
				const direction = highlight.effect === 'trace' ? (index === 0 ? 1 : -1) : 0;
				if (highlight.effect === 'trace' && length > 0) {
					const distance = length * 0.5 * highlight.progress;
					outline.setAttribute('stroke-dasharray', `${traceLength} ${Math.max(0.001, length - traceLength)}`);
					outline.setAttribute('stroke-dashoffset', String(direction > 0
						? -(originLength + distance)
						: traceLength - originLength + distance));
				} else {
					outline.setAttribute('opacity', String(highlight.opacity));
				}
			}
		}

		const icons = new Map((selectedScenario?.tokens ?? []).map(({ token, icon }) => [token, icon ?? '']));
		for (const token of value?.tokens ?? []) {
			activeTokens.add(token.instanceId);
			let marker = markerGroups.get(token.instanceId);
			if (!marker) {
				marker = root.ownerDocument.createElementNS(SVG_NS, 'g');
				marker.setAttribute('aria-label', token.token);
				marker.setAttribute('data-token-instance', String(token.instanceId));
				const circle = root.ownerDocument.createElementNS(SVG_NS, 'circle');
				circle.setAttribute('r', '11');
				circle.setAttribute('fill', 'var(--primary)');
				circle.setAttribute('stroke', 'var(--background)');
				circle.setAttribute('stroke-width', '2');
				marker.append(circle);
				appendMarkerIcon(marker, icons.get(token.token) ?? '', token.token);
				markerGroups.set(token.instanceId, marker);
				marker.setAttribute('data-token-artwork', JSON.stringify([token.token, icons.get(token.token) ?? '']));
				markerLayer.append(marker);
			} else {
				const artwork = JSON.stringify([token.token, icons.get(token.token) ?? '']);
				if (marker.getAttribute('data-token-artwork') !== artwork) {
					const circle = marker.firstElementChild;
					if (circle) marker.replaceChildren(circle);
					appendMarkerIcon(marker, icons.get(token.token) ?? '', token.token);
					marker.setAttribute('data-token-artwork', artwork);
				}
			}
			marker.setAttribute('transform', `translate(${token.x} ${token.y})`);
		}
		removeInactive(progressPaths, activeEdges);
		removeInactive(markerGroups, activeTokens);
		removeInactive(highlightGroups, activeHighlights);
	}

	function frameRenderCacheFor(root: SVGSVGElement): FrameRenderCache {
		let cache = frameRenderCaches.get(root);
		if (!cache) {
			cache = { progressPaths: new Map(), markerGroups: new Map(), highlightGroups: new Map() };
			frameRenderCaches.set(root, cache);
		}
		return cache;
	}

	function removeInactive<T extends Element | Element[]>(elements: Map<number, T>, active: Set<number>) {
		for (const [id, element] of elements) {
			if (active.has(id)) continue;
			for (const child of Array.isArray(element) ? element : [element]) child.remove();
			elements.delete(id);
		}
	}

	function ensureLayer(root: SVGSVGElement, name: string, aboveNodes = false) {
		let layer = root.querySelector<SVGGElement>(`:scope > g[data-${name}]`);
		if (layer) return layer;
		layer = root.ownerDocument.createElementNS(SVG_NS, 'g');
		layer.setAttribute(`data-${name}`, 'true');
		layer.setAttribute('pointer-events', 'none');
		layer.setAttribute('aria-hidden', 'true');
		const firstNode = root.querySelector(':scope > g[data-node-id]');
		if (!aboveNodes && firstNode) root.insertBefore(layer, firstNode);
		else root.append(layer);
		return layer;
	}

	function appendMarkerIcon(marker: SVGGElement, icon: string, token: string) {
		const paths = TOKEN_ICONS[icon.toLowerCase().replace(/^lucide\//, '')];
		if (paths) {
			const iconGroup = marker.ownerDocument.createElementNS(SVG_NS, 'g');
			iconGroup.setAttribute('transform', 'scale(.68)');
			for (const data of paths) {
				const path = marker.ownerDocument.createElementNS(SVG_NS, 'path');
				path.setAttribute('d', data);
				path.setAttribute('fill', 'none');
				path.setAttribute('stroke', 'var(--primary-foreground)');
				path.setAttribute('stroke-width', '1.6');
				path.setAttribute('stroke-linecap', 'round');
				path.setAttribute('stroke-linejoin', 'round');
				iconGroup.append(path);
			}
			marker.append(iconGroup);
			return;
		}
		const text = marker.ownerDocument.createElementNS(SVG_NS, 'text');
		const label = Array.from(token.trim() || '?').slice(0, 2).join('').toUpperCase();
		text.textContent = label;
		text.setAttribute('y', '3');
		text.setAttribute('text-anchor', 'middle');
		text.setAttribute('font-size', label.length === 1 ? '9' : '6.5');
		text.setAttribute('font-weight', '700');
		text.setAttribute('fill', 'var(--primary-foreground)');
		marker.append(text);
	}

	function nearestPathLength(path: SVGGeometryElement, length: number, x: number, y: number) {
		let nearest = 0;
		let minDistance = Number.POSITIVE_INFINITY;
		for (let index = 0; index <= 128; index += 1) {
			const candidate = (length * index) / 128;
			const point = path.getPointAtLength(candidate);
			const distance = (point.x - x) ** 2 + (point.y - y) ** 2;
			if (distance < minDistance) {
				minDistance = distance;
				nearest = candidate;
			}
		}
		return nearest;
	}

	const SVG_NS = 'http://www.w3.org/2000/svg';
	const TOKEN_ICONS: Record<string, string[]> = {
		mail: ['M-8 -5H8V6H-8Z', 'M-8 -4L0 2L8 -4'],
		message: ['M-7 -7H7A3 3 0 0 1 10 -4V2A3 3 0 0 1 7 5H-2L-7 9V5A3 3 0 0 1 -10 2V-4A3 3 0 0 1 -7 -7Z'],
		user: ['M0 -8A3.5 3.5 0 1 0 0 -1A3.5 3.5 0 0 0 0 -8Z', 'M-8 8A8 5 0 0 1 8 8'],
		package: ['M-8 -5L0 -9L8 -5V5L0 9L-8 5Z', 'M0 -9V0M-8 -5L0 0L8 -5M0 0V9'],
		file: ['M-6 -9H2L7 -4V9H-6Z', 'M2 -9V-4H7M-3 1H4M-3 5H4'],
		server: ['M-8 -7H8V-1H-8ZM-8 2H8V8H-8Z'],
		database: ['M-7 -6C-7 -9 7 -9 7 -6V6C7 9 -7 9 -7 6Z', 'M-7 -6C-7 -3 7 -3 7 -6M-7 0C-7 3 7 3 7 0']
	};
</script>

<section class="preview" aria-label="Diagram preview">
	<header class="toolbar">
		<div class="title" title={scenario?.title ?? 'Static visual'}>{scenario?.title ?? 'Static visual'}</div>
		{#if scene && (scene.groups.length > 0 || scene.nodes.length > 0 || scene.edges.length > 0)}
			<SceneNavigator
				{scene}
				onFocusTarget={handleTargetFocus}
				onFocusNode={focusNode}
				onFocusConnection={handleConnectionFocus}
			/>
		{/if}
		{#if scene && scene.nodes.length > 0}
			<div class="node-focus">
				<label for={`${previewCanvasId}-node`}>Focus node</label>
				<select
					id={`${previewCanvasId}-node`}
					aria-controls={previewCanvasId}
					value={selectedNodeId}
					onchange={handleNodeSelection}
				>
					<option value="">Choose a node</option>
					{#each scene.nodes as node (node.id)}
						<option value={node.id}>{node.label ?? node.id}</option>
					{/each}
				</select>
			</div>
		{/if}
		<div class="zoom-controls" role="group" aria-label="Preview zoom controls">
			<button type="button" aria-label="Zoom out" disabled={zoomPercent <= 50} onclick={() => setZoom(zoomPercent - 10)}>−</button>
			<output aria-label={`Zoom ${zoomPercent} percent`}>{zoomPercent}%</output>
			<button type="button" aria-label="Zoom in" disabled={zoomPercent >= 500} onclick={() => setZoom(zoomPercent + 10)}>+</button>
			<button type="button" class="fit" aria-label="Fit diagram and reset node focus" onclick={fitDiagram}>Fit</button>
		</div>
		<div class="export-controls" role="group" aria-label="Export diagram">
			{#if scenario}
				<button
					type="button"
					aria-label="Export scenario animation as MP4"
					disabled={!safeSvg || !playback || durationMs <= 0 || animationExportBusy}
					onclick={() => { animationExportProgress = 0; animationExportOpen = true; }}
				>{animationExportBusy ? 'Rendering…' : 'MP4'}</button>
			{/if}
			<button
				type="button"
				aria-label="Download diagram as SVG"
				disabled={!safeSvg || exporting !== null}
				onclick={() => void exportDiagram('svg')}
			>{exporting === 'svg' ? 'Preparing…' : 'SVG'}</button>
			<button
				type="button"
				aria-label="Download diagram as PNG"
				disabled={!safeSvg || exporting !== null}
				onclick={() => void exportDiagram('png')}
			>{exporting === 'png' ? 'Preparing…' : 'PNG'}</button>
		</div>
	</header>

	<div id={previewCanvasId} class="canvas" bind:this={canvas} tabindex="-1" role="region" aria-label="Diagram canvas">
		{#if safeSvg}
			<div class="svg-sheet" bind:this={svgHost} style:width={displaySize.width} style:height={displaySize.height}>
				{@html safeSvg}
			</div>
		{:else}
			<p class="empty">{props.svg ? 'The diagram could not be displayed safely.' : 'No visual is available.'}</p>
		{/if}
	</div>

	{#if scenario}
		<div class="timeline">
			<button
				type="button"
				class="primary"
				aria-label={playing ? 'Pause scenario' : 'Play scenario'}
				disabled={!playback || durationMs <= 0}
				onclick={togglePlayback}
			>{playing ? 'Pause' : 'Play'}</button>
			<button type="button" aria-label="Restart scenario" disabled={!playback} onclick={resetPlayback}>Reset</button>
			<input
				type="range"
				min="0"
				max={Math.max(1, durationMs)}
				step="1"
				value={timeMs}
				aria-label="Scenario timeline"
				disabled={!playback}
				oninput={seek}
			/>
			<output class="time" aria-label={`Playback time ${formatTime(timeMs)} of ${formatTime(durationMs)}`}>
				{formatTime(timeMs)} / {formatTime(durationMs)}
			</output>
		</div>
	{/if}
	{#if playbackError}
		<p class="error" role="alert">Playback unavailable: {playbackError}</p>
	{/if}
	{#if exportError}
		<p class="error" role="alert">Export failed: {exportError}</p>
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

<style>
	.preview { display: flex; min-width: 0; min-height: 15rem; height: 100%; flex-direction: column; overflow: hidden; color: var(--foreground); background: var(--muted); }
	.toolbar { display: flex; min-height: 2.75rem; align-items: center; justify-content: space-between; gap: .75rem; padding: .5rem .75rem; border-bottom: 1px solid var(--border); background: var(--background); }
	.title { min-width: 0; flex: 1 1 auto; overflow: hidden; font-size: .875rem; font-weight: 600; text-overflow: ellipsis; white-space: nowrap; }
	.node-focus { display: flex; min-width: 0; flex: 0 1 auto; align-items: center; gap: .4rem; }
	.node-focus label { color: var(--muted-foreground); font-size: .75rem; white-space: nowrap; }
	.node-focus select { width: min(13rem, 24vw); min-width: 8rem; min-height: 2rem; padding: .25rem .5rem; border: 1px solid var(--border); border-radius: .375rem; color: var(--foreground); background: var(--background); font: inherit; font-size: .75rem; }
	.zoom-controls { display: flex; flex: 0 0 auto; align-items: center; gap: .25rem; }
	.export-controls { display: flex; flex: 0 0 auto; align-items: center; gap: .25rem; }
	button { min-height: 2rem; min-width: 2rem; padding: .25rem .5rem; border: 1px solid var(--border); border-radius: .375rem; color: var(--foreground); background: var(--background); font: inherit; font-size: .75rem; cursor: pointer; }
	button:hover:not(:disabled) { background: var(--accent); }
	button:disabled { cursor: not-allowed; opacity: .5; }
	button:focus-visible, input:focus-visible, select:focus-visible, .canvas:focus-visible { outline: 2px solid var(--ring); outline-offset: 2px; }
	.zoom-controls output { min-width: 3rem; text-align: center; font-family: ui-monospace, monospace; font-size: .75rem; color: var(--muted-foreground); }
	.fit { min-width: auto; margin-left: .25rem; }
	.canvas { display: flex; min-height: 0; flex: 1 1 auto; overflow: auto; padding: 1rem; background-image: radial-gradient(color-mix(in oklch, var(--muted-foreground) 16%, transparent) .75px, transparent .75px); background-size: 22px 22px; }
	.svg-sheet { flex: 0 0 auto; margin: auto; }
	.svg-sheet :global(svg.schemra-preview-svg) { display: block; width: 100%; height: 100%; }
	.empty { margin: auto; padding: 1rem; color: var(--muted-foreground); font-size: .875rem; text-align: center; }
	.timeline { display: flex; min-width: 0; align-items: center; gap: .375rem; padding: .5rem .75rem; border-top: 1px solid var(--border); background: var(--background); }
	.timeline .primary { color: var(--primary-foreground); background: var(--primary); border-color: var(--primary); }
	.timeline input { min-width: 2rem; flex: 1 1 auto; accent-color: var(--primary); }
	.time { flex: 0 0 auto; color: var(--muted-foreground); font-family: ui-monospace, monospace; font-size: .6875rem; white-space: nowrap; }
	.error { margin: 0; padding: .5rem .75rem; border-top: 1px solid var(--destructive); color: var(--destructive); background: var(--background); font-size: .75rem; overflow-wrap: anywhere; }
	@media (max-width: 640px) {
		.toolbar { align-items: flex-start; flex-wrap: wrap; }
		.title { flex-basis: 100%; }
		.node-focus { flex: 1 1 auto; }
		.node-focus select { width: 100%; min-width: 0; }
		.zoom-controls { margin-left: auto; }
		.export-controls { margin-left: auto; }
	}
	@media (max-width: 480px) { .timeline { flex-wrap: wrap; } .timeline input { order: 3; flex-basis: 100%; } }
</style>
