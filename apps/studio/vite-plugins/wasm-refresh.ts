import { spawn, type ChildProcess } from 'node:child_process';
import { resolve } from 'node:path';
import type { Plugin, ViteDevServer } from 'vite';

const wasmCrates = ['schemra-wasm', 'schemra-scene', 'schemra-engine', 'schemra-compiler', 'schemra-syntax', 'schemra-ir'];
const workspaceCrates = [...wasmCrates, 'schemra-cli'];
const watchedEvents = ['add', 'change', 'unlink'] as const;
const debounceMs = 350;
const maxCapturedOutput = 12_000;

function appendTail(current: string, chunk: Buffer): string {
	return `${current}${chunk.toString()}`.slice(-maxCapturedOutput);
}

function terminateProcessTree(child: ChildProcess): void {
	if (child.exitCode !== null || child.signalCode !== null) return;

	if (process.platform !== 'win32' && child.pid !== undefined) {
		try {
			process.kill(-child.pid, 'SIGTERM');
			return;
		} catch {
			// The process may have already exited; try the direct child below.
		}
	}

	child.kill('SIGTERM');
}

export function wasmRefreshPlugin(studioRoot: string, workspaceRoot: string): Plugin {
	return {
		name: 'schemra-wasm-refresh',
		apply: 'serve',
		configureServer(server: ViteDevServer) {
			const watchedPaths = [
				resolve(workspaceRoot, 'Cargo.toml'),
				resolve(workspaceRoot, 'Cargo.lock'),
				...workspaceCrates.map((crate) => resolve(workspaceRoot, `crates/${crate}/Cargo.toml`)),
				...wasmCrates.map((crate) => resolve(workspaceRoot, `crates/${crate}/src`))
			];

			let timer: ReturnType<typeof setTimeout> | undefined;
			let child: ChildProcess | undefined;
			let building = false;
			let rerunPending = false;
			let closing = false;

			const runBuild = () => {
				if (closing) return;
				if (building) {
					rerunPending = true;
					return;
				}

				building = true;
				const startedAt = Date.now();
				let stdout = '';
				let stderr = '';
				let spawnError: Error | undefined;
				server.config.logger.info('[schemra-wasm] rebuilding after Rust source change');

				child = spawn('bun', ['run', 'build:wasm'], {
					cwd: studioRoot,
					detached: process.platform !== 'win32',
					stdio: ['ignore', 'pipe', 'pipe'],
					windowsHide: true
				});
				const currentChild = child;
				currentChild.stdout?.on('data', (chunk: Buffer) => {
					stdout = appendTail(stdout, chunk);
				});
				currentChild.stderr?.on('data', (chunk: Buffer) => {
					stderr = appendTail(stderr, chunk);
				});
				currentChild.once('error', (error) => {
					spawnError = error;
				});
				currentChild.once('close', (code, signal) => {
					child = undefined;
					building = false;
					if (!closing) {
						if (code === 0 && !spawnError) {
							server.config.logger.info(
								`[schemra-wasm] build completed in ${((Date.now() - startedAt) / 1000).toFixed(1)}s; reloading browser`
							);
							server.ws.send({ type: 'full-reload' });
						} else {
							const details = [spawnError?.message, stderr.trim(), stdout.trim()]
								.filter(Boolean)
								.join('\n');
							server.config.logger.error(
								`[schemra-wasm] build failed (exit ${code ?? signal ?? 'unknown'}). Vite remains available.${details ? `\n${details}` : ''}`
							);
						}
					}

					if (rerunPending && !closing) {
						rerunPending = false;
						scheduleBuild();
					}
				});
			};

			const scheduleBuild = () => {
				if (closing) return;
				if (timer) clearTimeout(timer);
				timer = setTimeout(() => {
					timer = undefined;
					runBuild();
				}, debounceMs);
			};

			const onFileChange = (filePath: string) => {
				if (filePath.endsWith('.rs') || filePath.endsWith('Cargo.toml') || filePath.endsWith('Cargo.lock')) {
					scheduleBuild();
				}
			};

			const cleanup = () => {
				if (closing) return;
				closing = true;
				if (timer) clearTimeout(timer);
				for (const event of watchedEvents) server.watcher.off(event, onFileChange);
				server.watcher.off('close', cleanup);
				if (child) terminateProcessTree(child);
			};

			server.watcher.add(watchedPaths);
			for (const event of watchedEvents) server.watcher.on(event, onFileChange);
			server.watcher.once('close', cleanup);
			server.httpServer?.once('close', cleanup);
		}
	};
}
