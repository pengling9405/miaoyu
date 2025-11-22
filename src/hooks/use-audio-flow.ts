import { useMutation } from "@tanstack/react-query";
import { commands } from "~/lib/tauri";

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
			await commands.startDictating();
		},
		onError: (error) => {
			console.error("Failed to start dictating:", error);
		},
	});
}
