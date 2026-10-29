export function installMemoryStorage() {
	const records = new Map<string, string>();
	const original = Object.getOwnPropertyDescriptor(globalThis, 'localStorage');
	Object.defineProperty(globalThis, 'localStorage', {
		configurable: true,
		value: {
			getItem: (key: string) => records.get(key) ?? null,
			setItem: (key: string, value: string) => records.set(key, String(value)),
			removeItem: (key: string) => records.delete(key),
			clear: () => records.clear()
		}
	});
	return {
		records,
		restore() {
			if (original) Object.defineProperty(globalThis, 'localStorage', original);
			else Reflect.deleteProperty(globalThis, 'localStorage');
		}
	};
}
