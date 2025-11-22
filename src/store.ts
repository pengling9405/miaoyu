import { useQuery, useQueryClient } from "@tanstack/react-query";
import { Store } from "@tauri-apps/plugin-store";
import { useEffect } from "react";

import type { HotkeysStore, SettingsStore } from "~/lib/tauri";

let _store: Promise<Store> | undefined;

const store = () => {
	if (!_store) {
		_store = Store.load("store");
	}

	return _store;
};

type StoreValue<T> = T | null | undefined;

function declareStore<T extends object>(name: string) {
	const queryKey = ["store", name];

	const get = async (): Promise<T | null> => {
		const s = await store();
		return (await s.get<T>(name)) ?? null;
	};

	const listen = async (fn: (value: StoreValue<T>) => void) => {
		const s = await store();
		const unlisten = await s.onKeyChange<T>(name, fn);
		return unlisten;
	};

	const set = async (value?: Partial<T>) => {
		const s = await store();

		if (value === undefined) {
			await s.delete(name);
		} else {
			const current = (await s.get<T>(name)) ?? ({} as T);
			await s.set(name, {
				...current,
				...value,
			});
		}

		await s.save();
	};

	const useStoreQuery = () => {
		const queryClient = useQueryClient();
		const query = useQuery({
			queryKey,
			queryFn: get,
		});

		useEffect(() => {
			let cleanup: (() => void) | undefined;

			listen((value) => {
				queryClient.setQueryData(queryKey, value ?? null);
			}).then((unlisten) => {
				cleanup = unlisten;
			});

			return () => {
				cleanup?.();
			};
		}, [queryClient]);

		const setAndUpdate = async (value?: Partial<T>) => {
			await set(value);

			if (value === undefined) {
				queryClient.setQueryData(queryKey, null);
			} else {
				const previous = queryClient.getQueryData<T | null>(queryKey);
				const next = {
					...(previous ?? ({} as T)),
					...value,
				};
				queryClient.setQueryData(queryKey, next);
			}
		};

		return {
			...query,
			set: setAndUpdate,
		};
	};

	return {
		get,
		listen,
		set,
		useQuery: useStoreQuery,
	};
}

export const hotkeysStore = declareStore<HotkeysStore>("hotkeys");

export type AppSettingsStore = SettingsStore & {
	cancelWithEscape?: boolean | null;
	appLanguage?: "zh-CN" | "en-US";
};

export const settingsStore = declareStore<AppSettingsStore>("settings");
