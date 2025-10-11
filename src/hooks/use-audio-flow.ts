import { useMutation } from "@tanstack/react-query";
import { commands } from "~/lib/tauri";

export function useStopDictating() {
	return useMutation({
		mutationFn: async () => {
			return await commands.stopDictating(true); // UI 触发
		},
		onError: (error) => {
			console.error("Failed to complete dictation:", error);
		},
	});
}

export function useCancelDictating() {
	return useMutation({
		mutationFn: async () => {
			return await commands.cancelDictating();
		},
		onError: (error) => {
			console.error("Failed to cancel dictating:", error);
		},
	});
}
export function useStartDictating() {
	return useMutation({
		mutationFn: async () => {
			await commands.startDictating("normal"); // Normal 模式
		},
		onError: (error) => {
			console.error("Failed to start dictating:", error);
		},
	});
}
